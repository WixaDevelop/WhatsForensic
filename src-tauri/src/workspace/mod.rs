//! Capa `workspace` — gestión del directorio de caso.
//!
//! Un caso es un directorio con `case.json` (manifest), `audit.log`
//! (encadenado por hash, append-only), `evidence/<id>/` (pristine + working +
//! hashes), `exports/`, `tool_version.txt`. Esta capa garantiza:
//!
//! - Layout consistente entre OS.
//! - Acceso al manifest con file locking (`fs2`) para impedir dos instancias
//!   sobre el mismo caso.
//! - Audit log append-only que encadena por hash cada entrada con la anterior.

pub mod audit_log;
pub mod layout;
pub mod manifest;
