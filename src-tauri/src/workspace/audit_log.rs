//! Audit log append-only, JSON Lines, encadenado por hash.
//!
//! TODO fase 1: cada entrada lleva `seq`, `ts_utc`, `ts_monotonic`, `actor`,
//! `action`, `params`, `result`, `prev_hash`, `line_hash`. Documentado como
//! trazabilidad técnica, **no** como firma criptográfica.
//!
//! IMPORTANTE: forzar LF (no CRLF) al escribir, para que el hash encadenado
//! sea bit-identical entre Windows y Linux.
