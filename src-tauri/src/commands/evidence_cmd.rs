//! Comandos de evidencia: preview, ingest (con Channel), list, verify.

use crate::config::AppState;
use crate::error::{AppError, AppErrorKind};
use crate::evidence::{
    hasher::HashProgress,
    ingest::{self, EvidencePreview, IngestStep},
};
use crate::workspace::{audit_log, manifest};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use tauri::ipc::Channel;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidencePreviewInput {
    pub path: String,
}

#[tauri::command]
pub async fn evidence_preview(input: EvidencePreviewInput) -> Result<EvidencePreview, AppError> {
    let path = PathBuf::from(&input.path);
    ingest::preview(&path).await
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestProgressEvent {
    pub run_id: String,
    pub step: IngestStep,
    pub progress: HashProgress,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceIngestInput {
    pub path: String,
    pub declared_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceIngestResult {
    pub run_id: String,
    pub evidence_id: String,
}

/// Ingesta async con streaming de progreso vía Channel. Devuelve el `run_id` y
/// `evidence_id` cuando termina. El frontend puede cancelar con `task_cancel(run_id)`.
#[tauri::command]
pub async fn evidence_ingest(
    state: tauri::State<'_, AppState>,
    input: EvidenceIngestInput,
    on_event: Channel<IngestProgressEvent>,
) -> Result<EvidenceIngestResult, AppError> {
    let path = PathBuf::from(&input.path);
    if !path.is_file() {
        return Err(AppError::invalid_input(
            "FILE_NOT_FOUND",
            "El archivo provisto no existe o no es un archivo regular.",
        ));
    }

    // Snapshot del estado mínimo necesario, sin retener el guard durante el await.
    let (audit_log_path, evidence_dir, case_json_path) = {
        let guard = state.current_case.read().unwrap();
        let c = guard.as_ref().ok_or_else(|| {
            AppError::new(
                AppErrorKind::InvalidInput,
                "NO_CASE_OPEN",
                "No hay un caso abierto. Abrí o creá uno primero.",
            )
        })?;
        let evidence_id_local = Uuid::new_v4().to_string();
        let evidence_dir = c.paths.evidence_dir(&evidence_id_local);
        (
            c.paths.audit_log.clone(),
            (evidence_id_local, evidence_dir),
            c.paths.case_json.clone(),
        )
    };
    let (evidence_id, evidence_dir) = evidence_dir;

    let token = CancellationToken::new();
    let run_id = state.register_task(token.clone());

    let _ = audit_log::record(
        &audit_log_path,
        "evidence_ingest_start",
        json!({
            "evidence_id": evidence_id,
            "path": input.path,
            "declared_type": input.declared_type,
            "run_id": run_id,
        }),
        json!({}),
    );

    let run_id_for_cb = run_id.clone();
    let on_event_for_cb = on_event.clone();
    let progress_cb = move |step: IngestStep, hp: HashProgress| {
        let _ = on_event_for_cb.send(IngestProgressEvent {
            run_id: run_id_for_cb.clone(),
            step,
            progress: hp,
        });
    };

    let report_result = ingest::ingest(
        &path,
        &evidence_id,
        &evidence_dir,
        token.clone(),
        progress_cb,
    )
    .await;

    state.drop_task(&run_id);

    let report = match report_result {
        Ok(r) => r,
        Err(e) => {
            let _ = audit_log::record(
                &audit_log_path,
                "evidence_ingest_failed",
                json!({"evidence_id": evidence_id, "run_id": run_id}),
                json!({"error_code": e.code, "error_kind": format!("{:?}", e.kind)}),
            );
            return Err(e);
        }
    };

    // Update manifest with the new evidence entry.
    let mut current_manifest = manifest::read(&case_json_path)?;
    current_manifest.evidences.insert(
        evidence_id.clone(),
        manifest::EvidenceEntry {
            evidence_id: evidence_id.clone(),
            filename: report.filename.clone(),
            source_type: input.declared_type.clone(),
            original_path: report.original_path.clone(),
            original_size: report.original_size,
            original_sha256: report.original_sha256.clone(),
            pristine_sha256: report.pristine_sha256.clone(),
            working_sha256: report.working_sha256.clone(),
            sidecar_hashes: report.sidecar_hashes.clone(),
            has_wal: report.has_wal,
            has_shm: report.has_shm,
            has_journal: report.has_journal,
            ingested_at: Utc::now(),
            analysis_mode_used: None,
        },
    );
    manifest::write(&case_json_path, &current_manifest)?;

    // Reload manifest into AppState.
    if let Some(c) = state.current_case.write().unwrap().as_mut() {
        c.manifest = current_manifest;
    }

    let _ = audit_log::record(
        &audit_log_path,
        "evidence_ingest_done",
        json!({"evidence_id": evidence_id, "run_id": run_id}),
        json!({
            "original_sha256": report.original_sha256,
            "pristine_sha256": report.pristine_sha256,
            "working_sha256": report.working_sha256,
            "has_wal": report.has_wal,
            "has_shm": report.has_shm,
            "has_journal": report.has_journal,
        }),
    );

    tracing::info!(evidence_id = %evidence_id, "evidence ingested");
    Ok(EvidenceIngestResult {
        run_id,
        evidence_id,
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceSummary {
    pub evidence_id: String,
    pub filename: String,
    pub source_type: Option<String>,
    pub original_size: u64,
    pub original_sha256: String,
    pub pristine_sha256: String,
    pub working_sha256: String,
    pub has_wal: bool,
    pub has_shm: bool,
    pub has_journal: bool,
    pub ingested_at: chrono::DateTime<chrono::Utc>,
}

impl From<&manifest::EvidenceEntry> for EvidenceSummary {
    fn from(e: &manifest::EvidenceEntry) -> Self {
        Self {
            evidence_id: e.evidence_id.clone(),
            filename: e.filename.clone(),
            source_type: e.source_type.clone(),
            original_size: e.original_size,
            original_sha256: e.original_sha256.clone(),
            pristine_sha256: e.pristine_sha256.clone(),
            working_sha256: e.working_sha256.clone(),
            has_wal: e.has_wal,
            has_shm: e.has_shm,
            has_journal: e.has_journal,
            ingested_at: e.ingested_at,
        }
    }
}

#[tauri::command]
pub fn evidence_list(state: tauri::State<'_, AppState>) -> Result<Vec<EvidenceSummary>, AppError> {
    let guard = state.current_case.read().unwrap();
    let c = guard.as_ref().ok_or_else(|| {
        AppError::new(
            AppErrorKind::InvalidInput,
            "NO_CASE_OPEN",
            "No hay un caso abierto.",
        )
    })?;
    Ok(c.manifest
        .evidences
        .values()
        .map(EvidenceSummary::from)
        .collect())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceVerifyInput {
    pub evidence_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrityReport {
    pub evidence_id: String,
    pub pristine_matches: bool,
    pub working_matches: bool,
    pub expected_pristine_sha256: String,
    pub actual_pristine_sha256: String,
    pub expected_working_sha256: String,
    pub actual_working_sha256: String,
}

/// Re-hashea las copias pristine y working y compara con los hashes registrados
/// en el manifest.
#[tauri::command]
pub async fn evidence_verify(
    state: tauri::State<'_, AppState>,
    input: EvidenceVerifyInput,
) -> Result<IntegrityReport, AppError> {
    let (audit_log_path, evidence_entry, evidence_dir) = {
        let guard = state.current_case.read().unwrap();
        let c = guard.as_ref().ok_or_else(|| {
            AppError::new(
                AppErrorKind::InvalidInput,
                "NO_CASE_OPEN",
                "No hay un caso abierto.",
            )
        })?;
        let entry = c
            .manifest
            .evidences
            .get(&input.evidence_id)
            .ok_or_else(|| {
                AppError::invalid_input(
                    "EVIDENCE_NOT_FOUND",
                    "El evidence_id solicitado no existe en este caso.",
                )
            })?
            .clone();
        (
            c.paths.audit_log.clone(),
            entry,
            c.paths.evidence_dir(&input.evidence_id),
        )
    };

    let pristine_path = evidence_dir.join("pristine").join(&evidence_entry.filename);
    let working_path = evidence_dir.join("working").join(&evidence_entry.filename);

    let actual_pristine = crate::evidence::hasher::hash_file_streaming(
        &pristine_path,
        CancellationToken::new(),
        |_| {},
    )
    .await?;
    let actual_working = crate::evidence::hasher::hash_file_streaming(
        &working_path,
        CancellationToken::new(),
        |_| {},
    )
    .await?;

    let pristine_matches = actual_pristine == evidence_entry.pristine_sha256;
    let working_matches = actual_working == evidence_entry.working_sha256;

    let _ = audit_log::record(
        &audit_log_path,
        "evidence_verify",
        json!({"evidence_id": input.evidence_id}),
        json!({
            "pristine_matches": pristine_matches,
            "working_matches": working_matches,
            "actual_pristine_sha256": actual_pristine,
            "actual_working_sha256": actual_working,
        }),
    );

    Ok(IntegrityReport {
        evidence_id: input.evidence_id,
        pristine_matches,
        working_matches,
        expected_pristine_sha256: evidence_entry.pristine_sha256,
        actual_pristine_sha256: actual_pristine,
        expected_working_sha256: evidence_entry.working_sha256,
        actual_working_sha256: actual_working,
    })
}

#[tauri::command]
pub fn task_cancel(state: tauri::State<'_, AppState>, run_id: String) -> Result<bool, AppError> {
    Ok(state.cancel_task(&run_id))
}

#[tauri::command]
pub fn default_workspace_root() -> Result<Option<String>, AppError> {
    Ok(crate::workspace::layout::default_workspace_root()
        .map(|p| p.display().to_string().replace('\\', "/")))
}
