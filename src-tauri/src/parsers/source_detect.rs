//! Detección heurística de fuente.

use crate::db::introspect::SchemaSnapshot;
use crate::parsers::traits::{Confidence, Parser};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParserMatch {
    pub key: String,
    pub display_name: String,
    pub confidence: Confidence,
}

/// Sugiere parsers ordenados por confianza descendente.
pub fn suggest(
    filename: &str,
    schema: &SchemaSnapshot,
    parsers: &[&dyn Parser],
) -> Vec<ParserMatch> {
    let mut matches: Vec<ParserMatch> = parsers
        .iter()
        .map(|p| ParserMatch {
            key: p.key().to_string(),
            display_name: p.display_name().to_string(),
            confidence: p.detect(filename, schema),
        })
        .filter(|m| !matches!(m.confidence, Confidence::None))
        .collect();
    matches.sort_by_key(|m| std::cmp::Reverse(confidence_score(m.confidence)));
    matches
}

fn confidence_score(c: Confidence) -> u8 {
    match c {
        Confidence::None => 0,
        Confidence::Low => 1,
        Confidence::Medium => 2,
        Confidence::High => 3,
    }
}
