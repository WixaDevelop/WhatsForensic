//! Whitelist de statements permitidos sobre la DB working.
//!
//! Solo aceptamos `SELECT`, `PRAGMA` y `WITH` (CTE). Cualquier otro statement
//! retorna `permission/SQL_NOT_ALLOWED`. Múltiples statements en la misma
//! string (separados por `;`) también son rechazados.

use crate::error::{AppError, AppErrorKind};

const ALLOWED_PREFIXES: &[&str] = &["SELECT", "PRAGMA", "WITH"];

/// Valida que el SQL provisto sea aceptado por la política forense.
pub fn validate(sql: &str) -> Result<(), AppError> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            AppErrorKind::InvalidInput,
            "EMPTY_SQL",
            "El SQL provisto está vacío.",
        ));
    }
    let first_word: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_uppercase();
    if !ALLOWED_PREFIXES.contains(&first_word.as_str()) {
        return Err(AppError::new(
            AppErrorKind::Permission,
            "SQL_NOT_ALLOWED",
            "Solo se permiten consultas SELECT, PRAGMA o WITH (CTE).",
        )
        .with_context("first_word", serde_json::json!(first_word)));
    }
    // Reject multiple statements (any `;` that is not just a trailing one).
    let core = trimmed.trim_end_matches(';');
    if core.contains(';') {
        return Err(AppError::new(
            AppErrorKind::Permission,
            "MULTIPLE_STATEMENTS",
            "No se permiten múltiples statements en la misma consulta.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_select() {
        assert!(validate("SELECT * FROM t").is_ok());
        assert!(validate("  select 1  ").is_ok());
    }

    #[test]
    fn accepts_pragma() {
        assert!(validate("PRAGMA table_info(x)").is_ok());
    }

    #[test]
    fn accepts_with_cte() {
        assert!(validate("WITH x AS (SELECT 1) SELECT * FROM x").is_ok());
    }

    #[test]
    fn rejects_insert() {
        assert!(validate("INSERT INTO t VALUES (1)").is_err());
    }

    #[test]
    fn rejects_update() {
        assert!(validate("UPDATE t SET a=1").is_err());
    }

    #[test]
    fn rejects_drop() {
        assert!(validate("DROP TABLE t").is_err());
    }

    #[test]
    fn rejects_multiple_statements() {
        let r = validate("SELECT 1; SELECT 2");
        assert!(r.is_err());
        assert_eq!(r.err().unwrap().code, "MULTIPLE_STATEMENTS");
    }

    #[test]
    fn allows_trailing_semicolon() {
        assert!(validate("SELECT 1;").is_ok());
    }
}
