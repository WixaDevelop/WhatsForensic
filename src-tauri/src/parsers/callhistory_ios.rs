//! Parser de CallHistory iOS sobre `CallHistory.storedata` / `CallHistory.sqlite`.
//!
//! Core Data: tablas `ZCALLRECORD`, `ZRECENT`, `Z_PRIMARYKEY`. Timestamps en
//! Mac Absolute. Mapeos en `resources/schemas/callhistory_ios/unknown.toml`.

use crate::db::introspect::SchemaSnapshot;
use crate::error::{AppError, AppErrorKind};
use crate::parsers::common_model::{Call, MessageDirection, ParsedEvidence};
use crate::parsers::timestamps::{convert, TimestampFormat};
use crate::parsers::traits::{Confidence, Parser};
use rusqlite::{types::ValueRef, Connection};
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};

const SEED_TOML: &str = include_str!("../../resources/schemas/callhistory_ios/unknown.toml");

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Mapping {
    meta: Meta,
    tables: TablesSection,
    direction: DirectionSection,
    call_type: CallTypeSection,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Meta {
    source: String,
    version: String,
    verified: bool,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TablesSection {
    required: Vec<String>,
    optional: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DirectionSection {
    verified: bool,
    labels: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct CallTypeSection {
    verified: bool,
    labels: HashMap<String, String>,
}

pub struct CallHistoryIos {
    mapping: Mapping,
}

impl CallHistoryIos {
    pub fn new() -> Result<Self, AppError> {
        let mapping: Mapping = toml::from_str(SEED_TOML).map_err(|e| {
            AppError::new(
                AppErrorKind::Internal,
                "MAPPING_PARSE_FAILED",
                format!("No se pudo parsear el mapping seed CallHistory: {e}"),
            )
        })?;
        Ok(Self { mapping })
    }
}

impl Parser for CallHistoryIos {
    fn key(&self) -> &'static str {
        "callhistory_ios"
    }

    fn display_name(&self) -> &'static str {
        "CallHistory iOS (Core Data)"
    }

    fn detect(&self, filename: &str, schema: &SchemaSnapshot) -> Confidence {
        let filename_hint = filename.to_lowercase().contains("callhistory");
        let has_record = schema.tables.contains_key("ZCALLRECORD");
        match (filename_hint, has_record) {
            (true, true) => Confidence::High,
            (_, true) => Confidence::Medium,
            (true, false) => Confidence::Low,
            _ => Confidence::None,
        }
    }

    fn parse(&self, conn: &Connection) -> Result<ParsedEvidence, AppError> {
        let mut out = ParsedEvidence::new(
            "callhistory_ios",
            &self.mapping.meta.version,
            self.mapping.meta.verified
                && self.mapping.call_type.verified
                && self.mapping.direction.verified,
        );

        for req in &self.mapping.tables.required {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
                    [req],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if count == 0 {
                out.warn(
                    "schema_mismatch",
                    &format!("Tabla esperada faltante: {req}"),
                    json!({"table": req}),
                );
            }
        }

        // Detectar columnas reales de ZCALLRECORD via PRAGMA table_info (los nombres
        // pueden variar; usamos COALESCE para campos opcionales).
        let table_info_cols: Vec<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(\"ZCALLRECORD\")")
                .map_err(map_err)?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(1))
                .map_err(map_err)?;
            rows.collect::<Result<Vec<String>, _>>().unwrap_or_default()
        };
        if table_info_cols.is_empty() {
            return Ok(out);
        }
        let has = |c: &str| table_info_cols.iter().any(|x| x.eq_ignore_ascii_case(c));

        // Construir consulta robusta.
        let col_date = if has("ZDATE") { "ZDATE" } else { "NULL" };
        let col_dur = if has("ZDURATION") {
            "ZDURATION"
        } else {
            "NULL"
        };
        let col_dir = if has("ZORIGINATED") {
            "ZORIGINATED"
        } else {
            "NULL"
        };
        let col_type = if has("ZCALLTYPE") {
            "ZCALLTYPE"
        } else if has("ZSERVICE_PROVIDER") {
            "ZSERVICE_PROVIDER"
        } else {
            "NULL"
        };
        let col_addr = if has("ZADDRESS") { "ZADDRESS" } else { "NULL" };

        let sql = format!(
            "SELECT Z_PK, {col_date}, {col_dur}, {col_dir}, {col_type}, {col_addr} FROM ZCALLRECORD ORDER BY Z_PK"
        );
        let mut stmt = conn.prepare(&sql).map_err(map_err)?;
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([]).map_err(map_err)?;

        while let Some(row) = rows.next().map_err(map_err)? {
            let pk: i64 = row.get(0).map_err(map_err)?;
            let ts_raw: Option<f64> = row.get(1).map_err(map_err)?;
            let dur: Option<i64> = row.get(2).map_err(map_err)?;
            let dir_raw: Option<i64> = row.get(3).map_err(map_err)?;
            let type_raw: Option<i64> = row.get(4).map_err(map_err)?;
            let addr_ref = row.get_ref(5).unwrap_or(ValueRef::Null);
            let peer = match addr_ref {
                ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
                ValueRef::Blob(b) => Some(format!("<blob:{} bytes>", b.len())),
                _ => None,
            };

            let direction_label =
                dir_raw.and_then(|v| self.mapping.direction.labels.get(&v.to_string()).cloned());
            let direction = match direction_label.as_deref() {
                Some("outgoing") => MessageDirection::Outgoing,
                Some("incoming") => MessageDirection::Incoming,
                _ => MessageDirection::Unknown,
            };

            let (type_interp, type_verified) = match type_raw {
                Some(c) => match self.mapping.call_type.labels.get(&c.to_string()) {
                    Some(s) => (Some(s.clone()), self.mapping.call_type.verified),
                    None => (Some(format!("unknown({c})")), false),
                },
                None => (None, false),
            };

            let ts_utc = ts_raw.and_then(|v| convert(v, TimestampFormat::MacAbsolute));

            let mut raw_row: BTreeMap<String, serde_json::Value> = BTreeMap::new();
            for (i, col) in column_names.iter().enumerate() {
                let value: serde_json::Value = match row.get_ref(i).unwrap_or(ValueRef::Null) {
                    ValueRef::Null => serde_json::Value::Null,
                    ValueRef::Integer(v) => json!(v),
                    ValueRef::Real(v) => json!(v),
                    ValueRef::Text(t) => {
                        let s = String::from_utf8_lossy(t).into_owned();
                        json!(s)
                    }
                    ValueRef::Blob(b) => json!(format!("<blob:{} bytes>", b.len())),
                };
                raw_row.insert(col.clone(), value);
            }

            out.calls.push(Call {
                id: format!("call:{pk}"),
                source_pk: pk,
                timestamp_utc: ts_utc,
                timestamp_raw: ts_raw,
                timestamp_raw_format: Some("mac_absolute".to_string()),
                peer,
                direction,
                duration_seconds: dur,
                call_type_raw: type_raw,
                call_type_interpreted: type_interp,
                call_type_verified: type_verified,
                raw_row,
            });
        }

        out.calls.sort_by_key(|c| (c.timestamp_utc, c.source_pk));
        Ok(out)
    }
}

fn map_err(e: rusqlite::Error) -> AppError {
    AppError::new(
        AppErrorKind::Io,
        "PARSER_QUERY_FAILED",
        format!("Consulta del parser CallHistory iOS falló: {e}"),
    )
}
