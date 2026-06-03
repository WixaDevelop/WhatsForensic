//! Detección de gaps en PKs autoincrementales y `sqlite_sequence`.
//!
//! Reporta hechos (rangos faltantes, vecinos) y una nota de interpretación fija.
//! No calcula scores de probabilidad de borrado.

use crate::db::introspect::SchemaSnapshot;
use crate::error::{AppError, AppErrorKind};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

const INTERPRETATION_NOTE: &str = "Posibles causas: borrado por el usuario, transacción abortada, migración de la aplicación, reuso de IDs por la app, o errores transitorios. No constituye prueba de borrado.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gap {
    pub table: String,
    pub column: String,
    pub range_start: i64,
    pub range_end: i64,
    pub size: i64,
    pub prev_pk: Option<i64>,
    pub next_pk: Option<i64>,
    pub source: GapSource,
    pub interpretation_note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapSource {
    /// Falta entre dos PKs adyacentes.
    PkSequence,
    /// `sqlite_sequence.seq` mayor que el max PK observado.
    SqliteSequenceTail,
}

pub fn detect(conn: &Connection, schema: &SchemaSnapshot) -> Result<Vec<Gap>, AppError> {
    let mut out: Vec<Gap> = Vec::new();

    for (table_name, table) in &schema.tables {
        // Encontrar columna PK INTEGER.
        let Some(pk_col) = table.columns.iter().find(|c| c.pk) else {
            continue;
        };
        if !pk_col.col_type.to_uppercase().contains("INT") {
            continue;
        }

        let col_name = &pk_col.name;
        let sql = format!("SELECT \"{col_name}\" FROM \"{table_name}\" ORDER BY \"{col_name}\"");
        let mut stmt = conn.prepare(&sql).map_err(map_err)?;
        let rows = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .map_err(map_err)?;

        let mut prev: Option<i64> = None;
        for row in rows {
            let pk = row.map_err(map_err)?;
            if let Some(p) = prev {
                if pk > p + 1 {
                    let range_start = p + 1;
                    let range_end = pk - 1;
                    let size = range_end - range_start + 1;
                    out.push(Gap {
                        table: table_name.clone(),
                        column: col_name.clone(),
                        range_start,
                        range_end,
                        size,
                        prev_pk: Some(p),
                        next_pk: Some(pk),
                        source: GapSource::PkSequence,
                        interpretation_note: INTERPRETATION_NOTE.to_string(),
                    });
                }
            }
            prev = Some(pk);
        }
    }

    // sqlite_sequence vs MAX(pk) por tabla.
    if let Ok(mut stmt) = conn.prepare("SELECT name, seq FROM sqlite_sequence") {
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
            .map_err(map_err)?;
        for row in rows {
            let (name, seq) = row.map_err(map_err)?;
            if let Some(table) = schema.tables.get(&name) {
                if let Some(pk_col) = table.columns.iter().find(|c| c.pk) {
                    let col = &pk_col.name;
                    let max_pk: Option<i64> = conn
                        .query_row(&format!("SELECT MAX(\"{col}\") FROM \"{name}\""), [], |r| {
                            r.get(0)
                        })
                        .ok();
                    let max_pk = max_pk.unwrap_or(0);
                    if seq > max_pk {
                        let range_start = max_pk + 1;
                        let range_end = seq;
                        let size = range_end - range_start + 1;
                        out.push(Gap {
                            table: name.clone(),
                            column: col.clone(),
                            range_start,
                            range_end,
                            size,
                            prev_pk: Some(max_pk),
                            next_pk: None,
                            source: GapSource::SqliteSequenceTail,
                            interpretation_note: INTERPRETATION_NOTE.to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(out)
}

fn map_err(e: rusqlite::Error) -> AppError {
    AppError::new(
        AppErrorKind::Io,
        "GAP_DETECT_FAILED",
        format!("Detección de gaps falló: {e}"),
    )
}
