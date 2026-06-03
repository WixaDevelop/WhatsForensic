//! Capa `analysis` — análisis derivados independientes de la fuente.
//!
//! Trabaja sobre `ParsedEvidence` del modelo común. Produce:
//! - Gaps en PKs autoincrementales y `sqlite_sequence`.
//! - Señales de eliminados (categorías A y B, estrictamente separadas).
//! - Estadísticas agregadas.
//!
//! No calcula scores de probabilidad de borrado. Reporta hechos y posibles causas.

pub mod deleted_hints;
pub mod gaps;
pub mod stats;
