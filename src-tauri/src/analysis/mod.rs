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

use crate::db::introspect::SchemaSnapshot;
use crate::error::AppError;
use crate::parsers::common_model::ParsedEvidence;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisFindings {
    pub gaps: Vec<gaps::Gap>,
    pub deleted_hints: Vec<deleted_hints::DeletedHint>,
    pub stats: stats::Stats,
}

pub fn compute(
    conn: &Connection,
    schema: &SchemaSnapshot,
    parsed: &ParsedEvidence,
) -> Result<AnalysisFindings, AppError> {
    let gaps = gaps::detect(conn, schema)?;
    let deleted_hints = deleted_hints::detect(parsed);
    let stats = stats::compute(parsed);
    Ok(AnalysisFindings {
        gaps,
        deleted_hints,
        stats,
    })
}
