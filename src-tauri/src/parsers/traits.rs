//! Trait `Parser` que cumple cada fuente conocida.
//!
//! TODO fase 3:
//! ```ignore
//! trait Parser {
//!     fn detect(path: &Path, schema: &SchemaSnapshot) -> Confidence;
//!     fn schema_expected() -> ExpectedSchema;
//!     fn parse(conn: &Connection, schema_version: &str) -> Result<ParsedEvidence, ParserError>;
//! }
//! ```
//! Cada parser **debe** validar columnas esperadas y emitir warnings explícitos
//! cuando falten. Nunca falla silencioso.
