//! Introspección de schema: tablas, columnas, índices, metadata.
//!
//! Produce un `SchemaSnapshot` serializable que viaja al frontend para que el
//! analista pueda inspeccionar el esquema antes de elegir un parser.

use crate::error::{AppError, AppErrorKind};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaSnapshot {
    pub sqlite_version: String,
    pub user_version: i64,
    pub application_id: i64,
    pub page_count: i64,
    pub page_size: i64,
    pub journal_mode: String,
    pub tables: BTreeMap<String, TableInfo>,
    pub indexes: Vec<IndexInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    pub name: String,
    pub kind: String,
    pub sql: String,
    pub columns: Vec<ColumnInfo>,
    pub row_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    pub cid: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
    pub notnull: bool,
    pub pk: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub name: String,
    pub table: String,
    pub unique: bool,
    pub sql: Option<String>,
}

/// Toma un snapshot completo del schema. La conexión debe estar abierta en
/// modo read-only por `db::opener`.
pub fn snapshot(conn: &Connection) -> Result<SchemaSnapshot, AppError> {
    let sqlite_version: String = conn
        .query_row("SELECT sqlite_version()", [], |r| r.get(0))
        .map_err(map_err)?;
    let user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap_or(0);
    let application_id: i64 = conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .unwrap_or(0);
    let page_count: i64 = conn
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .unwrap_or(0);
    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |r| r.get(0))
        .unwrap_or(0);
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap_or_else(|_| "unknown".to_string());

    let mut tables: BTreeMap<String, TableInfo> = BTreeMap::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT name, type, COALESCE(sql, '') FROM sqlite_master \
                 WHERE type IN ('table', 'view') \
                   AND name NOT LIKE 'sqlite_%' \
                 ORDER BY name",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(map_err)?;
        for row in rows {
            let (name, kind, sql) = row.map_err(map_err)?;
            let columns = list_columns(conn, &name)?;
            let row_count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM \"{name}\""), [], |r| {
                    r.get(0)
                })
                .unwrap_or(-1);
            tables.insert(
                name.clone(),
                TableInfo {
                    name,
                    kind,
                    sql,
                    columns,
                    row_count,
                },
            );
        }
    }

    let mut indexes: Vec<IndexInfo> = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT name, tbl_name, sql FROM sqlite_master \
                 WHERE type = 'index' AND name NOT LIKE 'sqlite_%' \
                 ORDER BY name",
            )
            .map_err(map_err)?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(map_err)?;
        for row in rows {
            let (name, table, sql) = row.map_err(map_err)?;
            let is_unique = sql
                .as_ref()
                .is_some_and(|s| s.to_uppercase().contains("UNIQUE"));
            indexes.push(IndexInfo {
                name,
                table,
                unique: is_unique,
                sql,
            });
        }
    }

    Ok(SchemaSnapshot {
        sqlite_version,
        user_version,
        application_id,
        page_count,
        page_size,
        journal_mode,
        tables,
        indexes,
    })
}

fn list_columns(conn: &Connection, table: &str) -> Result<Vec<ColumnInfo>, AppError> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info(\"{table}\")"))
        .map_err(map_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ColumnInfo {
                cid: r.get(0)?,
                name: r.get(1)?,
                col_type: r.get(2)?,
                notnull: r.get::<_, i64>(3)? != 0,
                default_value: r.get(4)?,
                pk: r.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(map_err)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(map_err)?);
    }
    Ok(out)
}

fn map_err(e: rusqlite::Error) -> AppError {
    AppError::new(
        AppErrorKind::Io,
        "INTROSPECT_FAILED",
        format!("Introspección del schema falló: {e}"),
    )
}
