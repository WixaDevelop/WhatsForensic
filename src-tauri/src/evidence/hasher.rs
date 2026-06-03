//! Hashing SHA-256 en streaming, cancelable, con reporte de progreso.
//!
//! TODO fase 1: implementar `hash_file_streaming(path, cancel, progress)`
//! con `sha2::Sha256`, chunks de 1 MiB, chequeo periódico de `CancellationToken`.
