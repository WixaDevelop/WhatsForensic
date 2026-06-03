//! Conversión de timestamps.
//!
//! TODO fase 3:
//! - Mac Absolute / Cocoa: segundos desde 2001-01-01 UTC. Epoch = 978307200.
//! - Unix segundos: valor típico `< 2e9`.
//! - Unix milisegundos: valor `> 1e12`.
//! - Heurística de detección con rangos y contexto de columna.
//!
//! Conservar siempre el valor crudo (`timestamp_raw`) y el formato detectado
//! (`timestamp_raw_format`) junto al UTC.
