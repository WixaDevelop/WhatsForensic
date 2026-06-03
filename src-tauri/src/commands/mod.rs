//! Capa Tauri delgada. Cada comando vive en su propio archivo `*_cmd.rs`.
//!
//! Reglas (ver `src-tauri/CLAUDE.md`):
//! - Validar inputs antes de bajar a las capas internas.
//! - No exponer tipos internos: convertir a DTOs serializables.
//! - Todos los retornos son `Result<T, AppError>`.
//! - Toda llamada a `tracing::*` aquí es para debug del developer; las acciones
//!   sobre evidencia se registran adicionalmente en `audit.log`.

pub mod analysis_cmd;
pub mod case_cmd;
pub mod evidence_cmd;
pub mod export_cmd;
pub mod system_cmd;
