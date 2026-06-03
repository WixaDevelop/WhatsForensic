//! Ingesta de evidencia: detección de sidecars, doble copia, verificación.
//!
//! Flujo (regla forense no negociable 1, 2, 3, 4):
//! 1. Hash del original (streaming, cancelable).
//! 2. Copia a `pristine/` + re-hash + verificación.
//! 3. Copia `pristine/` → `working/` + hash.
//! 4. Detección y copia de hermanos (`-wal`, `-shm`, `-journal`).
//! 5. Sidecars también se hashean y se copian a ambas carpetas.
//!
//! El archivo original **nunca** se modifica ni se abre con SQLite.

use crate::error::{AppError, AppErrorKind};
use crate::evidence::{hasher, header};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarSet {
    pub wal: Option<PathBuf>,
    pub shm: Option<PathBuf>,
    pub journal: Option<PathBuf>,
}

impl SidecarSet {
    pub fn any(&self) -> bool {
        self.wal.is_some() || self.shm.is_some() || self.journal.is_some()
    }
}

/// Detecta los sidecars (`-wal`, `-shm`, `-journal`) adyacentes al archivo.
pub fn detect_sidecars(path: &Path) -> SidecarSet {
    let mut set = SidecarSet::default();
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return set;
    };
    let dir = path.parent().unwrap_or_else(|| Path::new("."));

    for (suffix, slot) in [
        ("-wal", &mut set.wal),
        ("-shm", &mut set.shm),
        ("-journal", &mut set.journal),
    ] {
        let p = dir.join(format!("{name}{suffix}"));
        if p.exists() {
            *slot = Some(p);
        }
    }
    set
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidencePreview {
    pub original_path: String,
    pub size: u64,
    pub header: header::SqliteHeader,
    pub sidecars: SidecarSet,
}

/// Vista previa del archivo: tamaño + header + sidecars. **No** hashea, **no** copia.
pub async fn preview(path: &Path) -> Result<EvidencePreview, AppError> {
    let metadata = tokio::fs::metadata(path).await.map_err(|e| {
        AppError::new(
            AppErrorKind::Io,
            "FILE_NOT_FOUND",
            format!("No se encontró el archivo: {e}"),
        )
        .with_context("path", serde_json::json!(path.display().to_string()))
    })?;
    let header = header::validate(path).await?;
    let sidecars = detect_sidecars(path);
    Ok(EvidencePreview {
        original_path: path.display().to_string().replace('\\', "/"),
        size: metadata.len(),
        header,
        sidecars,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestReport {
    pub evidence_id: String,
    pub filename: String,
    pub original_path: String,
    pub original_size: u64,
    pub original_sha256: String,
    pub pristine_sha256: String,
    pub working_sha256: String,
    pub sidecar_hashes: BTreeMap<String, String>,
    pub has_wal: bool,
    pub has_shm: bool,
    pub has_journal: bool,
}

/// Pipeline de ingesta. `evidence_dir` debe existir ya (creado por el caller).
///
/// El callback `progress` se invoca con `HashProgress` durante cada hash. Los
/// pasos (`step`) ayudan al callback a presentar el progreso global.
pub async fn ingest<F>(
    original: &Path,
    evidence_id: &str,
    evidence_dir: &Path,
    cancel: CancellationToken,
    progress: F,
) -> Result<IngestReport, AppError>
where
    F: Fn(IngestStep, hasher::HashProgress) + Clone,
{
    let pristine_dir = evidence_dir.join("pristine");
    let working_dir = evidence_dir.join("working");
    tokio::fs::create_dir_all(&pristine_dir)
        .await
        .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;
    tokio::fs::create_dir_all(&working_dir)
        .await
        .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;

    let filename = original
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            AppError::new(
                AppErrorKind::InvalidInput,
                "INVALID_FILENAME",
                "El nombre del archivo no es válido.",
            )
        })?
        .to_string();

    let metadata = tokio::fs::metadata(original)
        .await
        .map_err(|e| AppError::new(AppErrorKind::Io, "FILE_NOT_FOUND", format!("{e}")))?;
    let original_size = metadata.len();

    let p_hashing_original = progress.clone();
    let original_sha = hasher::hash_file_streaming(original, cancel.clone(), move |hp| {
        p_hashing_original(IngestStep::HashingOriginal, hp)
    })
    .await?;

    let pristine_path = pristine_dir.join(&filename);
    tokio::fs::copy(original, &pristine_path)
        .await
        .map_err(|e| AppError::new(AppErrorKind::Io, "COPY_FAILED", format!("{e}")))?;

    let p_hashing_pristine = progress.clone();
    let pristine_sha = hasher::hash_file_streaming(&pristine_path, cancel.clone(), move |hp| {
        p_hashing_pristine(IngestStep::HashingPristine, hp)
    })
    .await?;
    if pristine_sha != original_sha {
        return Err(AppError::new(
            AppErrorKind::Integrity,
            "HASH_MISMATCH_PRISTINE",
            "El hash de la copia pristine no coincide con el original.",
        )
        .with_context("original_sha256", serde_json::json!(original_sha))
        .with_context("pristine_sha256", serde_json::json!(pristine_sha)));
    }

    let working_path = working_dir.join(&filename);
    tokio::fs::copy(&pristine_path, &working_path)
        .await
        .map_err(|e| AppError::new(AppErrorKind::Io, "COPY_FAILED", format!("{e}")))?;
    let p_hashing_working = progress.clone();
    let working_sha = hasher::hash_file_streaming(&working_path, cancel.clone(), move |hp| {
        p_hashing_working(IngestStep::HashingWorking, hp)
    })
    .await?;

    let sidecars = detect_sidecars(original);
    // Materializar paths como owned PathBuf antes del loop, para no retener
    // referencias a `sidecars` cruzando los awaits (Send requirement).
    let sidecar_paths: Vec<PathBuf> = [
        sidecars.wal.clone(),
        sidecars.shm.clone(),
        sidecars.journal.clone(),
    ]
    .into_iter()
    .flatten()
    .collect();
    let mut sidecar_hashes: BTreeMap<String, String> = BTreeMap::new();

    for sc in sidecar_paths {
        let sc_name = sc
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("sidecar")
            .to_string();
        let dest_pristine = pristine_dir.join(&sc_name);
        let dest_working = working_dir.join(&sc_name);
        tokio::fs::copy(&sc, &dest_pristine)
            .await
            .map_err(|e| AppError::new(AppErrorKind::Io, "COPY_FAILED", format!("{e}")))?;
        tokio::fs::copy(&dest_pristine, &dest_working)
            .await
            .map_err(|e| AppError::new(AppErrorKind::Io, "COPY_FAILED", format!("{e}")))?;
        let p_hashing_sidecar = progress.clone();
        let h = hasher::hash_file_streaming(&dest_pristine, cancel.clone(), move |hp| {
            p_hashing_sidecar(IngestStep::HashingSidecar, hp)
        })
        .await?;
        sidecar_hashes.insert(sc_name, h);
    }

    Ok(IngestReport {
        evidence_id: evidence_id.to_string(),
        filename,
        original_path: original.display().to_string().replace('\\', "/"),
        original_size,
        original_sha256: original_sha,
        pristine_sha256: pristine_sha,
        working_sha256: working_sha,
        sidecar_hashes,
        has_wal: sidecars.wal.is_some(),
        has_shm: sidecars.shm.is_some(),
        has_journal: sidecars.journal.is_some(),
    })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestStep {
    HashingOriginal,
    HashingPristine,
    HashingWorking,
    HashingSidecar,
}
