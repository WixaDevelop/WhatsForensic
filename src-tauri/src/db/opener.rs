//! Apertura segura de SQLite. **Punto único de entrada.**
//!
//! Reglas:
//! - Antes de abrir con SQLite, valida el header con `evidence::header` (bytes-only).
//! - Modo `CommittedOnly`: URI `file:<path>?immutable=1` + flags read-only/URI/no-mutex.
//!   SQLite documenta que `immutable=1` deshabilita el locking, ignora cambios
//!   y **no asocia el WAL**. Útil para snapshots reproducibles.
//! - Modo `WithWal`: flags read-only/no-mutex sin `immutable`. SQLite puede
//!   asociar el WAL y eventualmente hacer checkpoint, modificando la copia
//!   working. Por eso se trabaja sobre `working`, nunca sobre `pristine`.
//! - `PRAGMA query_only = ON` se aplica en ambos modos como cinturón extra.

use crate::error::{AppError, AppErrorKind};
use crate::evidence::header;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenMode {
    /// Apertura con `immutable=1`. **No** asocia WAL. Snapshot solo committed.
    CommittedOnly,
    /// Apertura sin `immutable`. Puede asociar y modificar el WAL.
    WithWal,
}

impl OpenMode {
    pub fn as_str(self) -> &'static str {
        match self {
            OpenMode::CommittedOnly => "committed_only",
            OpenMode::WithWal => "with_wal",
        }
    }
}

/// Abre una DB con el modo solicitado, previa validación de header.
pub async fn open(path: &Path, mode: OpenMode) -> Result<Connection, AppError> {
    let header = header::validate(path).await?;
    if !header.valid_magic {
        return Err(AppError::new(
            AppErrorKind::InvalidInput,
            "INVALID_SQLITE_HEADER",
            "El archivo no tiene un header SQLite válido.",
        )
        .with_context("path", serde_json::json!(path.display().to_string())));
    }
    let conn = match mode {
        OpenMode::CommittedOnly => open_committed_only(path)?,
        OpenMode::WithWal => open_with_wal(path)?,
    };
    conn.execute_batch("PRAGMA query_only = ON;").map_err(|e| {
        AppError::new(
            AppErrorKind::Internal,
            "PRAGMA_FAILED",
            format!("PRAGMA query_only falló: {e}"),
        )
    })?;
    Ok(conn)
}

fn open_committed_only(path: &Path) -> Result<Connection, AppError> {
    let uri = path_to_immutable_uri(path)?;
    tracing::debug!(uri = %uri, "opening SQLite in committed_only mode");
    Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| {
        AppError::new(
            AppErrorKind::Io,
            "DB_OPEN_FAILED",
            format!("No se pudo abrir la DB en modo solo committed: {e}"),
        )
        .with_context("uri", serde_json::json!(uri))
    })
}

fn open_with_wal(path: &Path) -> Result<Connection, AppError> {
    tracing::debug!(path = %path.display(), "opening SQLite in with_wal mode");
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| {
        AppError::new(
            AppErrorKind::Io,
            "DB_OPEN_FAILED",
            format!("No se pudo abrir la DB en modo incluye WAL: {e}"),
        )
    })
}

/// Construye una URI `file:...?immutable=1` con percent-encoding seguro.
///
/// En Linux: `file:/path`. En Windows: `file:///c:/path` (post-canonicalize).
fn path_to_immutable_uri(path: &Path) -> Result<String, AppError> {
    let abs = dunce::canonicalize(path)
        .map_err(|e| AppError::new(AppErrorKind::Io, "CANONICALIZE_FAILED", format!("{e}")))?;
    let path_str = abs.to_string_lossy().replace('\\', "/");
    // Percent-encode no-ASCII bytes y caracteres reservados de URI.
    let encoded: String = path_str
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    let prefix = if cfg!(windows) { "file:///" } else { "file:" };
    Ok(format!("{prefix}{encoded}?immutable=1"))
}

/// Re-calcula SHA-256 del archivo. Se usa después de cerrar una conexión
/// abierta con `WithWal` para registrar cambios esperados (checkpoint) en el
/// audit log.
pub async fn rehash(path: &Path) -> Result<String, AppError> {
    crate::evidence::hasher::hash_file_streaming(
        path,
        tokio_util::sync::CancellationToken::new(),
        |_| {},
    )
    .await
}
