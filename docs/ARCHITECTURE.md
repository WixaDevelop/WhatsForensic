# Arquitectura — Forensic SQLite Analyzer

Documento de diseño detallado. Para reglas operativas y gotchas críticos ver `CLAUDE.md` en el root.

-----

## 1. Arquitectura general

Aplicación desktop monolítica. Separación frontend/backend vía IPC de Tauri v2. Backend en Rust hace todo el trabajo sensible (I/O de evidencia, hashing, parsing, exportación). Frontend React es solo presentación e interacción.

**Principio rector:** la evidencia original es de solo lectura absoluta. La aplicación nunca abre el archivo original con SQLite. Solo lo lee como bytes para hashear y copiar. Todo análisis se hace sobre una copia de trabajo dentro de un workspace de caso.

### Capas del backend Rust

1. **`evidence`** — registro, hashing, copia de archivos fuente. No conoce SQLite.
1. **`workspace`** — gestión del directorio de caso, manifest, audit log.
1. **`db`** — apertura segura de SQLite, solo sobre copias, read-only forzado.
1. **`parsers`** — un módulo por fuente conocida. Cada parser declara su esquema esperado.
1. **`analysis`** — gaps, señales de eliminados, estadísticas. Independiente de la fuente.
1. **`report`** — generación XLSX.
1. **`commands`** — capa Tauri delgada, orquesta lo anterior.

### Modelo de ejecución

Comandos largos (hashing, parsing, exportación) corren en tareas async con reporte de progreso vía Channels de Tauri v2 (más eficientes que `emit` para streaming de datos). El frontend nunca bloquea esperando resultado síncrono de operación pesada. Tareas son **cancelables** vía token compartido (`tokio_util::sync::CancellationToken`).

### Alternativas descartadas y por qué

- *Cliente-servidor con backend separado:* overkill, agrega superficie de ataque. Descartado.
- *Todo en frontend con SQL.js / WebAssembly:* pierde acceso nativo controlado al filesystem, hashing eficiente y aislamiento de Rust. Mal encaje forense.
- *`tauri-plugin-sql`:* existe pero abstrae demasiado. Para forense se necesita control fino sobre flags de apertura.

-----

## 2. Estructura de carpetas

```
forensic-tool/
├── CLAUDE.md                       # Reglas operativas (root)
├── .claude/
│   ├── settings.json               # Permission model restrictivo
│   └── commands/                   # (opcional, fases posteriores)
├── .github/workflows/              # CI
├── .gitignore                      # Incluye reglas anti-evidencia-real
│
├── src-tauri/                      # Backend Rust
│   ├── CLAUDE.md                   # Convenciones Rust
│   ├── Cargo.toml
│   ├── Cargo.lock                  # Versionado
│   ├── tauri.conf.json
│   ├── capabilities/               # Capability-based permissions (Tauri v2)
│   │   └── default.json
│   ├── resources/
│   │   └── schemas/                # Mapeos externalizados por versión
│   │       ├── whatsapp_ios/
│   │       ├── whatsapp_android/
│   │       └── callhistory_ios/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── error.rs                # AppError serializable
│       ├── config.rs
│       │
│       ├── evidence/
│       │   ├── mod.rs
│       │   ├── hasher.rs           # SHA-256 streaming, cancelable
│       │   ├── ingest.rs           # Detección DB+WAL+SHM+journal, doble copia
│       │   ├── header.rs           # Validación header SQLite, page_size
│       │   └── metadata.rs
│       │
│       ├── workspace/
│       │   ├── mod.rs
│       │   ├── layout.rs
│       │   ├── manifest.rs         # case.json con file locking
│       │   └── audit_log.rs        # Append-only, encadenado por hash
│       │
│       ├── db/
│       │   ├── mod.rs
│       │   ├── opener.rs           # Modos "solo committed" / "incluye WAL"
│       │   ├── introspect.rs
│       │   └── safe_query.rs       # Whitelist SELECT/PRAGMA
│       │
│       ├── parsers/
│       │   ├── mod.rs
│       │   ├── traits.rs           # Trait Parser
│       │   ├── source_detect.rs
│       │   ├── whatsapp_android.rs
│       │   ├── whatsapp_ios.rs
│       │   ├── callhistory_ios.rs
│       │   ├── timestamps.rs       # Conversiones Mac Absolute / Unix s/ms
│       │   └── common_model.rs
│       │
│       ├── analysis/
│       │   ├── mod.rs
│       │   ├── gaps.rs
│       │   ├── deleted_hints.rs
│       │   └── stats.rs
│       │
│       ├── report/
│       │   ├── mod.rs
│       │   ├── xlsx_writer.rs
│       │   ├── sheets/
│       │   └── charts.rs
│       │
│       └── commands/
│           ├── mod.rs
│           ├── case_cmd.rs
│           ├── evidence_cmd.rs
│           ├── analysis_cmd.rs
│           └── export_cmd.rs
│
├── src/                            # Frontend React + TS
│   ├── CLAUDE.md                   # Convenciones frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── api/
│   │   ├── tauri.ts                # Wrapper tipado de invoke()
│   │   └── events.ts               # Channels y eventos
│   ├── types/domain.ts             # Tipos espejados de Rust
│   ├── pages/                      # CaseSetup, EvidenceIngest, Analysis, Findings, Export
│   ├── components/
│   ├── state/caseStore.ts
│   └── styles/
│
├── tests/
│   └── fixtures/                   # Generadores de SQLite sintético
│
└── docs/
    ├── ARCHITECTURE.md             # Este archivo
    ├── METHODOLOGY.md
    ├── SCHEMAS.md
    └── LIMITATIONS.md
```

-----

## 3. Modelo de datos

### 3.1 Workspace en disco

Un caso es un directorio. Estructura ya descrita en `CLAUDE.md`.

**`case.json` (campos lógicos):**

- `case_id` (UUID v4)
- `name`, `description`, `investigator`
- `created_at` (UTC), `timezone` (IANA tz)
- `tool_version`, `tool_build_hash`
- `evidences[]`: id, filename, source_type, original_path, original_size, original_mtime, original_sha256, pristine_sha256, working_sha256, has_wal, has_shm, has_journal, ingested_at, analysis_mode_used

**`audit.log`:** JSON Lines, append-only. Cada línea contiene:

- `seq` (monotónico)
- `ts_utc` (wall-clock)
- `ts_monotonic` (para detectar manipulación de reloj, débilmente)
- `actor` (usuario SO)
- `action`, `params`, `result`
- `prev_hash` (SHA-256 de la línea anterior serializada)
- `line_hash` (SHA-256 de esta línea sin `line_hash`)

Encadenamiento documentado como **trazabilidad**, no anti-tamper criptográfico.

### 3.2 Modelo de dominio (común a parsers)

Cada parser produce un `ParsedEvidence`:

- **Conversation**: id interno, display_name, participants, source_table, source_pk, first_seen, last_seen.
- **Message**: id interno, conversation_id, source_pk, timestamp_utc, timestamp_raw (valor sin convertir), timestamp_raw_format (“mac_absolute” | “unix_s” | “unix_ms”), sender, direction, body, media_ref, status_flags, message_type_raw, message_type_interpreted (puede ser `Unknown(raw)`), is_possibly_deleted, is_possibly_revoked, raw_row (JSON).
- **Call**: id, timestamp_utc, timestamp_raw, peer, direction, duration_seconds, call_type_raw, call_type_interpreted, source_pk.
- **Attachment**: id, message_id, mime_hint, original_filename, size_bytes_if_known.
- **SchemaSnapshot**: tablas y columnas detectadas con tipos. Para auditoría.

**Decisión:** cada `Message` y `Call` lleva `raw_row` con la fila SQLite original serializada. Cuesta espacio, es esencial forense.

### 3.3 Tipos compartidos Rust↔TS

Fase 0–1: tipos TS escritos a mano espejados de Rust. Evaluar `ts-rs` o `tauri-specta` al final de fase 1, después de fijar las APIs.

-----

## 4. Contrato de comandos Tauri

Todos retornan `Result<T, AppError>` serializable. Frontend nunca recibe panics.

**Casos:**

- `case_create(name, investigator, timezone)` → `CaseId`
- `case_open(path)` → `CaseSummary`
- `case_close()` → `()`
- `case_get_current()` → `CaseSummary | null`

**Evidencia:**

- `evidence_preview(path)` → `EvidencePreview` *(no copia, no hashea)*
- `evidence_ingest(path, declared_type?)` → `EvidenceId` *(async con Channel de progreso)*
- `evidence_list()` → `Vec<EvidenceSummary>`
- `evidence_verify(evidence_id)` → `IntegrityReport`
- `evidence_introspect(evidence_id)` → `SchemaSnapshot`

**Análisis:**

- `analysis_run(evidence_id, options)` → `AnalysisRunId`
- `analysis_get_results(run_id)` → `AnalysisResults`
- `analysis_query_messages(filters, pagination)` → `PagedMessages`
- `analysis_query_calls(filters, pagination)` → `PagedCalls`
- `analysis_get_gaps(evidence_id)` → `GapsReport`
- `analysis_get_deleted_hints(evidence_id)` → `DeletedHintsReport`
- `analysis_cancel(run_id)` → `()`

**Exportación:**

- `export_xlsx(case_id, output_path, options)` → `ExportResult`

**Auditoría:**

- `audit_get_log(limit, offset)` → `Vec<AuditEntry>`
- `audit_export(output_path)` → `()` *(exporta audit.log + hash global como archivo separado)*

**Eventos / Channels:**

- Channel `evidence:ingest` con `{phase, bytes_done, bytes_total}`
- Channel `analysis:progress` con `{phase, percent, current_item}`
- Channel `export:progress` con `{percent, current_sheet}`
- Evento `error:occurred` con `AppError`

**Convención de errores:**

```
AppError {
  kind: "io" | "schema_mismatch" | "integrity" | "permission" | "invalid_input" | "internal",
  code: string,        // estable, para mapear en frontend
  message: string,     // texto humano en español
  context: object,     // datos adicionales
}
```

**Capability-based permissions (Tauri v2):** cada comando expuesto al frontend debe estar declarado en `src-tauri/capabilities/default.json`. No declarar capabilities globales abiertas. Filesystem scoping: limitar a `$APPDATA` + workspace_root configurable por el usuario.

-----

## 5. Estrategia de acceso SQLite seguro

Capas de defensa, en orden:

1. **Apertura solo sobre copia working dentro del workspace.** Validado por `db::opener` que rechaza paths fuera.
1. **Validación de header** antes de abrir: primeros 16 bytes deben ser `"SQLite format 3\0"`. Leer `page_size` del header (offset 16–17, big endian).
1. **Modos de apertura:**

   **Modo “solo committed”:**

   ```
   URI: file:<absolute_path>?immutable=1
   Flags: SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI | SQLITE_OPEN_NO_MUTEX
   ```

   Documentación SQLite: con `immutable=1` deshabilita locking, change detection y **no asocia WAL**. Equivale a snapshot de la DB principal.

   **Modo “incluye WAL”:**

   ```
   Flags: SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX
   PRAGMA query_only = ON;
   ```

   SQLite puede asociar y eventualmente modificar el WAL/SHM. Por eso se opera sobre `working`, no sobre `pristine`. Re-hashear working después de cerrar.
1. **`PRAGMA query_only = ON`** tras abrir, en ambos modos, como cinturón extra.
1. **Wrapper `safe_query`** que rechaza statements que no empiecen por `SELECT` o `PRAGMA` (trim + uppercase + match).
1. **Frontend nunca envía SQL.** Solo invoca comandos tipados.
1. **Cada consulta se registra en audit.log** con SQL, evidence_id, modo de apertura, duración.

**El modo de análisis usado se registra obligatoriamente** en `case.json`, `audit.log` y el reporte XLSX.

-----

## 6. Parsers — diseño común

Trait base:

```
Parser:
  - detect(path: &Path, schema: &SchemaSnapshot) -> Confidence
  - schema_expected() -> ExpectedSchema
  - parse(conn: &Connection, schema_version: &str) -> Result<ParsedEvidence>
```

**Reglas:**

- Los mapeos `código → significado` viven en `src-tauri/resources/schemas/<source>/<version>.toml`. **Nunca en código.**
- Cada parser valida la presencia de columnas esperadas. Si falta una columna esperada, emite warning explícito y produce resultado parcial marcado como tal. **Nunca falla silencioso, nunca asume.**
- Detección de fuente: combina nombre de archivo + tablas presentes + columnas clave. Sugiere parser; el usuario confirma.
- Si el usuario fuerza un parser sobre una DB que no coincide, el parser produce errores específicos por columna faltante, no abortes globales.
- Conservar `raw_row` para cada registro.
- Conservar `*_raw` además del valor interpretado para tipos, timestamps y direcciones.

### Timestamps

Detección y conversión:

- **Mac Absolute / Cocoa:** segundos desde 2001-01-01 UTC. Multiplicar por nada, sumar epoch 978307200 s para convertir a Unix.
- **Unix segundos:** rango típico para datos recientes < 2×10^9.
- **Unix milisegundos:** valores > 10^12.
- **Heurística:** si `valor > 10^12` → ms; si `valor < 10^10 && valor > 10^8` → segundos; si `valor < 10^9 && valor > 0` → posible Mac Absolute, requiere contexto de columna.

Conservar siempre `timestamp_raw` y `timestamp_raw_format`. Convertir a UTC para presentación. Aplicar zona horaria del caso para la visualización en XLSX.

-----

## 7. Flujo completo de análisis

1. **Crear caso.** Nombre, investigador, zona horaria. Se crea directorio, `case.json`, `audit.log` con entrada de creación.
1. **Vista previa de evidencia.** Backend lee metadatos del archivo + detecta hermanos (`-wal`, `-shm`, `-journal`), valida header SQLite, intenta detectar tipo. **No copia, no hashea aún.**
1. **Confirmación de ingesta.** Backend:
- Calcula SHA-256 original (streaming, cancelable).
- Copia archivo + hermanos a `pristine/`.
- Calcula SHA-256 pristine. Si difiere de original → abortar.
- Copia `pristine/` → `working/`.
- Calcula SHA-256 working.
- Registra en `case.json` y `audit.log`.
1. **Introspección.** Backend abre `working` en modo “solo committed”, lista tablas y columnas, sugiere parser. Devuelve `SchemaSnapshot`.
1. **Selección de parser y modo.** Usuario confirma parser y elige modo “solo committed” o “incluye WAL”. Se registra en `case.json`.
1. **Parseo.** Parser ejecuta consultas conocidas, mapea al modelo común, conserva `raw_row`. Resultado vive en memoria del proceso en fase 1.
1. **Análisis derivado.** Gaps, señales de eliminados, estadísticas.
1. **Exploración interactiva.** Frontend muestra resultados con filtros, paginación, inspección de `raw_row`.
1. **Exportación XLSX.** Genera archivo en `exports/`, registra en audit.log.
1. **Re-hash de working.** Después de cualquier sesión con modo “incluye WAL”, re-hashear y comparar con hash registrado. Si difiere, registrarlo como “alteración esperada por checkpoint” en audit.log (no es error).

-----

## 8. Detección de gaps

Definición operativa: discontinuidad en una secuencia que se espera contigua.

**Tipos:**

1. Gaps en PKs autoincrementales (`Z_PK`, `_id`, etc.).
1. Saltos en `sqlite_sequence` vs. máximo PK observado.
1. Discontinuidades temporales anómalas (heurística, alto ruido, **opcional**).

**Salida por gap:** tabla, rango faltante, tamaño, contexto (PKs y timestamps vecinos), `interpretation_note` con texto fijo: *“Posibles causas: borrado, transacción abortada, migración, reuso de IDs por la aplicación. No constituye prueba de borrado.”*

**No calcular** “score de probabilidad de borrado”. Reportar hechos y posibles causas.

-----

## 9. Detección de señales de eliminados/revocados

Categorías, todas con prudencia explícita y separadas en dos hojas distintas del XLSX:

### A. Revocaciones declaradas por la app

Mensajes que la propia aplicación marca como revocados (códigos de tipo específicos). **Mapeo externalizado por versión, NO hardcodear.** El parser consulta `resources/schemas/<source>/<version>.toml`.

### B. Anomalías estructurales deducidas

1. Mensajes con cuerpo vacío y metadatos presentes.
1. Inconsistencias entre tablas relacionadas (FK rotas).
1. Registros en tablas auxiliares de rastros (depende de versión, **verificar empíricamente**).
1. Mensajes presentes en WAL pero no en la DB principal (modo “incluye WAL”).

### Fuera de alcance fase 1

- Carving en freelist y unallocated space. Requiere parser propio del formato de página SQLite. Documentar en `LIMITATIONS.md` que **fase 1 no recupera datos de páginas marcadas libres**. Reservar fase 8.
- Reconstrucción de WAL frames antiguos sobrescritos.

**Cada hallazgo lleva:**

- Tipo de señal (categoría A o B)
- `evidence_strength`: `weak | moderate | strong` con criterios documentados en METHODOLOGY.md
- Nota de prudencia fija

**Vocabulario:** “señal compatible con”, “indicio”, “hallazgo a investigar”, “requiere correlación”. Prohibido: “prueba”, “demuestra”, “confirma”.

-----

## 10. Exportación XLSX

Librería: `rust_xlsxwriter`. Sin dependencias nativas. Soporta charts, formatos, hojas múltiples.

**Hojas en orden:**

1. Portada
1. Evidencia (hashes original/pristine/working, WAL/SHM/journal presentes, modo de análisis usado)
1. Resumen ejecutivo
1. Metodología y limitaciones (texto fijo desde `METHODOLOGY.md`)
1. Conversaciones
1. Mensajes (dividir por conversación si excede umbral configurable)
1. Llamadas
1. Gaps (con columna de interpretación)
1. Revocaciones declaradas por la app
1. Anomalías estructurales
1. Estadísticas
1. Gráficos
1. Schema detectado (para auditoría técnica)
1. Audit log resumido

**Detalles obligatorios:**

- Timestamps en dos columnas: UTC y zona horaria del caso.
- Hashes en monoespaciada.
- Texto largo con wrap, truncado a N caracteres con indicador.
- `raw_row_json` opcional configurable al exportar.
- Freeze de primera fila, autofilter.
- **No incrustar multimedia** en fase 1.
- Versión de herramienta y de dependencias críticas embedidas en portada (reproducibilidad).
- Ordenar resultados explícitamente para garantizar output bit-identical sobre misma entrada.

-----

## 11. Errores, logging y auditoría

Tres canales separados, **no mezclar**:

1. **Application logging (`tracing`):** debug del developer. Niveles `trace/debug/info/warn/error`. Rotación de archivos. Va a `<app_data>/logs/`. No contiene contenido de evidencia.
1. **Audit log del caso (`audit.log`):** qué hizo el usuario/herramienta sobre la evidencia. JSON Lines encadenado. Va dentro del workspace de caso. Sí contiene referencias a evidence_ids.
1. **AppError serializable al frontend:** errores que el usuario debe ver. Tipos cerrados con códigos estables.

**Reglas:**

- `thiserror` por módulo, `anyhow` solo en capa de comandos para conversión final a `AppError`.
- Nunca panic en código de producción.
- Errores de I/O sobre evidencia → siempre en audit.log además del tracing.
- Errores de validación de hash → críticos, audit.log + bloqueo de operación.

-----

## 12. Concurrencia y cancelación

- Tareas pesadas (hashing, parsing, export) son async con `tokio::spawn`.
- Cada tarea recibe un `CancellationToken` y revisa periódicamente.
- Comandos `*_cancel(run_id)` disponibles.
- Si el usuario cierra la app durante hashing: el archivo parcial se borra, se registra en audit.log.
- Lock file en directorio de caso para impedir dos instancias abiertas sobre el mismo caso.

-----

## 13. Reproducibilidad

Para que dos ejecuciones sobre la misma entrada produzcan output bit-identical:

- Ordenar resultados explícitamente antes de exportar (PK ascendente, o timestamp ascendente con tie-break por PK).
- Usar `BTreeMap` en vez de `HashMap` para serialización.
- Embedir en cada XLSX: versión de herramienta, hash de la build, versión de dependencias críticas.
- Timestamps de generación del reporte son la única excepción permitida.

-----

## 14. CI/CD

Pipeline mínimo antes de merge a main:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo audit` (vulnerabilidades en deps)
- `npm run typecheck`
- `npm run lint`
- Build de Tauri en Windows (matriz)

Tests usan **solo fixtures sintéticos** generados en `tests/fixtures/`. Nunca evidencia real.

-----

## 15. Plan de fases

**Fase 0 — Fundaciones (1–2 semanas)**

- Setup Tauri v2 + React + TS + Vite.
- CI con lints, formato, tests.
- Estructura de carpetas, módulos vacíos.
- Tipos compartidos TS espejados a mano.
- Logging estructurado (`tracing`).
- Generador básico de SQLite sintético para tests.

**Fase 1 — Núcleo de evidencia (2–3 semanas)**

- Crear/abrir caso, manifest con file locking, audit log encadenado.
- Vista previa: detección de hermanos, validación header.
- Hashing SHA-256 con progreso y cancelación.
- Ingesta con doble copia (pristine + working) y verificación.
- UI de creación de caso e ingesta.
- Tests con fixtures sintéticos.

**Fase 2 — Acceso SQLite seguro e introspección (1–2 semanas)**

- `db::opener` con ambos modos.
- `safe_query` con whitelist.
- Introspección de schema.
- Re-hash de working después de cada apertura.
- UI de inspección.
- Verificar empíricamente comportamiento de `immutable=1` en Windows con archivos reales.

**Fase 3 — Primer parser: WhatsApp iOS (2–3 semanas)**

- Parser sobre `ChatStorage.sqlite` con esquema verificado contra evidencia real disponible.
- Mapeos en TOML externalizados.
- Conversión de timestamps Mac Absolute.
- UI de exploración con filtros y paginación.
- Documentar en `SCHEMAS.md` la versión validada.

**Fase 4 — Segundo y tercer parser (2 semanas)**

- WhatsApp Android (`msgstore.db`).
- CallHistory iOS (storedata + sqlite).

**Fase 5 — Análisis derivado (2 semanas)**

- Gaps.
- Señales de eliminados (categorías A y B separadas).
- Estadísticas.

**Fase 6 — Exportación XLSX (2 semanas)**

- Todas las hojas.
- Charts.
- Reproducibilidad bit-identical.

**Fase 7 — Hardening y empaquetado (1–2 semanas)**

- Manejo de errores robusto.
- Instalador Windows.
- Firma de código (al menos autofirmada inicialmente).
- Documentación de usuario.

**Fase 8 (opcional) — Recovery avanzado**

- Carving de freelist.
- Reconstrucción de WAL frames antiguos.

-----

## 16. Riesgos técnicos y mitigaciones

|Riesgo                                                                   |Impacto              |Mitigación                                                                                               |
|-------------------------------------------------------------------------|---------------------|---------------------------------------------------------------------------------------------------------|
|Modo “incluye WAL” modifica la copia working por checkpoint automático   |Medio                |Trabajar sobre `working` (no `pristine`), re-hashear después, registrar cambio en audit.log como esperado|
|Esquema de WhatsApp cambia entre versiones, parser falla silencioso      |Alto                 |Validar columnas esperadas, warnings explícitos, mapeos versionados en TOML                              |
|Tipos desconocidos clasificados mal                                      |Medio                |Conservar `*_raw` siempre, mapeo externalizado, valor `Unknown(raw)` por defecto                         |
|Dataset >1M mensajes degrada UI                                          |Medio                |Tabla virtualizada, paginación backend                                                                   |
|Dos instancias sobre mismo caso                                          |Medio                |Lock file en directorio de caso                                                                          |
|Crash durante parseo → reparsear todo                                    |Bajo-Medio           |Aceptable fase 1, cache en SQLite separada en fase 5–6                                                   |
|Charts con datasets grandes pesan demasiado                              |Bajo                 |Agregar por bucket temporal                                                                              |
|Falsos positivos en “eliminados”                                         |**Alto reputacional**|Separar categorías A y B, `evidence_strength`, nota metodológica obligatoria                             |
|Audit log encadenado no es firma digital, da falsa sensación de seguridad|Medio                |Documentarlo explícitamente como trazabilidad                                                            |
|APIs Tauri v2 cambian en minor versions                                  |Medio                |Pinear versión exacta, aislar uso en `commands`                                                          |
|Antivirus Windows marca el ejecutable                                    |Medio                |Firma de código (Authenticode)                                                                           |
|`cargo audit` reporta vulnerabilidades en deps                           |Variable             |Política de revisión por release                                                                         |
|Reproducibilidad rota por orden no-determinístico                        |Medio                |`BTreeMap`, ordenar antes de serializar, tests de reproducibilidad                                       |

-----

## 17. Supuestos a verificar antes de implementar

**Tauri v2 (línea actual 2.x estable, confirmar versión exacta):**

- Compatibilidad de `ts-rs` o `tauri-specta` con la versión usada.
- Configuración exacta de capabilities para los comandos definidos.
- Empaquetado Windows con `tauri-bundler` (MSI/NSIS, firma).
- Channel API: confirmar uso correcto para streaming de progreso.

**rusqlite / SQLite:**

- Confirmar empíricamente en Windows que `immutable=1` realmente no asocia WAL y no toca el archivo en disco.
- Comportamiento al abrir DB con journal pendiente (corrupción posible si la app terminó mid-write).

**Esquemas reales (CRÍTICO — nunca asumir):**

- `ChatStorage.sqlite` de WhatsApp iOS: tablas `ZWAMESSAGE`, `ZWACHATSESSION`, `ZWAMEDIAITEM` y columnas, **por versión exacta**.
- `msgstore.db` de WhatsApp Android, **por versión**.
- Códigos de tipo de mensaje y revocación, **por versión**.
- `CallHistory.storedata` vs `CallHistory.sqlite`: cuál aplica según iOS.
- Core Data: tablas Z_, formato Mac Absolute por columna.
- Tablas auxiliares de rastros (si existen).

**rust_xlsxwriter:**

- Límites prácticos con cientos de miles de filas.
- Tipos de chart disponibles.
- Comportamiento con emojis y RTL.

**Entorno operativo:**

- ¿Archivos solo en disco local o también red? Afecta locking.
- Tamaño típico de evidencias (define si conviene streaming desde inicio).
- **Descifrado FUERA DE ALCANCE fase 1** (`msgstore.db.crypt14`, backups iOS cifrados). Documentar en `LIMITATIONS.md`.

-----

## 18. Glosario

- **Caso:** unidad de trabajo. Directorio con manifest, evidencias, exports.
- **Evidencia:** archivo fuente registrado en un caso. Tiene pristine + working.
- **Pristine:** copia inmutable, nunca abierta con SQLite. Evidencia copiada formal.
- **Working:** copia operativa, se abre read-only. Puede tocar el WAL en modo “incluye WAL”.
- **Parser:** módulo que conoce el esquema de una fuente y lo traduce al modelo común.
- **raw_row:** fila SQLite original serializada, conservada para trazabilidad.
- **Modo de análisis:** “solo committed” (con `immutable=1`, sin WAL) o “incluye WAL” (puede modificar working).
- **Señal:** indicio de anomalía. Nunca “prueba”.
- **evidence_strength:** clasificación `weak | moderate | strong` de un hallazgo, con criterios documentados.
