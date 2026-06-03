//! Manifest del caso (`case.json`) con file locking cross-platform.
//!
//! Schema versionado (`schema_version`) para permitir migración futura.
//!
//! El file lock vive en `<case_dir>/.lock` y se mantiene durante toda la
//! sesión: impide que dos instancias abran el mismo caso.

use crate::error::{AppError, AppErrorKind};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::path::Path;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaseManifest {
    pub schema_version: u32,
    pub case_id: String,
    pub name: String,
    pub description: Option<String>,
    pub investigator: String,
    pub created_at: DateTime<Utc>,
    pub timezone: String,
    pub tool_version: String,
    pub tool_build_hash: String,
    pub evidences: BTreeMap<String, EvidenceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceEntry {
    pub evidence_id: String,
    pub filename: String,
    pub source_type: Option<String>,
    pub original_path: String,
    pub original_size: u64,
    pub original_sha256: String,
    pub pristine_sha256: String,
    pub working_sha256: String,
    pub sidecar_hashes: BTreeMap<String, String>,
    pub has_wal: bool,
    pub has_shm: bool,
    pub has_journal: bool,
    pub ingested_at: DateTime<Utc>,
    pub analysis_mode_used: Option<String>,
}

/// Lee el manifest desde disco. Falla si el archivo no existe o si su schema
/// no coincide con la versión actual conocida.
pub fn read(case_json: &Path) -> Result<CaseManifest, AppError> {
    let content = std::fs::read_to_string(case_json).map_err(|e| {
        AppError::new(
            AppErrorKind::Io,
            "READ_MANIFEST_FAILED",
            format!("No se pudo leer el manifest: {e}"),
        )
        .with_context("path", serde_json::json!(case_json.display().to_string()))
    })?;
    let manifest: CaseManifest = serde_json::from_str(&content).map_err(|e| {
        AppError::new(
            AppErrorKind::InvalidInput,
            "PARSE_MANIFEST_FAILED",
            format!("El manifest del caso no es JSON válido: {e}"),
        )
    })?;
    if manifest.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(AppError::new(
            AppErrorKind::InvalidInput,
            "MANIFEST_NEWER_VERSION",
            "El manifest pertenece a una versión más nueva de la herramienta. Actualizá WhatsForensics.",
        )
        .with_context("found_version", serde_json::json!(manifest.schema_version))
        .with_context("supported_version", serde_json::json!(CURRENT_SCHEMA_VERSION)));
    }
    Ok(manifest)
}

/// Escribe el manifest atómicamente: tmp + rename.
pub fn write(case_json: &Path, manifest: &CaseManifest) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| AppError::new(AppErrorKind::Internal, "SERIALIZE_FAILED", format!("{e}")))?;
    let tmp = case_json.with_extension("json.tmp");
    std::fs::write(&tmp, json)
        .map_err(|e| AppError::new(AppErrorKind::Io, "WRITE_FAILED", format!("{e}")))?;
    std::fs::rename(&tmp, case_json)
        .map_err(|e| AppError::new(AppErrorKind::Io, "RENAME_FAILED", format!("{e}")))?;
    Ok(())
}

/// Adquiere el lock exclusivo del caso. Devuelve el handle del archivo de lock,
/// que el caller debe mantener vivo durante toda la sesión.
///
/// En `fs2` (Linux flock + Windows LockFileEx), el OS libera automáticamente
/// el lock cuando el proceso termina, incluso si crashea — el lock no queda
/// stale en ese sentido. Sí queda stale si la app se cuelga sin terminar.
pub fn acquire_lock(lock_path: &Path) -> Result<LockHandle, AppError> {
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;
    }
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|e| AppError::new(AppErrorKind::Io, "OPEN_LOCK_FAILED", format!("{e}")))?;

    match file.try_lock_exclusive() {
        Ok(()) => Ok(LockHandle { _file: file }),
        Err(e) => Err(AppError::new(
            AppErrorKind::Permission,
            "CASE_ALREADY_OPEN",
            "El caso ya está abierto en otra instancia. Cerrá la otra ventana e intentá nuevamente.",
        )
        .with_context("io_error", serde_json::json!(e.to_string()))),
    }
}

/// Handle del lock. Mantenelo en `AppState` durante toda la sesión del caso.
pub struct LockHandle {
    _file: File,
}
