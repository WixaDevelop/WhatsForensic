//! Comando de exportación XLSX.

use crate::commands::case_cmd::CaseSummary;
use crate::config::AppState;
use crate::error::{AppError, AppErrorKind};
use crate::report::xlsx_writer::{export, ExportInputs, ExportOptions};
use crate::workspace::audit_log;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportXlsxInput {
    pub run_id: String,
    pub output_path: String,
    pub include_raw_row_json: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportXlsxResult {
    pub output_path: String,
    pub bytes_written: u64,
}

#[tauri::command]
pub fn export_xlsx(
    state: tauri::State<'_, AppState>,
    input: ExportXlsxInput,
) -> Result<ExportXlsxResult, AppError> {
    let (case_summary, evidence_entry, audit_log_path, audit_entries) = {
        let guard = state.current_case.read().unwrap();
        let c = guard.as_ref().ok_or_else(|| {
            AppError::new(
                AppErrorKind::InvalidInput,
                "NO_CASE_OPEN",
                "No hay un caso abierto.",
            )
        })?;
        let runs = state.analysis_runs.read().unwrap();
        let run = runs
            .get(&input.run_id)
            .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;
        let entry = c
            .manifest
            .evidences
            .get(&run.evidence_id)
            .cloned()
            .ok_or_else(|| {
                AppError::invalid_input(
                    "EVIDENCE_NOT_FOUND",
                    "El evidence_id del run no existe en este caso.",
                )
            })?;
        let summary = CaseSummary::from(c);
        let audit_path = c.paths.audit_log.clone();
        let entries = audit_log::read_all(&audit_path).unwrap_or_default();
        (summary, entry, audit_path, entries)
    };

    let runs = state.analysis_runs.read().unwrap();
    let run = runs
        .get(&input.run_id)
        .ok_or_else(|| AppError::invalid_input("RUN_NOT_FOUND", "Run no encontrado."))?;

    let output_path = PathBuf::from(&input.output_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;
    }

    let options = ExportOptions {
        include_raw_row_json: input.include_raw_row_json,
        timezone: case_summary.timezone.clone(),
    };

    export(
        ExportInputs {
            case: &case_summary,
            evidence: &evidence_entry,
            parsed: &run.parsed,
            findings: &run.findings,
            audit_entries: &audit_entries,
        },
        &output_path,
        &options,
    )?;
    drop(runs);

    let bytes_written = std::fs::metadata(&output_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let _ = audit_log::record(
        &audit_log_path,
        "export_xlsx",
        json!({
            "run_id": input.run_id,
            "output_path": input.output_path,
            "include_raw_row_json": input.include_raw_row_json,
        }),
        json!({
            "bytes_written": bytes_written,
        }),
    );

    Ok(ExportXlsxResult {
        output_path: input.output_path,
        bytes_written,
    })
}
