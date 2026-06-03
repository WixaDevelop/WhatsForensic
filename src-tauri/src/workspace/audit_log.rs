//! Audit log: JSON Lines, append-only, encadenado por hash SHA-256.
//!
//! Cada entrada lleva `prev_hash` (hash de la línea anterior) y `line_hash`
//! (hash de esta línea sin el campo `line_hash`). Documentado como
//! **trazabilidad técnica**, **no** firma criptográfica.
//!
//! Garantías de cross-platform:
//! - LF (`\n`) forzado entre líneas, nunca CRLF.
//! - Timestamps en UTC ISO-8601 con sub-segundo (`chrono::DateTime<Utc>::to_rfc3339`).
//! - El hash NO incluye `ts_monotonic` ni `line_hash` (este último sería recursivo).

use crate::error::{AppError, AppErrorKind};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub seq: u64,
    pub ts_utc: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub params: serde_json::Value,
    pub result: serde_json::Value,
    pub prev_hash: String,
    pub line_hash: String,
}

/// Lee la última entrada del audit log y devuelve `(prev_hash, next_seq)`.
/// Si el archivo no existe, devuelve `("", 0)`.
fn last_state(audit_log_path: &Path) -> Result<(String, u64), AppError> {
    if !audit_log_path.exists() {
        return Ok((String::new(), 0));
    }
    let file = std::fs::File::open(audit_log_path)
        .map_err(|e| AppError::new(AppErrorKind::Io, "READ_AUDIT_FAILED", format!("{e}")))?;
    let reader = BufReader::new(file);
    let mut last_line: Option<String> = None;
    for line in reader.lines() {
        let line =
            line.map_err(|e| AppError::new(AppErrorKind::Io, "READ_AUDIT_FAILED", format!("{e}")))?;
        if !line.trim().is_empty() {
            last_line = Some(line);
        }
    }
    match last_line {
        Some(s) => {
            let entry: AuditEntry = serde_json::from_str(&s).map_err(|e| {
                AppError::new(
                    AppErrorKind::Integrity,
                    "AUDIT_PARSE_FAILED",
                    format!("Última línea del audit log corrupta: {e}"),
                )
            })?;
            Ok((entry.line_hash, entry.seq + 1))
        }
        None => Ok((String::new(), 0)),
    }
}

/// Construye una entrada y la appendea al audit log con LF.
pub fn record(
    audit_log_path: &Path,
    action: impl AsRef<str>,
    params: serde_json::Value,
    result: serde_json::Value,
) -> Result<AuditEntry, AppError> {
    if let Some(parent) = audit_log_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::new(AppErrorKind::Io, "MKDIR_FAILED", format!("{e}")))?;
    }

    let (prev_hash, seq) = last_state(audit_log_path)?;
    let actor = whoami::username();
    let ts_utc = Utc::now();
    let action_str = action.as_ref().to_string();

    // Construir el valor a hashear con orden de campos determinístico.
    // Usamos serde_json::Map (que respeta insertion order) para asegurar
    // que la serialización sea estable.
    let mut hash_input = serde_json::Map::new();
    hash_input.insert("seq".into(), json!(seq));
    hash_input.insert("tsUtc".into(), json!(ts_utc));
    hash_input.insert("actor".into(), json!(actor));
    hash_input.insert("action".into(), json!(action_str));
    hash_input.insert("params".into(), params.clone());
    hash_input.insert("result".into(), result.clone());
    hash_input.insert("prevHash".into(), json!(prev_hash));
    let serialized_for_hash = serde_json::to_string(&serde_json::Value::Object(hash_input))
        .map_err(|e| AppError::new(AppErrorKind::Internal, "SERIALIZE_FAILED", format!("{e}")))?;
    let mut h = Sha256::new();
    h.update(serialized_for_hash.as_bytes());
    let line_hash = hex::encode(h.finalize());

    let entry = AuditEntry {
        seq,
        ts_utc,
        actor,
        action: action_str,
        params,
        result,
        prev_hash,
        line_hash,
    };

    let line = serde_json::to_string(&entry)
        .map_err(|e| AppError::new(AppErrorKind::Internal, "SERIALIZE_FAILED", format!("{e}")))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(audit_log_path)
        .map_err(|e| AppError::new(AppErrorKind::Io, "OPEN_AUDIT_FAILED", format!("{e}")))?;
    file.write_all(line.as_bytes())
        .map_err(|e| AppError::new(AppErrorKind::Io, "WRITE_FAILED", format!("{e}")))?;
    file.write_all(b"\n")
        .map_err(|e| AppError::new(AppErrorKind::Io, "WRITE_FAILED", format!("{e}")))?;

    Ok(entry)
}

/// Lee todas las entradas del audit log (en orden cronológico).
pub fn read_all(audit_log_path: &Path) -> Result<Vec<AuditEntry>, AppError> {
    if !audit_log_path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(audit_log_path)
        .map_err(|e| AppError::new(AppErrorKind::Io, "READ_AUDIT_FAILED", format!("{e}")))?;
    let mut out = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: AuditEntry = serde_json::from_str(line).map_err(|e| {
            AppError::new(
                AppErrorKind::Integrity,
                "AUDIT_PARSE_FAILED",
                format!("Entrada del audit log corrupta: {e}"),
            )
        })?;
        out.push(entry);
    }
    Ok(out)
}

/// Verifica la cadena de hashes. Devuelve la primera entrada cuya cadena
/// es inconsistente, o `None` si toda la cadena es consistente.
pub fn verify_chain(entries: &[AuditEntry]) -> Option<u64> {
    let mut expected_prev = String::new();
    for entry in entries {
        if entry.prev_hash != expected_prev {
            return Some(entry.seq);
        }
        // Re-hash this entry to check line_hash.
        let mut hash_input = serde_json::Map::new();
        hash_input.insert("seq".into(), json!(entry.seq));
        hash_input.insert("tsUtc".into(), json!(entry.ts_utc));
        hash_input.insert("actor".into(), json!(entry.actor));
        hash_input.insert("action".into(), json!(entry.action));
        hash_input.insert("params".into(), entry.params.clone());
        hash_input.insert("result".into(), entry.result.clone());
        hash_input.insert("prevHash".into(), json!(entry.prev_hash));
        let serialized = serde_json::to_string(&serde_json::Value::Object(hash_input)).unwrap();
        let mut h = Sha256::new();
        h.update(serialized.as_bytes());
        let computed = hex::encode(h.finalize());
        if computed != entry.line_hash {
            return Some(entry.seq);
        }
        expected_prev = entry.line_hash.clone();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_chains_entries() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path();

        let e1 = record(
            path,
            "case_create",
            json!({"name": "case1"}),
            json!({"ok": true}),
        )
        .unwrap();
        assert_eq!(e1.seq, 0);
        assert_eq!(e1.prev_hash, "");

        let e2 = record(
            path,
            "evidence_ingest",
            json!({"file": "x.db"}),
            json!({"ok": true}),
        )
        .unwrap();
        assert_eq!(e2.seq, 1);
        assert_eq!(e2.prev_hash, e1.line_hash);

        let entries = read_all(path).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(verify_chain(&entries).is_none(), "chain should verify");
    }

    #[test]
    fn detects_tampering() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path();

        record(path, "a", json!({}), json!({})).unwrap();
        record(path, "b", json!({}), json!({})).unwrap();

        // Tamper: rewrite an entry with a different action.
        let mut entries = read_all(path).unwrap();
        entries[1].action = "TAMPERED".to_string();
        // Without recomputing line_hash, verify_chain should detect this.
        assert!(verify_chain(&entries).is_some());
    }
}
