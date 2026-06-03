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

use crate::error::AppError;
use crate::parsers::traits::Parser;

/// Construye y devuelve los parsers disponibles. Cada uno auto-carga su
/// mapping seed desde `resources/schemas/`.
pub fn all_parsers() -> Result<Vec<Box<dyn Parser>>, AppError> {
    Ok(vec![
        Box::new(whatsapp_ios::WhatsAppIos::new()?),
        Box::new(whatsapp_android::WhatsAppAndroid::new()?),
        Box::new(callhistory_ios::CallHistoryIos::new()?),
    ])
}

pub fn by_key(key: &str) -> Result<Box<dyn Parser>, AppError> {
    match key {
        "whatsapp_ios" => Ok(Box::new(whatsapp_ios::WhatsAppIos::new()?)),
        "whatsapp_android" => Ok(Box::new(whatsapp_android::WhatsAppAndroid::new()?)),
        "callhistory_ios" => Ok(Box::new(callhistory_ios::CallHistoryIos::new()?)),
        _ => Err(AppError::invalid_input(
            "UNKNOWN_PARSER",
            format!("Parser desconocido: {key}"),
        )),
    }
}
