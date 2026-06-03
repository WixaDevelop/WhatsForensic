//! Detección de gaps en PKs autoincrementales y `sqlite_sequence`.
//!
//! TODO fase 5: cada gap reporta tabla, rango faltante, tamaño, PKs/timestamps
//! vecinos como contexto. La columna `interpretation_note` lleva texto fijo:
//! *"Posibles causas: borrado, transacción abortada, migración, reuso de IDs.
//! No constituye prueba de borrado."* No calcular scores de probabilidad.
