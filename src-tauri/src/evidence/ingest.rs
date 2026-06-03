//! Ingesta de evidencia: detección DB+WAL+SHM+journal, doble copia, verificación.
//!
//! TODO fase 1: implementar el flujo `evidence_ingest`:
//! 1. Detectar hermanos (`-wal`, `-shm`, `-journal`) junto al archivo principal.
//! 2. Hashear original (streaming).
//! 3. Copiar a `pristine/` + re-hashear + verificar igualdad con original.
//! 4. Copiar `pristine/` → `working/` + hashear working.
//! 5. Registrar en `case.json` y `audit.log`.
