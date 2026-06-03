//! Estado compartido de la aplicación.
//!
//! `AppState` se monta con `tauri::Builder::manage` y se inyecta en cada comando
//! vía `tauri::State<AppState>`. En esta fase está vacío; en fase 1 incorpora el
//! caso abierto actualmente, el workspace_root, y los handles de tareas en curso.

use std::path::PathBuf;
use std::sync::RwLock;

/// Estado compartido entre comandos.
///
/// Concurrencia: lecturas frecuentes, escrituras puntuales (abrir/cerrar caso),
/// por eso `RwLock` y no `Mutex`. Cualquier estado mutable nuevo debe quedar
/// encapsulado detrás de un método del módulo `commands`, no expuesto directo.
#[derive(Default)]
pub struct AppState {
    pub workspace_root: RwLock<Option<PathBuf>>,
    // TODO fase 1: `current_case: RwLock<Option<CaseHandle>>`.
    // TODO fase 1: registry de tareas async cancelables.
}
