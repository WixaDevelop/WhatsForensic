//! Validación del header SQLite y lectura de page_size.
//!
//! Lee los primeros 100 bytes del archivo (header de SQLite v3) y verifica el
//! magic. Nunca abre el archivo con SQLite — solo bytes.

use crate::error::{AppError, AppErrorKind};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::AsyncReadExt;

const SQLITE_MAGIC: &[u8; 16] = b"SQLite format 3\0";
const HEADER_LEN: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteHeader {
    pub valid_magic: bool,
    pub page_size: u32,
    pub file_size: u64,
}

/// Lee y valida el header SQLite de un archivo. No abre con SQLite.
pub async fn validate(path: &Path) -> Result<SqliteHeader, AppError> {
    let mut file = tokio::fs::File::open(path).await.map_err(|e| {
        AppError::new(
            AppErrorKind::Io,
            "OPEN_FAILED",
            format!("No se pudo abrir el archivo: {e}"),
        )
        .with_context("path", serde_json::json!(path.display().to_string()))
    })?;

    let file_size = file
        .metadata()
        .await
        .map(|m| m.len())
        .map_err(|e| AppError::new(AppErrorKind::Io, "METADATA_FAILED", format!("{e}")))?;

    let mut buf = [0u8; HEADER_LEN];
    let n = file.read(&mut buf).await.map_err(|e| {
        AppError::new(
            AppErrorKind::Io,
            "READ_FAILED",
            format!("Error leyendo el header: {e}"),
        )
    })?;

    if n < HEADER_LEN {
        return Ok(SqliteHeader {
            valid_magic: false,
            page_size: 0,
            file_size,
        });
    }

    let valid_magic = &buf[0..16] == SQLITE_MAGIC;
    let raw_ps = u16::from_be_bytes([buf[16], buf[17]]);
    // SQLite documenta que page_size = 1 significa 65536.
    let page_size = if raw_ps == 1 { 65536u32 } else { raw_ps as u32 };

    Ok(SqliteHeader {
        valid_magic,
        page_size,
        file_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn rejects_non_sqlite() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"This is not a SQLite database file content here............................................................")
            .unwrap();
        let h = validate(tmp.path()).await.unwrap();
        assert!(!h.valid_magic);
    }

    #[tokio::test]
    async fn rejects_too_short() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(b"SQLite format 3\0").unwrap();
        let h = validate(tmp.path()).await.unwrap();
        assert!(!h.valid_magic);
        assert_eq!(h.page_size, 0);
    }
}
