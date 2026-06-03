//! Comandos de caso: crear, abrir, cerrar, obtener actual.

use crate::config::{AppState, OpenCase};
use crate::error::{AppError, AppErrorKind};
use crate::workspace::{audit_log, layout::CasePaths, manifest};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseSummary {
    pub case_id: String,
    pub name: String,
    pub description: Option<String>,
    pub investigator: String,
    pub timezone: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub tool_version: String,
    pub case_dir: String,
    pub evidence_count: usize,
}

impl From<&OpenCase> for CaseSummary {
    fn from(c: &OpenCase) -> Self {
        Self {
            case_id: c.manifest.case_id.clone(),
            name: c.manifest.name.clone(),
            description: c.manifest.description.clone(),
            investigator: c.manifest.investigator.clone(),
            timezone: c.manifest.timezone.clone(),
            created_at: c.manifest.created_at,
            tool_version: c.manifest.tool_version.clone(),
            case_dir: c.paths.case_dir.display().to_string().replace('\\', "/"),
            evidence_count: c.manifest.evidences.len(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseCreateInput {
    pub name: String,
    pub description: Option<String>,
    pub investigator: String,
    pub timezone: String,
    pub workspace_root: String,
}

fn build_hash() -> String {
    option_env!("WF_BUILD_HASH")
        .unwrap_or("untracked")
        .to_string()
}

#[tauri::command]
pub fn case_create(
    state: tauri::State<'_, AppState>,
    input: CaseCreateInput,
) -> Result<CaseSummary, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::invalid_input(
            "EMPTY_NAME",
            "El nombre del caso no puede estar vacío.",
        ));
    }
    if input.investigator.trim().is_empty() {
        return Err(AppError::invalid_input(
            "EMPTY_INVESTIGATOR",
            "El nombre del investigador no puede estar vacío.",
        ));
    }
    // Validate timezone string.
    let _: chrono_tz::Tz = input.timezone.parse().map_err(|_| {
        AppError::invalid_input(
            "INVALID_TIMEZONE",
            format!("La zona horaria '{}' no es válida (debe ser IANA: 'America/Argentina/Buenos_Aires').", input.timezone),
        )
    })?;

    let workspace_root = PathBuf::from(&input.workspace_root);
    std::fs::create_dir_all(&workspace_root)
        .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;

    let case_id = Uuid::new_v4().to_string();
    let paths = CasePaths::new(&workspace_root, &case_id);
    std::fs::create_dir_all(&paths.case_dir)
        .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;
    std::fs::create_dir_all(&paths.evidence_root)
        .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;
    std::fs::create_dir_all(&paths.exports_dir)
        .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;

    let lock = manifest::acquire_lock(&paths.lock_file)?;

    let now = Utc::now();
    let manifest_obj = manifest::CaseManifest {
        schema_version: manifest::CURRENT_SCHEMA_VERSION,
        case_id: case_id.clone(),
        name: input.name.trim().to_string(),
        description: input
            .description
            .map(|d| d.trim().to_string())
            .filter(|s| !s.is_empty()),
        investigator: input.investigator.trim().to_string(),
        created_at: now,
        timezone: input.timezone.clone(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        tool_build_hash: build_hash(),
        evidences: BTreeMap::new(),
    };

    manifest::write(&paths.case_json, &manifest_obj)?;
    std::fs::write(
        &paths.tool_version_file,
        format!("{}\n{}\n", env!("CARGO_PKG_VERSION"), build_hash()),
    )
    .map_err(|e| AppError::new(AppErrorKind::Io, "WRITE_FAILED", format!("{e}")))?;

    audit_log::record(
        &paths.audit_log,
        "case_create",
        json!({
            "case_id": case_id,
            "name": manifest_obj.name,
            "investigator": manifest_obj.investigator,
            "timezone": manifest_obj.timezone,
        }),
        json!({"ok": true}),
    )?;

    let open_case = OpenCase {
        manifest: manifest_obj,
        paths,
        _lock: lock,
    };
    let summary = CaseSummary::from(&open_case);

    *state.current_case.write().unwrap() = Some(open_case);

    tracing::info!(case_id = %case_id, "case created");
    Ok(summary)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseOpenInput {
    pub case_dir: String,
}

#[tauri::command]
pub fn case_open(
    state: tauri::State<'_, AppState>,
    input: CaseOpenInput,
) -> Result<CaseSummary, AppError> {
    let case_dir = PathBuf::from(&input.case_dir);
    if !case_dir.is_dir() {
        return Err(AppError::invalid_input(
            "NOT_A_DIRECTORY",
            "La ruta del caso no apunta a un directorio existente.",
        ));
    }
    let case_json = case_dir.join("case.json");
    if !case_json.exists() {
        return Err(AppError::invalid_input(
            "NOT_A_CASE",
            "El directorio no contiene un case.json. ¿Seguro que es un caso de WhatsForensics?",
        ));
    }
    let manifest_obj = manifest::read(&case_json)?;
    let paths = CasePaths::new(
        case_dir.parent().unwrap_or(&case_dir),
        &manifest_obj.case_id,
    );
    // If case_dir name differs from case_id, layout would be wrong. Validate:
    if paths.case_dir != case_dir {
        return Err(AppError::invalid_input(
            "CASE_DIR_MISMATCH",
            "El directorio del caso no coincide con el case_id del manifest. ¿Fue movido o renombrado?",
        )
        .with_context("expected", json!(paths.case_dir.display().to_string()))
        .with_context("provided", json!(case_dir.display().to_string())));
    }
    let lock = manifest::acquire_lock(&paths.lock_file)?;

    audit_log::record(
        &paths.audit_log,
        "case_open",
        json!({"case_id": manifest_obj.case_id}),
        json!({"ok": true}),
    )?;

    let open_case = OpenCase {
        manifest: manifest_obj,
        paths,
        _lock: lock,
    };
    let summary = CaseSummary::from(&open_case);
    *state.current_case.write().unwrap() = Some(open_case);

    tracing::info!(case_id = %summary.case_id, "case opened");
    Ok(summary)
}

#[tauri::command]
pub fn case_close(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let case = state.current_case.write().unwrap().take();
    if let Some(c) = case {
        let _ = audit_log::record(
            &c.paths.audit_log,
            "case_close",
            json!({"case_id": c.manifest.case_id}),
            json!({"ok": true}),
        );
        tracing::info!(case_id = %c.manifest.case_id, "case closed");
    }
    Ok(())
}

#[tauri::command]
pub fn case_get_current(
    state: tauri::State<'_, AppState>,
) -> Result<Option<CaseSummary>, AppError> {
    Ok(state
        .current_case
        .read()
        .unwrap()
        .as_ref()
        .map(CaseSummary::from))
}
