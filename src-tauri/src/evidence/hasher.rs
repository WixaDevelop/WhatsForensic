//! Hashing SHA-256 en streaming, cancelable, con reporte de progreso.
//!
//! La función `hash_file_streaming` lee el archivo en chunks de 1 MiB, actualiza
//! un `Sha256` incrementalmente y reporta progreso al callback solo cuando cambia
//! el porcentaje entero (throttle para evitar inundar el Channel).

use crate::error::{AppError, AppErrorKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::io::{AsyncReadExt, BufReader};
use tokio_util::sync::CancellationToken;

const CHUNK_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashProgress {
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub percent: u8,
}

/// Calcula el SHA-256 de un archivo en streaming.
///
/// Devuelve el hash como string hex en minúsculas (64 chars).
///
/// Cancela limpiamente si `cancel.is_cancelled()`. El progreso solo se emite
/// cuando cambia el porcentaje entero (máx ~101 eventos por archivo).
pub async fn hash_file_streaming<F>(
    path: &Path,
    cancel: CancellationToken,
    progress: F,
) -> Result<String, AppError>
where
    F: Fn(HashProgress),
{
    let file = tokio::fs::File::open(path).await.map_err(|e| {
        AppError::new(
            AppErrorKind::Io,
            "OPEN_FAILED",
            format!("No se pudo abrir el archivo para hashing: {e}"),
        )
        .with_context("path", serde_json::json!(path.display().to_string()))
    })?;

    let total = file.metadata().await.map(|m| m.len()).unwrap_or(0);
    let mut reader = BufReader::with_capacity(CHUNK_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut bytes_done: u64 = 0;
    let mut last_pct: i16 = -1;

    progress(HashProgress {
        bytes_done: 0,
        bytes_total: total,
        percent: 0,
    });

    loop {
        if cancel.is_cancelled() {
            return Err(AppError::new(
                AppErrorKind::Internal,
                "CANCELED",
                "Operación de hashing cancelada por el usuario.",
            ));
        }

        let n = reader.read(&mut buf).await.map_err(|e| {
            AppError::new(
                AppErrorKind::Io,
                "READ_FAILED",
                format!("Error leyendo el archivo: {e}"),
            )
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        bytes_done += n as u64;

        let pct = bytes_done
            .saturating_mul(100)
            .checked_div(total)
            .unwrap_or(0)
            .min(100) as i16;
        if pct != last_pct {
            progress(HashProgress {
                bytes_done,
                bytes_total: total,
                percent: pct as u8,
            });
            last_pct = pct;
        }
    }

    progress(HashProgress {
        bytes_done,
        bytes_total: total,
        percent: 100,
    });

    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hashes_empty_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let hash = hash_file_streaming(tmp.path(), CancellationToken::new(), |_| {})
            .await
            .unwrap();
        // SHA-256 of empty input.
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[tokio::test]
    async fn hashes_known_content() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut tmp, b"abc").unwrap();
        let hash = hash_file_streaming(tmp.path(), CancellationToken::new(), |_| {})
            .await
            .unwrap();
        assert_eq!(
            hash,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[tokio::test]
    async fn reports_progress_at_least_once() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut tmp, b"hello world").unwrap();
        let count = std::sync::atomic::AtomicUsize::new(0);
        hash_file_streaming(tmp.path(), CancellationToken::new(), |_| {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
        .await
        .unwrap();
        assert!(count.load(std::sync::atomic::Ordering::SeqCst) >= 2); // start + end
    }

    #[tokio::test]
    async fn honors_cancellation() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut tmp, &vec![0u8; 4 * CHUNK_SIZE]).unwrap();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let result = hash_file_streaming(tmp.path(), cancel, |_| {}).await;
        assert!(result.is_err());
        assert_eq!(result.err().unwrap().code, "CANCELED");
    }
}
