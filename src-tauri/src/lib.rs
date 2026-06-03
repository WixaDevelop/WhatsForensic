//! WhatsForensics — forensic SQLite analyzer.
//!
//! Esta biblioteca expone el punto de entrada `run()` que monta la aplicación Tauri
//! y registra los comandos disponibles para el frontend. La separación bin/lib es
//! requerida por la plantilla oficial de Tauri v2 (ver `main.rs`).
//!
//! Las reglas operativas y forenses están en `CLAUDE.md` (root) y el diseño en
//! `docs/ARCHITECTURE.md`.

pub mod analysis;
pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod evidence;
pub mod parsers;
pub mod report;
pub mod workspace;

use crate::config::AppState;

/// Inicializa logging estructurado con `tracing` y arranca la aplicación Tauri.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "starting WhatsForensics"
    );

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::system_cmd::system_info,
            commands::system_cmd::progress_demo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Configura `tracing` con filtro por env var `WF_LOG` (default: `info`).
///
/// En esta fase solo escribe a stdout. En fase 1 añadiremos `tracing-appender`
/// con rotación diaria sobre el directorio de logs por plataforma.
fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_env("WF_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_thread_ids(false))
        .init();
}
