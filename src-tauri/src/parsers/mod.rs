//! Capa `parsers` — un módulo por fuente conocida.
//!
//! Cada parser declara su esquema esperado y produce un `ParsedEvidence` con
//! el modelo común (ver [`common_model`]). Los mapeos código→significado
//! viven en `src-tauri/resources/schemas/<source>/<version>.toml`, **nunca**
//! en código.

pub mod callhistory_ios;
pub mod common_model;
pub mod source_detect;
pub mod timestamps;
pub mod traits;
pub mod whatsapp_android;
pub mod whatsapp_ios;
