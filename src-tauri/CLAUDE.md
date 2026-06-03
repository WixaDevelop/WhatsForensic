# CLAUDE.md — Backend Rust (src-tauri)

Convenciones específicas para el código Rust del proyecto. Las reglas generales y forenses están en el `CLAUDE.md` del root.

-----

## Convención de idioma

- **Código:** inglés. Nombres de tipos, funciones, variables, módulos, parámetros, errores internos.
- **Comentarios `//`:** inglés cuando explican mecánica de código.
- **Doc comments `///`:** español cuando documentan API pública del módulo (porque el equipo trabaja en español).
- **Mensajes de `AppError.message`:** español (los ve el usuario en el frontend).
- **Logs de `tracing`:** inglés (son técnicos, no van al usuario).
- **Strings de audit log en campos `action_description`:** español (van al reporte que ve el investigador).

Ejemplo:

```rust
/// Calcula el hash SHA-256 de un archivo en streaming, con soporte de cancelación.
///
/// Devuelve un error si el archivo no se puede leer o si la operación es cancelada.
pub async fn hash_file_streaming(
    path: &Path,
    cancel: CancellationToken,
    progress: Sender<HashProgress>,
) -> Result<FileHash, HashError> {
    // Open file in read-only mode. Never modify source bytes.
    let file = File::open(path).await?;
    tracing::debug!(path = %path.display(), "starting hash");
    // ...
}
```

-----

## Errores

- Cada módulo define su propio enum de error con `thiserror`. Ejemplo: `HashError`, `IngestError`, `ParserError`.
- `anyhow` se usa solo en la capa `commands/` para colapsar errores de módulos múltiples antes de convertir a `AppError`.
- Nunca `panic!`, `unwrap()`, ni `expect()` en código de producción. Solo permitidos en tests y en código `#[cfg(test)]`.
- Conversión final a `AppError` se hace en `commands/` con un `From<E> for AppError` por cada error de módulo.
- `AppError` tiene campos estables: `kind`, `code`, `message`, `context`. El frontend mapea por `code`, no por `message`.

```rust
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("source file not found: {path}")]
    SourceNotFound { path: PathBuf },

    #[error("hash mismatch after copy: original={original} copy={copy}")]
    HashMismatch { original: String, copy: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
```

-----

## Logging y auditoría

Dos canales separados, **nunca mezclar**:

### `tracing` (application logging)

- Para debug del developer.
- Niveles: `trace`, `debug`, `info`, `warn`, `error`.
- Va a `<app_data>/logs/` con rotación diaria.
- No incluir contenido de evidencia (mensajes, texto de filas).
- Sí incluir paths, hashes, evidence_ids, duraciones.

```rust
tracing::info!(evidence_id = %id, mode = ?mode, "opened database");
tracing::warn!(table = %name, "expected column missing");
```

### Audit log del caso

- Vive en `<workspace>/<case_id>/audit.log`.
- JSON Lines, append-only, encadenado por hash.
- Registra acciones que tocan la evidencia: ingesta, apertura, parseo, exportación.
- Llamadas explícitas vía `workspace::audit_log::record(...)`.
- Nunca llamar `tracing` y audit log con el mismo propósito: son canales distintos.

-----

## Acceso SQLite (recordatorio crítico)

Toda apertura pasa exclusivamente por `db::opener`. Nunca llamar `Connection::open` directamente desde fuera de ese módulo.

```rust
// Solo committed (sin WAL):
let conn = db::opener::open_committed_only(&working_path)?;

// Incluye WAL (puede modificar working):
let conn = db::opener::open_with_wal(&working_path)?;
```

Después de cualquier `open_with_wal`, llamar `db::opener::rehash_after_close(...)` antes de continuar.

Toda query pasa por `db::safe_query::run(...)` que valida que el statement empiece por SELECT o PRAGMA.

-----

## Async y cancelación

- Tauri usa Tokio. Tareas pesadas con `tokio::spawn`.
- Funciones largas reciben `CancellationToken` de `tokio_util::sync`.
- Revisar `cancel.is_cancelled()` periódicamente, al menos en cada iteración de chunk.
- Progreso reportado por `tokio::sync::mpsc::Sender<Progress>` o por Channel de Tauri.
- No bloquear el hilo principal de Tauri con operaciones de I/O sync. Usar `tokio::fs` o `tokio::task::spawn_blocking` cuando sea necesario.

-----

## Reproducibilidad

- `BTreeMap` en lugar de `HashMap` cuando el resultado se serializa o exporta.
- Ordenar `Vec` explícitamente antes de exportar: `sort_by_key(|m| (m.timestamp_utc, m.source_pk))`.
- No usar `SystemTime::now()` para datos del reporte excepto en marca de generación.
- Versión de herramienta embebida en cada export: leer de `env!("CARGO_PKG_VERSION")` + hash de build si está disponible.

-----

## Tests

- `tests/` para tests de integración.
- `#[cfg(test)] mod tests` dentro de cada módulo para unit tests.
- Tests usan **solo** fixtures sintéticos generados en `tests/fixtures/`.
- Nunca commitear bases reales como fixtures.
- Tests de reproducibilidad: ejecutar parser dos veces sobre la misma fixture, hashear output, verificar igualdad bit-identical.

-----

## Dependencias

- `Cargo.lock` versionado.
- Pinear versiones por mayor en `Cargo.toml`: `tauri = "2"`, `rusqlite = "0.32"`, etc. Permitir patches automáticos.
- Antes de agregar una dependencia nueva, verificar:
  - Mantenida activamente (commits recientes).
  - Sin advisories abiertos en `cargo audit`.
  - Compatible con la licencia del proyecto.
- Crates clave actuales (verificar versión exacta al inicializar):
  - `tauri` (línea 2.x)
  - `rusqlite` con feature `bundled`
  - `sha2`
  - `rust_xlsxwriter`
  - `serde`, `serde_json`
  - `thiserror`
  - `anyhow`
  - `uuid` con feature `v4`
  - `chrono` con feature `serde`
  - `chrono-tz`
  - `tracing`, `tracing-subscriber`, `tracing-appender`
  - `tokio` (viene con Tauri)
  - `tokio-util` para `CancellationToken`
  - `walkdir`
  - `toml` para leer mapeos externalizados

-----

## Comandos Tauri

- Cada comando vive en `commands/<scope>_cmd.rs`.
- Firma estándar:

```rust
#[tauri::command]
pub async fn evidence_ingest(
    state: tauri::State<'_, AppState>,
    path: String,
    declared_type: Option<String>,
) -> Result<EvidenceId, AppError> {
    // ...
}
```

- Cada comando registrado en `main.rs` debe tener su capability declarada en `capabilities/default.json`.
- Validar inputs en el comando antes de pasar a las capas internas.
- No exponer tipos internos: convertir a DTOs serializables.

-----

## Convenciones de estilo

- `cargo fmt` obligatorio antes de commit.
- `cargo clippy -- -D warnings` debe pasar en CI.
- Doc comments en módulos públicos.
- Funciones públicas con tests asociados.
- Naming: `snake_case` para funciones y variables, `PascalCase` para tipos, `SCREAMING_SNAKE` para constantes.
