//! Capa `db` — apertura segura de SQLite, solo sobre copias, read-only forzado.
//!
//! Reglas (críticas):
//! - `Connection::open` directo está **prohibido fuera de este módulo**.
//!   Toda apertura pasa por `opener::open_committed_only` o
//!   `opener::open_with_wal`.
//! - Toda query pasa por [`safe_query::run`] que valida que el statement
//!   empiece por `SELECT` o `PRAGMA`.
//! - El path solo puede apuntar dentro del workspace de caso. El opener rechaza
//!   paths fuera.

pub mod introspect;
pub mod opener;
pub mod safe_query;
