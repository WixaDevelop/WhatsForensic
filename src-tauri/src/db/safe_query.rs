//! Wrapper de queries con whitelist `SELECT`/`PRAGMA`.
//!
//! TODO fase 2: `run(conn, sql, params)` que valida (trim + uppercase + prefix
//! match) que el statement empiece por `SELECT` o `PRAGMA`. Cualquier otro
//! statement retorna error `permission/SQL_NOT_ALLOWED`. Cada query exitosa se
//! registra en audit.log con sql, evidence_id, modo de apertura y duración.
