//! Trait `Parser` que cumple cada fuente conocida.

use crate::db::introspect::SchemaSnapshot;
use crate::error::AppError;
use crate::parsers::common_model::ParsedEvidence;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// No coincide.
    None,
    /// Algunas señales coinciden, otras no.
    Low,
    /// Coincide la mayoría de los esperados.
    Medium,
    /// Coincide todo lo esperado.
    High,
}

pub trait Parser: Send + Sync {
    /// Identificador estable. Va al manifest.
    fn key(&self) -> &'static str;

    /// Nombre legible para humanos.
    fn display_name(&self) -> &'static str;

    /// Detecta cuán probable es que este parser aplique al schema.
    fn detect(&self, filename: &str, schema: &SchemaSnapshot) -> Confidence;

    /// Parsea la base. La conexión ya fue abierta por `db::opener` con el
    /// modo decidido por el usuario.
    fn parse(&self, conn: &Connection) -> Result<ParsedEvidence, AppError>;
}
