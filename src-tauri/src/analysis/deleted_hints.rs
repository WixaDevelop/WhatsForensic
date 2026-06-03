//! Señales de eliminación / revocación, en **dos categorías estrictamente separadas**.
//!
//! - **A** (`revoked_declared`): el parser detectó un código que la app marca
//!   como revocado. Esta categoría no garantiza recuperación del contenido.
//! - **B** (`structural_anomaly`): patrón observado que podría ser compatible
//!   con eliminación, pero también con otras causas. Lleva `evidence_strength`.

use crate::parsers::common_model::{Message, MessageDirection, ParsedEvidence};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HintCategory {
    A,
    B,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStrength {
    Weak,
    Moderate,
    Strong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedHint {
    pub category: HintCategory,
    pub message_id: String,
    pub kind: String,
    pub evidence_strength: EvidenceStrength,
    pub note: String,
}

pub fn detect(parsed: &ParsedEvidence) -> Vec<DeletedHint> {
    let mut out: Vec<DeletedHint> = Vec::new();
    let known_conv_ids: BTreeSet<&str> =
        parsed.conversations.iter().map(|c| c.id.as_str()).collect();

    for m in &parsed.messages {
        // Categoría A: la app declara revocación.
        if m.is_possibly_revoked {
            out.push(DeletedHint {
                category: HintCategory::A,
                message_id: m.id.clone(),
                kind: "revoked_declared".to_string(),
                evidence_strength: EvidenceStrength::Strong,
                note: "La aplicación marcó este mensaje con un código de tipo asociado a revocación. No implica recuperación del contenido.".to_string(),
            });
        }

        // Categoría B: anomalías estructurales.
        if is_empty_body_with_metadata(m) {
            out.push(DeletedHint {
                category: HintCategory::B,
                message_id: m.id.clone(),
                kind: "empty_body_with_metadata".to_string(),
                evidence_strength: EvidenceStrength::Weak,
                note: "Mensaje con cuerpo vacío pero metadatos presentes (sender/timestamp). Puede ser consistente con eliminación, pero también con mensajes multimedia, system o de tipo desconocido.".to_string(),
            });
        }

        if !known_conv_ids.contains(m.conversation_id.as_str())
            && m.conversation_id != "conv:orphan"
        {
            out.push(DeletedHint {
                category: HintCategory::B,
                message_id: m.id.clone(),
                kind: "fk_to_missing_conversation".to_string(),
                evidence_strength: EvidenceStrength::Moderate,
                note: "Mensaje referencia una conversación que no figura en la tabla de sesiones. Puede ser consistente con eliminación de la sesión.".to_string(),
            });
        }

        // Mensajes huérfanos (conv:orphan): el parser no pudo asociar a una
        // sesión válida — moderado por sí solo.
        if m.conversation_id == "conv:orphan" {
            out.push(DeletedHint {
                category: HintCategory::B,
                message_id: m.id.clone(),
                kind: "orphan_message_no_session_key".to_string(),
                evidence_strength: EvidenceStrength::Weak,
                note: "Mensaje sin chat_session válido. Posible borrado de la conversación, migración de schema, o fila parcial en WAL.".to_string(),
            });
        }
    }

    out
}

fn is_empty_body_with_metadata(m: &Message) -> bool {
    let empty_body = m
        .body
        .as_deref()
        .map(|b| b.trim().is_empty())
        .unwrap_or(true);
    let has_metadata = m.timestamp_utc.is_some()
        && (m.sender.is_some() || !matches!(m.direction, MessageDirection::Unknown));
    let is_likely_textual = m
        .message_type_interpreted
        .as_deref()
        .map(|t| t == "text")
        .unwrap_or(false);
    empty_body && has_metadata && is_likely_textual
}
