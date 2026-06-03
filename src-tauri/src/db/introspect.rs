//! Introspección de schema: tablas, columnas, tipos, índices.
//!
//! TODO fase 2: consulta `sqlite_master`, `PRAGMA table_info(...)`,
//! `PRAGMA index_list(...)`. Produce un `SchemaSnapshot` serializable que
//! viaja al frontend para sugerir parser.
