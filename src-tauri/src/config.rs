//! Estado compartido de la aplicación.
//!
//! `AppState` se monta con `tauri::Builder::manage` y se inyecta en cada comando
//! vía `tauri::State<AppState>`. Mantiene el caso actualmente abierto, su lock
//! exclusivo y los handles de tareas async cancelables.

use crate::workspace::{layout::CasePaths, manifest::CaseManifest, manifest::LockHandle};
use std::collections::HashMap;
use std::sync::RwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

/// Caso actualmente abierto. El lock se mantiene vivo durante toda la sesión.
pub struct OpenCase {
    pub manifest: CaseManifest,
    pub paths: CasePaths,
    pub _lock: LockHandle,
}

/// Estado compartido entre comandos.
///
/// Concurrencia: lecturas frecuentes, escrituras puntuales (abrir/cerrar caso,
/// registrar evidencia). Usamos `RwLock` para permitir varios readers en
/// paralelo. Las tareas async cancelables van en `tasks` con `Mutex`.
#[derive(Default)]
pub struct AppState {
    pub current_case: RwLock<Option<OpenCase>>,
    pub tasks: RwLock<HashMap<String, CancellationToken>>,
}

impl AppState {
    /// Registra un token de cancelación nuevo y devuelve un `run_id` para que
    /// el frontend pueda cancelar la tarea.
    pub fn register_task(&self, token: CancellationToken) -> String {
        let run_id = Uuid::new_v4().to_string();
        self.tasks.write().unwrap().insert(run_id.clone(), token);
        run_id
    }

    pub fn cancel_task(&self, run_id: &str) -> bool {
        if let Some(token) = self.tasks.write().unwrap().remove(run_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub fn drop_task(&self, run_id: &str) {
        self.tasks.write().unwrap().remove(run_id);
    }
}
