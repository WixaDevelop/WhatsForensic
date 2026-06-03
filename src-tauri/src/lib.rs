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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::system_cmd::system_info,
            commands::system_cmd::progress_demo,
            commands::case_cmd::case_create,
            commands::case_cmd::case_open,
            commands::case_cmd::case_close,
            commands::case_cmd::case_get_current,
            commands::evidence_cmd::evidence_preview,
            commands::evidence_cmd::evidence_ingest,
            commands::evidence_cmd::evidence_list,
            commands::evidence_cmd::evidence_verify,
            commands::evidence_cmd::task_cancel,
            commands::evidence_cmd::default_workspace_root,
            commands::db_cmd::evidence_introspect,
            commands::analysis_cmd::analysis_list_parsers,
            commands::analysis_cmd::analysis_detect_parsers,
            commands::analysis_cmd::analysis_run,
            commands::analysis_cmd::analysis_query_messages,
            commands::analysis_cmd::analysis_query_calls,
            commands::analysis_cmd::analysis_query_conversations,
            commands::analysis_cmd::analysis_get_warnings,
            commands::analysis_cmd::analysis_list_runs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Configura `tracing` con filtro por env var `WF_LOG` (default: `info`).
fn init_tracing() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_env("WF_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_thread_ids(false))
        .init();
}
