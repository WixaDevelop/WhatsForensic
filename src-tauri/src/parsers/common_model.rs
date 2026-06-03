//! Modelo común al que cada parser traduce su fuente.
//!
//! Reglas (CLAUDE.md root, regla 6):
//! - `raw_row` se conserva siempre.
//! - Junto al valor interpretado va el valor crudo (`*_raw`).
//! - Si un tipo desconocido no está mapeado, se etiqueta como
//!   `Unknown(<raw>)` en lugar de descartarlo.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageDirection {
    Incoming,
    Outgoing,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub display_name: Option<String>,
    pub source_table: String,
    pub source_pk: i64,
    pub first_seen_utc: Option<DateTime<Utc>>,
    pub last_seen_utc: Option<DateTime<Utc>>,
    pub message_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub source_pk: i64,
    pub timestamp_utc: Option<DateTime<Utc>>,
    pub timestamp_raw: Option<f64>,
    pub timestamp_raw_format: Option<String>,
    pub sender: Option<String>,
    pub direction: MessageDirection,
    pub body: Option<String>,
    pub media_ref: Option<String>,
    pub status_flags: Option<String>,
    pub message_type_raw: Option<i64>,
    pub message_type_interpreted: Option<String>,
    pub message_type_verified: bool,
    pub is_possibly_revoked: bool,
    pub raw_row: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Call {
    pub id: String,
    pub source_pk: i64,
    pub timestamp_utc: Option<DateTime<Utc>>,
    pub timestamp_raw: Option<f64>,
    pub timestamp_raw_format: Option<String>,
    pub peer: Option<String>,
    pub direction: MessageDirection,
    pub duration_seconds: Option<i64>,
    pub call_type_raw: Option<i64>,
    pub call_type_interpreted: Option<String>,
    pub call_type_verified: bool,
    pub raw_row: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParserWarning {
    pub category: String,
    pub message: String,
    pub context: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedEvidence {
    pub source_kind: String,
    pub schema_version_used: String,
    pub schema_verified: bool,
    pub conversations: Vec<Conversation>,
    pub messages: Vec<Message>,
    pub calls: Vec<Call>,
    pub warnings: Vec<ParserWarning>,
}

impl ParsedEvidence {
    pub fn new(source_kind: &str, schema_version: &str, verified: bool) -> Self {
        Self {
            source_kind: source_kind.to_string(),
            schema_version_used: schema_version.to_string(),
            schema_verified: verified,
            conversations: Vec::new(),
            messages: Vec::new(),
            calls: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn warn(&mut self, category: &str, message: &str, context: serde_json::Value) {
        self.warnings.push(ParserWarning {
            category: category.to_string(),
            message: message.to_string(),
            context: context
                .as_object()
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
        });
    }
}
