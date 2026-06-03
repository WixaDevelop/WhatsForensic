//! Capa `evidence` — registro, hashing y copia de archivos fuente.
//!
//! No conoce SQLite. Lee archivos como bytes. Todo el manejo del archivo
//! original (lectura, hash, copia) vive acá. Las copias `pristine` y `working`
//! se materializan en este módulo; el ciclo de vida y manifest los maneja
//! [`crate::workspace`].
//!
//! Reglas forenses no negociables aplicables a esta capa: 1, 2, 3, 4
//! (ver `CLAUDE.md` root).

pub mod hasher;
pub mod header;
pub mod ingest;
pub mod metadata;
