//! Comandos de sistema: verificación de salud y demos.
//!
//! `system_info` retorna metadatos de build útiles para diagnóstico y para
//! verificar la conectividad IPC.
//!
//! `progress_demo` ejemplifica el patrón de streaming de progreso usando
//! `tauri::ipc::Channel`. Este patrón se reutilizará en fase 1 para hashing,
//! en fase 3+ para parseo y en fase 6 para exportación.

use crate::error::{AppError, AppErrorKind};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

/// Información estática de la build, expuesta al frontend para el panel "Acerca de"
/// y para incluir en cada reporte XLSX (campo de reproducibilidad).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub tool_name: String,
    pub tool_version: String,
    pub rust_edition: String,
    pub target_os: String,
    pub target_arch: String,
}

#[tauri::command]
pub fn system_info() -> Result<SystemInfo, AppError> {
    Ok(SystemInfo {
        tool_name: "WhatsForensics".to_string(),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        rust_edition: "2021".to_string(),
        target_os: std::env::consts::OS.to_string(),
        target_arch: std::env::consts::ARCH.to_string(),
    })
}

/// Evento de progreso emitido por `progress_demo` a través del Channel.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub phase: String,
    pub percent: u8,
}

/// Demo del patrón Channel: emite 10 eventos de progreso espaciados por 100 ms.
///
/// El frontend pasa un Channel a este comando; el backend lo usa para hacer push
/// de eventos sin esperar a que termine el comando. Es el mismo patrón que
/// usaremos para hashing de evidencia, parseo y exportación.
#[tauri::command]
pub async fn progress_demo(on_event: Channel<ProgressEvent>) -> Result<(), AppError> {
    tracing::info!("progress_demo started");
    for i in 0..=10u8 {
        let evt = ProgressEvent {
            phase: "demo".to_string(),
            percent: i * 10,
        };
        on_event.send(evt).map_err(|e| {
            tracing::error!(error = %e, "failed to send progress event");
            AppError::new(
                AppErrorKind::Internal,
                "CHANNEL_SEND_FAILED",
                "No se pudo enviar progreso al frontend.",
            )
        })?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    tracing::info!("progress_demo completed");
    Ok(())
}
