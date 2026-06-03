//! Comandos de análisis: descubrir parsers, correr parser, paginar resultados.

use crate::analysis::{self, deleted_hints, gaps, stats, AnalysisFindings};
use crate::config::{AnalysisRun, AppState};
use crate::db::{introspect, opener};
use crate::error::{AppError, AppErrorKind};
use crate::parsers::{
    self,
    common_model::{Call, Conversation, Message, ParsedEvidence, ParserWarning},
    source_detect,
    traits::Confidence,
};
use crate::workspace::audit_log;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParserDescriptor {
    pub key: String,
    pub display_name: String,
}

#[tauri::command]
pub fn analysis_list_parsers() -> Result<Vec<ParserDescriptor>, AppError> {
    let parsers = parsers::all_parsers()?;
    Ok(parsers
        .iter()
        .map(|p| ParserDescriptor {
            key: p.key().to_string(),
            display_name: p.display_name().to_string(),
        })
        .collect())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectParsersInput {
    pub evidence_id: String,
    pub mode: opener::OpenMode,
}

#[tauri::command]
pub async fn analysis_detect_parsers(
    state: tauri::State<'_, AppState>,
    input: DetectParsersInput,
) -> Result<Vec<source_detect::ParserMatch>, AppError> {
    let (working_path, filename) = working_path_for(&state, &input.evidence_id)?;
    let conn = opener::open(&working_path, input.mode).await?;
    let schema = introspect::snapshot(&conn)?;
    drop(conn);

    let parsers_list = parsers::all_parsers()?;
    let refs: Vec<&dyn crate::parsers::traits::Parser> =
        parsers_list.iter().map(|b| b.as_ref()).collect();
    Ok(source_detect::suggest(&filename, &schema, &refs))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRunInput {
    pub evidence_id: String,
    pub parser_key: String,
    pub mode: opener::OpenMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRunSummary {
    pub run_id: String,
    pub evidence_id: String,
    pub parser_key: String,
    pub mode: String,
    pub source_kind: String,
    pub schema_version_used: String,
    pub schema_verified: bool,
    pub conversation_count: usize,
    pub message_count: usize,
    pub call_count: usize,
    pub warning_count: usize,
    pub revoked_count: usize,
    pub gap_count: usize,
    pub deleted_hint_count: usize,
}

#[tauri::command]
pub async fn analysis_run(
    state: tauri::State<'_, AppState>,
    input: AnalysisRunInput,
) -> Result<AnalysisRunSummary, AppError> {
    let (working_path, _filename) = working_path_for(&state, &input.evidence_id)?;
    let audit_log_path = {
        let guard = state.current_case.read().unwrap();
        guard
            .as_ref()
            .ok_or_else(|| {
                AppError::new(
                    AppErrorKind::InvalidInput,
                    "NO_CASE_OPEN",
                    "Sin caso abierto.",
                )
            })?
            .paths
            .audit_log
            .clone()
    };

    let parser = parsers::by_key(&input.parser_key)?;
    let conn = opener::open(&working_path, input.mode).await?;
    let schema = introspect::snapshot(&conn)?;
    let parsed = parser.parse(&conn)?;
    let findings = analysis::compute(&conn, &schema, &parsed)?;
    drop(conn);

    if matches!(input.mode, opener::OpenMode::WithWal) {
        let _ = opener::rehash(&working_path).await;
    }

    let run_id = Uuid::new_v4().to_string();
    let parsed_arc = Arc::new(parsed);
    let findings_arc = Arc::new(findings);
    let summary = build_summary(&run_id, &input, parsed_arc.as_ref(), findings_arc.as_ref());
    state.analysis_runs.write().unwrap().insert(
        run_id.clone(),
        AnalysisRun {
            run_id: run_id.clone(),
            evidence_id: input.evidence_id.clone(),
            parser_key: input.parser_key.clone(),
            mode: input.mode.as_str().to_string(),
            parsed: parsed_arc.clone(),
            findings: findings_arc.clone(),
        },
    );

    let _ = audit_log::record(
        &audit_log_path,
        "analysis_run",
        json!({
            "evidence_id": input.evidence_id,
            "parser_key": input.parser_key,
            "mode": input.mode.as_str(),
            "run_id": run_id,
        }),
        json!({
            "schema_verified": parsed_arc.schema_verified,
            "conversation_count": summary.conversation_count,
            "message_count": summary.message_count,
            "call_count": summary.call_count,
            "warning_count": summary.warning_count,
            "gap_count": summary.gap_count,
            "deleted_hint_count": summary.deleted_hint_count,
        }),
    );

    tracing::info!(
        run_id = %run_id,
        parser = %input.parser_key,
        messages = summary.message_count,
        calls = summary.call_count,
        "analysis run finished"
    );

    Ok(summary)
}

fn build_summary(
    run_id: &str,
    input: &AnalysisRunInput,
    parsed: &ParsedEvidence,
    findings: &AnalysisFindings,
) -> AnalysisRunSummary {
    AnalysisRunSummary {
        run_id: run_id.to_string(),
        evidence_id: input.evidence_id.clone(),
        parser_key: input.parser_key.clone(),
        mode: input.mode.as_str().to_string(),
        source_kind: parsed.source_kind.clone(),
        schema_version_used: parsed.schema_version_used.clone(),
        schema_verified: parsed.schema_verified,
        conversation_count: parsed.conversations.len(),
        message_count: parsed.messages.len(),
        call_count: parsed.calls.len(),
        warning_count: parsed.warnings.len(),
        revoked_count: parsed
            .messages
            .iter()
            .filter(|m| m.is_possibly_revoked)
            .count(),
        gap_count: findings.gaps.len(),
        deleted_hint_count: findings.deleted_hints.len(),
    }
}

#[tauri::command]
pub fn analysis_get_gaps(
    state: tauri::State<'_, AppState>,
    run_id: String,
) -> Result<Vec<gaps::Gap>, AppError> {
    let guard = state.analysis_runs.read().unwrap();
    let run = guard
        .get(&run_id)
        .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;
    Ok(run.findings.gaps.clone())
}

#[tauri::command]
pub fn analysis_get_deleted_hints(
    state: tauri::State<'_, AppState>,
    run_id: String,
) -> Result<Vec<deleted_hints::DeletedHint>, AppError> {
    let guard = state.analysis_runs.read().unwrap();
    let run = guard
        .get(&run_id)
        .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;
    Ok(run.findings.deleted_hints.clone())
}

#[tauri::command]
pub fn analysis_get_stats(
    state: tauri::State<'_, AppState>,
    run_id: String,
) -> Result<stats::Stats, AppError> {
    let guard = state.analysis_runs.read().unwrap();
    let run = guard
        .get(&run_id)
        .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;
    Ok(run.findings.stats.clone())
}

// ---------------------------------------------------------------------------
// Paginación
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest {
    pub run_id: String,
    pub page: usize,
    pub page_size: usize,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedMessages {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub messages: Vec<Message>,
}

#[tauri::command]
pub fn analysis_query_messages(
    state: tauri::State<'_, AppState>,
    request: PageRequest,
) -> Result<PagedMessages, AppError> {
    let guard = state.analysis_runs.read().unwrap();
    let run = guard
        .get(&request.run_id)
        .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;
    let parsed = run.parsed.clone();
    drop(guard);

    let filtered: Vec<&Message> = match request.filter.as_deref() {
        Some(q) if !q.is_empty() => {
            let lower = q.to_lowercase();
            parsed
                .messages
                .iter()
                .filter(|m| {
                    m.body
                        .as_deref()
                        .map(|b| b.to_lowercase().contains(&lower))
                        .unwrap_or(false)
                        || m.sender
                            .as_deref()
                            .map(|s| s.to_lowercase().contains(&lower))
                            .unwrap_or(false)
                })
                .collect()
        }
        _ => parsed.messages.iter().collect(),
    };
    let total = filtered.len();
    let start = request.page.saturating_mul(request.page_size);
    let end = start.saturating_add(request.page_size).min(total);
    let slice: Vec<Message> = if start >= total {
        Vec::new()
    } else {
        filtered[start..end].iter().map(|m| (*m).clone()).collect()
    };
    Ok(PagedMessages {
        total,
        page: request.page,
        page_size: request.page_size,
        messages: slice,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedCalls {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub calls: Vec<Call>,
}

#[tauri::command]
pub fn analysis_query_calls(
    state: tauri::State<'_, AppState>,
    request: PageRequest,
) -> Result<PagedCalls, AppError> {
    let guard = state.analysis_runs.read().unwrap();
    let run = guard
        .get(&request.run_id)
        .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;
    let parsed = run.parsed.clone();
    drop(guard);
    let total = parsed.calls.len();
    let start = request.page.saturating_mul(request.page_size);
    let end = start.saturating_add(request.page_size).min(total);
    let slice: Vec<Call> = if start >= total {
        Vec::new()
    } else {
        parsed.calls[start..end].to_vec()
    };
    Ok(PagedCalls {
        total,
        page: request.page,
        page_size: request.page_size,
        calls: slice,
    })
}

#[tauri::command]
pub fn analysis_query_conversations(
    state: tauri::State<'_, AppState>,
    run_id: String,
) -> Result<Vec<Conversation>, AppError> {
    let guard = state.analysis_runs.read().unwrap();
    let run = guard
        .get(&run_id)
        .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;
    Ok(run.parsed.conversations.clone())
}

#[tauri::command]
pub fn analysis_get_warnings(
    state: tauri::State<'_, AppState>,
    run_id: String,
) -> Result<Vec<ParserWarning>, AppError> {
    let guard = state.analysis_runs.read().unwrap();
    let run = guard
        .get(&run_id)
        .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;
    Ok(run.parsed.warnings.clone())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRunDescriptor {
    pub run_id: String,
    pub evidence_id: String,
    pub parser_key: String,
    pub mode: String,
    pub source_kind: String,
    pub schema_verified: bool,
    pub message_count: usize,
    pub call_count: usize,
}

#[tauri::command]
pub fn analysis_list_runs(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<AnalysisRunDescriptor>, AppError> {
    let guard = state.analysis_runs.read().unwrap();
    Ok(guard
        .values()
        .map(|r| AnalysisRunDescriptor {
            run_id: r.run_id.clone(),
            evidence_id: r.evidence_id.clone(),
            parser_key: r.parser_key.clone(),
            mode: r.mode.clone(),
            source_kind: r.parsed.source_kind.clone(),
            schema_verified: r.parsed.schema_verified,
            message_count: r.parsed.messages.len(),
            call_count: r.parsed.calls.len(),
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn working_path_for(
    state: &tauri::State<'_, AppState>,
    evidence_id: &str,
) -> Result<(std::path::PathBuf, String), AppError> {
    let guard = state.current_case.read().unwrap();
    let c = guard.as_ref().ok_or_else(|| {
        AppError::new(
            AppErrorKind::InvalidInput,
            "NO_CASE_OPEN",
            "Sin caso abierto.",
        )
    })?;
    let entry = c
        .manifest
        .evidences
        .get(evidence_id)
        .ok_or_else(|| {
            AppError::invalid_input("EVIDENCE_NOT_FOUND", "El evidence_id solicitado no existe.")
        })?
        .clone();
    let working_path = c
        .paths
        .evidence_dir(evidence_id)
        .join("working")
        .join(&entry.filename);
    Ok((working_path, entry.filename))
}

/// Marker — Confidence used by parser_match deserialization tests if any.
#[allow(dead_code)]
fn _force_use_confidence(_c: Confidence) {}
