//! Parser de WhatsApp iOS sobre `ChatStorage.sqlite`.
//!
//! Mapeos cargados de `resources/schemas/whatsapp_ios/unknown.toml`. Todos los
//! valores tienen `verified = false` hasta que un analista los confirme contra
//! evidencia real y agregue una entrada en `docs/SCHEMAS.md`.

use crate::db::introspect::SchemaSnapshot;
use crate::error::{AppError, AppErrorKind};
use crate::parsers::common_model::{Conversation, Message, MessageDirection, ParsedEvidence};
use crate::parsers::timestamps::{convert, TimestampFormat};
use crate::parsers::traits::{Confidence, Parser};
use rusqlite::{types::ValueRef, Connection};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::collections::HashMap;

const SEED_TOML: &str = include_str!("../../resources/schemas/whatsapp_ios/unknown.toml");

#[derive(Debug, Deserialize)]
struct Mapping {
    meta: Meta,
    tables: TablesSection,
    message_type: MessageTypeSection,
    direction: DirectionSection,
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
struct TablesSection {
    required: Vec<String>,
    #[allow(dead_code)]
    optional: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MessageTypeSection {
    verified: bool,
    labels: HashMap<String, String>,
    revocation_codes: Vec<i64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DirectionSection {
    verified: bool,
    labels: HashMap<String, String>,
}

pub struct WhatsAppIos {
    mapping: Mapping,
}

impl WhatsAppIos {
    pub fn new() -> Result<Self, AppError> {
        let mapping: Mapping = toml::from_str(SEED_TOML).map_err(|e| {
            AppError::new(
                AppErrorKind::Internal,
                "MAPPING_PARSE_FAILED",
                format!("No se pudo parsear el mapping seed: {e}"),
            )
        })?;
        Ok(Self { mapping })
    }
}

impl Parser for WhatsAppIos {
    fn key(&self) -> &'static str {
        "whatsapp_ios"
    }

    fn display_name(&self) -> &'static str {
        "WhatsApp iOS (ChatStorage.sqlite)"
    }

    fn detect(&self, filename: &str, schema: &SchemaSnapshot) -> Confidence {
        let filename_hint = filename.to_lowercase().contains("chatstorage");
        let has_message = schema.tables.contains_key("ZWAMESSAGE");
        let has_session = schema.tables.contains_key("ZWACHATSESSION");
        match (filename_hint, has_message, has_session) {
            (true, true, true) => Confidence::High,
            (_, true, true) => Confidence::High,
            (true, true, false) | (true, false, true) => Confidence::Medium,
            (_, true, false) | (_, false, true) => Confidence::Low,
            _ => Confidence::None,
        }
    }

    fn parse(&self, conn: &Connection) -> Result<ParsedEvidence, AppError> {
        let mut out = ParsedEvidence::new(
            "whatsapp_ios",
            &self.mapping.meta.version,
            self.mapping.meta.verified && self.mapping.message_type.verified,
        );

        // Validar tablas requeridas.
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

        // Sessions → Conversations.
        let mut conv_index: HashMap<i64, String> = HashMap::new();
        {
            let mut stmt = conn
                .prepare(
                    "SELECT Z_PK, COALESCE(ZPARTNERNAME, ''), COALESCE(ZSESSIONJID, ''), ZLASTMESSAGEDATE \
                     FROM ZWACHATSESSION ORDER BY Z_PK",
                )
                .map_err(map_err)?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, Option<f64>>(3)?,
                    ))
                })
                .map_err(map_err)?;
            for row in rows {
                let (pk, name, jid, last_ts) = row.map_err(map_err)?;
                let display = if !name.is_empty() { name } else { jid.clone() };
                let id = format!("conv:{pk}");
                conv_index.insert(pk, id.clone());
                let last_seen = last_ts.and_then(|v| convert(v, TimestampFormat::MacAbsolute));
                out.conversations.push(Conversation {
                    id,
                    display_name: Some(display),
                    source_table: "ZWACHATSESSION".to_string(),
                    source_pk: pk,
                    first_seen_utc: None,
                    last_seen_utc: last_seen,
                    message_count: 0,
                });
            }
        }

        // Messages.
        let mut stmt = conn
            .prepare(
                "SELECT \
                    Z_PK, ZCHATSESSION, ZMESSAGETYPE, ZISFROMME, \
                    ZFROMJID, ZTOJID, ZTEXT, ZMESSAGEDATE \
                 FROM ZWAMESSAGE ORDER BY Z_PK",
            )
            .map_err(map_err)?;
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([]).map_err(map_err)?;

        let mut counts_per_conv: HashMap<i64, u64> = HashMap::new();

        while let Some(row) = rows.next().map_err(map_err)? {
            let pk: i64 = row.get(0).map_err(map_err)?;
            let session_pk: Option<i64> = row.get(1).map_err(map_err)?;
            let msg_type_raw: Option<i64> = row.get(2).map_err(map_err)?;
            let is_from_me: Option<i64> = row.get(3).map_err(map_err)?;
            let from_jid: Option<String> = row.get(4).map_err(map_err)?;
            let to_jid: Option<String> = row.get(5).map_err(map_err)?;
            let body: Option<String> = row.get(6).map_err(map_err)?;
            let ts_raw: Option<f64> = row.get(7).map_err(map_err)?;

            let direction_label =
                is_from_me.and_then(|v| self.mapping.direction.labels.get(&v.to_string()).cloned());
            let direction = match direction_label.as_deref() {
                Some("outgoing") => MessageDirection::Outgoing,
                Some("incoming") => MessageDirection::Incoming,
                _ => MessageDirection::Unknown,
            };
            let sender = match direction {
                MessageDirection::Outgoing => to_jid.clone().or(from_jid.clone()),
                _ => from_jid.clone(),
            };

            let (interpreted, verified) = match msg_type_raw {
                Some(code) => match self.mapping.message_type.labels.get(&code.to_string()) {
                    Some(s) => (Some(s.clone()), self.mapping.message_type.verified),
                    None => (Some(format!("unknown({code})")), false),
                },
                None => (None, false),
            };
            let is_possibly_revoked = msg_type_raw
                .map(|c| self.mapping.message_type.revocation_codes.contains(&c))
                .unwrap_or(false);

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

            let conv_id = session_pk
                .and_then(|pk| conv_index.get(&pk).cloned())
                .unwrap_or_else(|| "conv:orphan".to_string());

            if let Some(pk) = session_pk {
                *counts_per_conv.entry(pk).or_insert(0) += 1;
            }

            out.messages.push(Message {
                id: format!("msg:{pk}"),
                conversation_id: conv_id,
                source_pk: pk,
                timestamp_utc: ts_utc,
                timestamp_raw: ts_raw,
                timestamp_raw_format: Some("mac_absolute".to_string()),
                sender,
                direction,
                body,
                media_ref: None,
                status_flags: None,
                message_type_raw: msg_type_raw,
                message_type_interpreted: interpreted,
                message_type_verified: verified,
                is_possibly_revoked,
                raw_row,
            });
        }

        // Aplicar message_count a cada conversación.
        for conv in &mut out.conversations {
            conv.message_count = *counts_per_conv.get(&conv.source_pk).unwrap_or(&0);
        }

        // Ordenamiento determinístico para reproducibilidad.
        out.messages.sort_by_key(|m| (m.timestamp_utc, m.source_pk));
        out.conversations.sort_by_key(|c| c.source_pk);

        Ok(out)
    }
}

fn map_err(e: rusqlite::Error) -> AppError {
    AppError::new(
        AppErrorKind::Io,
        "PARSER_QUERY_FAILED",
        format!("Consulta del parser WhatsApp iOS falló: {e}"),
    )
}
