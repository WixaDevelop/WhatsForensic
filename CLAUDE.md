# CLAUDE.md — Forensic SQLite Analyzer

Contexto operativo para Claude Code. Diseño completo en `docs/ARCHITECTURE.md`.

-----

## Cómo trabajar aquí

Actuar como **arquitecto senior y revisor técnico**, no como ejecutor pasivo.

- No asumir que lo solicitado es la mejor solución. Verificar coherencia técnica antes de codificar.
- Verificar contra fuentes actuales cuando se trate de Tauri v2, rusqlite, rust_xlsxwriter, esquemas SQLite/Core Data, o prácticas forenses.
- Si algo no se puede verificar, decirlo: *“No puedo confirmar esto con las fuentes disponibles.”*
- Nunca inventar columnas, tablas, códigos de tipo ni significados forenses.
- Diferenciar siempre: hecho confirmado / inferencia razonable / hipótesis / no confirmado.
- Cuando propongas mejoras, usar formato: Qué cambiaría / Por qué / Beneficio / Riesgo / Recomendación final.
- Toda propuesta que dependa del esquema real, versión de WhatsApp/iOS, o limitaciones de framework debe incluir sección **“Supuestos que debo verificar antes de implementar”**.
- No implementar fuera del alcance de la fase actual sin confirmación explícita.

-----

## Qué es esto

Herramienta de escritorio para análisis forense de bases SQLite extraídas de móviles (WhatsApp Android/iOS, CallHistory iOS, otras). Usuario: investigadores. Output: reportes XLSX que pueden apoyar peritajes. **No es herramienta certificada judicialmente**, pero el diseño debe tener rigor forense.

-----

## Convención bilingüe

El proyecto es bilingüe español/inglés. La regla:

- **Inglés:** nombres de archivos, carpetas, tipos, funciones, variables, módulos, commits, comentarios de código, logs técnicos (`tracing`).
- **Español:** este `CLAUDE.md`, toda la documentación en `docs/`, textos visibles al usuario (UI, mensajes de error de `AppError.message`, contenido del reporte XLSX), descripciones humanas en `audit.log`.
- Convenciones específicas por capa en `src-tauri/CLAUDE.md` y `src/CLAUDE.md`.

-----

## Reglas forenses no negociables

1. **La evidencia original es solo lectura absoluta.** Nunca abrirla con SQLite. Solo leerla como bytes para hashear y copiar.
1. **Doble copia obligatoria:** `pristine` (nunca abierta con SQLite, es la evidencia copiada formal) y `working` (operativa, se abre read-only).
1. **Copiar WAL, SHM y `-journal` si existen** junto a la DB. Detectar, reportar, hashear cada uno.
1. **SHA-256** del original y de cada copia. Verificación post-copia obligatoria. Re-hash después de cada apertura de la working.
1. **Audit log encadenado por hash** de todas las acciones. No es firma criptográfica fuerte — documentarlo como trazabilidad, no como anti-tamper.
1. **Conservar `raw_row`** de cada registro parseado.
1. **Nunca afirmar borrado.** Vocabulario obligatorio: “señal”, “indicio”, “anomalía”, “compatible con”, “requiere correlación”. Prohibido: “prueba”, “demuestra”, “confirma”.
1. **Nunca hardcodear** mapeos de códigos de tipo de mensaje. Externalizar a `src-tauri/resources/schemas/<source>/<version>.toml`.
1. **Reproducibilidad bit-identical** sobre la misma entrada: ordenar resultados explícitamente, no usar estructuras con orden no-determinístico al serializar, embedir versión de herramienta y dependencias en cada reporte.
1. **Nunca commitear evidencia real** al repo. Tests usan fixtures sintéticos en `tests/fixtures/`.

-----

## Stack

**Backend (Rust):** Tauri v2 (línea 2.x estable), rusqlite con feature `bundled`, sha2, rust_xlsxwriter, serde, thiserror (por módulo) + anyhow (solo en capa de comandos), uuid, chrono + chrono-tz, tracing, walkdir.

**Frontend:** React + TypeScript + Vite, Zustand para estado, tabla virtualizada para datasets grandes.

**Descartado:** `tauri-plugin-sql` (abstrae demasiado), `sqlx`, `diesel`.

**Plataforma objetivo:** Windows primero, diseño portable.

-----

## Acceso SQLite — gotcha crítico

Modo de apertura sobre la copia working:

- **Modo “solo committed”:** abrir con URI `file:<path>?immutable=1` + `OpenFlags::SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI | SQLITE_OPEN_NO_MUTEX`. SQLite documentado: con `immutable=1` deshabilita locking, detección de cambios **y no asocia WAL**. Equivale a “ver solo lo persistido en la DB principal”.
- **Modo “incluye WAL”:** abrir SIN `immutable`, solo `SQLITE_OPEN_READ_ONLY` + `PRAGMA query_only = ON`. SQLite **puede modificar la copia** (checkpoint automático). Por eso se trabaja sobre `working`, no sobre `pristine`, y se re-hashea después.

El modo usado **debe registrarse en el reporte XLSX y en audit.log**. Nunca abrir el archivo original.

Defensa adicional: `db::safe_query` rechaza statements que no empiecen por `SELECT` o `PRAGMA`. Frontend nunca envía SQL — solo invoca comandos tipados.

-----

## Formatos de timestamp a soportar

Conservar siempre el valor crudo en columna separada, además del convertido a UTC.

- **Mac Absolute / Cocoa:** segundos desde 2001-01-01 UTC. Core Data, iOS (`CallHistory.storedata`, columnas Z_).
- **Unix segundos:** común en Android.
- **Unix milisegundos:** común en Android (`msgstore.db`).
- **Detectar por rango:** un valor > 10^12 probablemente es ms, no s. Documentar la heurística.

-----

## Estructura del workspace de caso (runtime)

```
<workspace_root>/<case_id>/
├── case.json                     # Manifest
├── audit.log                     # JSON Lines, append-only, encadenado por hash
├── evidence/<evidence_id>/
│   ├── source_metadata.json
│   ├── pristine.db               # NUNCA abrir con SQLite
│   ├── pristine.db-wal           # si existía
│   ├── pristine.db-shm           # si existía
│   ├── pristine.db-journal       # si existía
│   ├── pristine.sha256
│   ├── working.db                # operativa, read-only
│   ├── working.db-wal/-shm/-journal (si aplica)
│   └── working.sha256
├── exports/report_<timestamp>.xlsx
└── tool_version.txt
```

-----

## Qué NO hacer

- Abrir el archivo original con SQLite, jamás.
- Usar `tauri-plugin-sql`.
- Aceptar SQL crudo desde el frontend.
- Hardcodear códigos de tipo de mensaje o sus significados.
- Lenguaje absoluto en outputs forenses.
- Inventar columnas o tablas sin verificación empírica.
- Fallar silencioso cuando un esquema no coincide — siempre reportar qué se intentó.
- Incrustar multimedia en el XLSX (fase 1).
- Calcular “scores de probabilidad de borrado”.
- Commitear evidencia real al repo.
- Persistir datos parseados en SQLite en fase 1 (vive en memoria; cache se introduce en fase 5–6).

-----

## Fases (estado actual: pre-Fase 0)

1. Fundaciones (setup, CI, módulos vacíos, logging).
1. Núcleo de evidencia (casos, manifest, audit log, hashing, ingesta).
1. Acceso SQLite seguro e introspección.
1. Primer parser (WhatsApp iOS).
1. Segundo/tercer parser (WhatsApp Android, CallHistory iOS).
1. Análisis derivado (gaps, señales, stats).
1. Exportación XLSX.
1. Hardening y empaquetado.
1. (Opcional) Recovery avanzado: carving de freelist, WAL frames antiguos.

No avanzar a una fase nueva sin confirmación explícita.

-----

## Comandos del proyecto

```bash
# Desarrollo
npm install
npm run tauri dev

# Verificación previa a commit
cargo fmt --check
cargo clippy -- -D warnings
cargo test
npm run typecheck
npm run lint

# Build de producción
npm run tauri build
```

-----

## Referencias

- Diseño completo: `docs/ARCHITECTURE.md`
- Metodología forense (incluida literal en XLSX): `docs/METHODOLOGY.md`
- Esquemas verificados por versión: `docs/SCHEMAS.md`
- Qué la herramienta no hace: `docs/LIMITATIONS.md`
- Convenciones específicas backend: `src-tauri/CLAUDE.md`
- Convenciones específicas frontend: `src/CLAUDE.md`
