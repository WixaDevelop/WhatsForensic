//! Comandos de acceso a SQLite y análisis básico de schema.

use crate::config::AppState;
use crate::db::{
    introspect::{self, SchemaSnapshot},
    opener::{self, OpenMode},
};
use crate::error::{AppError, AppErrorKind};
use crate::workspace::audit_log;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceIntrospectInput {
    pub evidence_id: String,
    pub mode: OpenMode,
}

/// Abre la working copy de la evidencia con el modo solicitado y devuelve
/// un snapshot del schema. Después de cerrar, re-hashea la working y lo
/// registra en audit.log.
#[tauri::command]
pub async fn evidence_introspect(
    state: tauri::State<'_, AppState>,
    input: EvidenceIntrospectInput,
) -> Result<SchemaSnapshot, AppError> {
    let (audit_log_path, working_path, case_json_path) = {
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
        let working_path = c
            .paths
            .evidence_dir(&input.evidence_id)
            .join("working")
            .join(&entry.filename);
        (
            c.paths.audit_log.clone(),
            working_path,
            c.paths.case_json.clone(),
        )
    };

    let hash_before = if matches!(input.mode, OpenMode::WithWal) {
        Some(opener::rehash(&working_path).await?)
    } else {
        None
    };

    let snapshot = {
        let conn = opener::open(&working_path, input.mode).await?;
        introspect::snapshot(&conn)?
    };

    let hash_after = if let Some(before) = hash_before {
        let after = opener::rehash(&working_path).await?;
        if after != before {
            tracing::info!(
                evidence_id = %input.evidence_id,
                before = %before,
                after = %after,
                "working copy changed after with_wal session (expected: checkpoint)"
            );
        }
        Some((before, after))
    } else {
        None
    };

    // Update analysis_mode_used in manifest.
    let mut current_manifest = crate::workspace::manifest::read(&case_json_path)?;
    if let Some(entry) = current_manifest.evidences.get_mut(&input.evidence_id) {
        entry.analysis_mode_used = Some(input.mode.as_str().to_string());
    }
    crate::workspace::manifest::write(&case_json_path, &current_manifest)?;
    if let Some(c) = state.current_case.write().unwrap().as_mut() {
        c.manifest = current_manifest;
    }

    let _ = audit_log::record(
        &audit_log_path,
        "evidence_introspect",
        json!({
            "evidence_id": input.evidence_id,
            "mode": input.mode.as_str(),
        }),
        json!({
            "table_count": snapshot.tables.len(),
            "index_count": snapshot.indexes.len(),
            "sqlite_version": snapshot.sqlite_version,
            "rehash": hash_after.map(|(b, a)| json!({"before": b, "after": a, "changed": b != a})),
        }),
    );

    Ok(snapshot)
}
