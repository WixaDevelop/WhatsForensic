//! Tipo de error serializable al frontend.
//!
//! El backend nunca propaga panics ni stack traces al frontend. Todos los errores
//! que cruzan el borde IPC se mapean a `AppError`, que tiene campos estables
//! pensados para que el frontend rute por `code` (no por `message`).
//!
//! Convención:
//! - `kind`: categoría amplia para decidir presentación (modal vs snackbar, etc.).
//! - `code`: identificador estable, en `SCREAMING_SNAKE_CASE`. No cambia entre versiones.
//! - `message`: texto en español, apto para mostrar al usuario.
//! - `context`: datos adicionales serializables (paths, IDs, hashes — nunca contenido de evidencia).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Categoría amplia del error. Permite al frontend decidir cómo presentarlo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorKind {
    /// Error de entrada/salida sobre el filesystem o la red.
    Io,
    /// El esquema esperado no coincide con el observado.
    SchemaMismatch,
    /// Falla de integridad (hash, header inválido, etc.). Bloqueante.
    Integrity,
    /// El comando carece de permisos para operar sobre el recurso.
    Permission,
    /// El input del usuario no es válido.
    InvalidInput,
    /// Error interno no clasificado. Reportar como bug.
    Internal,
}

/// Error serializable al frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub kind: AppErrorKind,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, serde_json::Value>,
}

impl AppError {
    pub fn new(kind: AppErrorKind, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: code.into(),
            message: message.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    pub fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AppErrorKind::Internal, code, message)
    }

    pub fn invalid_input(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(AppErrorKind::InvalidInput, code, message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}/{}] {}",
            kind_str(self.kind),
            self.code,
            self.message
        )
    }
}

impl std::error::Error for AppError {}

/// Conversión genérica desde `anyhow::Error` para colapsar errores en la capa de comandos.
/// El mensaje se vuelve español genérico; el detalle real va al tracing.
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!(error = %err, "unhandled error reached command boundary");
        AppError::internal("INTERNAL_ERROR", "Error interno. Consulte los logs.")
    }
}

fn kind_str(k: AppErrorKind) -> &'static str {
    match k {
        AppErrorKind::Io => "io",
        AppErrorKind::SchemaMismatch => "schema_mismatch",
        AppErrorKind::Integrity => "integrity",
        AppErrorKind::Permission => "permission",
        AppErrorKind::InvalidInput => "invalid_input",
        AppErrorKind::Internal => "internal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_with_snake_case_kind() {
        let e = AppError::new(
            AppErrorKind::SchemaMismatch,
            "MISSING_COLUMN",
            "Falta columna X",
        )
        .with_context("table", serde_json::json!("ZWAMESSAGE"));
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"kind\":\"schema_mismatch\""));
        assert!(json.contains("\"code\":\"MISSING_COLUMN\""));
        assert!(json.contains("\"context\""));
    }

    #[test]
    fn context_omitted_when_empty() {
        let e = AppError::internal("X", "msg");
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("\"context\""));
    }
}
