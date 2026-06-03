//! Estadísticas agregadas. `BTreeMap` para orden determinístico.

use crate::parsers::common_model::{MessageDirection, ParsedEvidence};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub total_conversations: usize,
    pub total_messages: usize,
    pub total_calls: usize,
    pub total_revoked: usize,
    pub first_message_utc: Option<chrono::DateTime<chrono::Utc>>,
    pub last_message_utc: Option<chrono::DateTime<chrono::Utc>>,
    pub messages_per_day: BTreeMap<String, usize>, // "YYYY-MM-DD"
    pub messages_per_type: BTreeMap<String, usize>,
    pub messages_per_direction: BTreeMap<String, usize>,
    pub messages_per_conversation: BTreeMap<String, usize>,
    pub calls_per_type: BTreeMap<String, usize>,
    pub calls_per_direction: BTreeMap<String, usize>,
}

pub fn compute(parsed: &ParsedEvidence) -> Stats {
    let mut stats = Stats {
        total_conversations: parsed.conversations.len(),
        total_messages: parsed.messages.len(),
        total_calls: parsed.calls.len(),
        ..Default::default()
    };

    let mut first_ts: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_ts: Option<chrono::DateTime<chrono::Utc>> = None;

    for m in &parsed.messages {
        if m.is_possibly_revoked {
            stats.total_revoked += 1;
        }
        if let Some(ts) = m.timestamp_utc {
            if first_ts.is_none_or(|f| ts < f) {
                first_ts = Some(ts);
            }
            if last_ts.is_none_or(|l| ts > l) {
                last_ts = Some(ts);
            }
            let day: NaiveDate = ts.date_naive();
            *stats
                .messages_per_day
                .entry(day.format("%Y-%m-%d").to_string())
                .or_insert(0) += 1;
        }
        let type_key = m
            .message_type_interpreted
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        *stats.messages_per_type.entry(type_key).or_insert(0) += 1;
        let dir_key = match m.direction {
            MessageDirection::Incoming => "incoming",
            MessageDirection::Outgoing => "outgoing",
            MessageDirection::Unknown => "unknown",
        };
        *stats
            .messages_per_direction
            .entry(dir_key.to_string())
            .or_insert(0) += 1;
        *stats
            .messages_per_conversation
            .entry(m.conversation_id.clone())
            .or_insert(0) += 1;
    }
    stats.first_message_utc = first_ts;
    stats.last_message_utc = last_ts;

    for c in &parsed.calls {
        let type_key = c
            .call_type_interpreted
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        *stats.calls_per_type.entry(type_key).or_insert(0) += 1;
        let dir_key = match c.direction {
            MessageDirection::Incoming => "incoming",
            MessageDirection::Outgoing => "outgoing",
            MessageDirection::Unknown => "unknown",
        };
        *stats
            .calls_per_direction
            .entry(dir_key.to_string())
            .or_insert(0) += 1;
    }

    stats
}
