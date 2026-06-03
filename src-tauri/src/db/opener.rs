//! Apertura segura de SQLite. Punto único de entrada.
//!
//! TODO fase 2:
//! - `open_committed_only(path)` → URI `file:<path>?immutable=1` + flags
//!   `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI | SQLITE_OPEN_NO_MUTEX`.
//!   No asocia WAL (documentado por SQLite).
//! - `open_with_wal(path)` → flags `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`
//!   + `PRAGMA query_only = ON`. Puede modificar la copia working.
//! - Rechazar paths fuera del workspace del caso.
//! - `rehash_after_close(path)`: re-calcular SHA-256 después de cualquier
//!   apertura en modo `with_wal` para registrar cambios esperados en audit.log.
