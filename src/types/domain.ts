/**
 * Tipos espejados a mano desde los structs serializables del backend Rust.
 *
 * Cualquier cambio en un struct serializable obliga a actualizar este archivo.
 * Cuando se evalúe `ts-rs` o `tauri-specta` al final de la Fase 1, migrar
 * gradualmente desde aquí.
 *
 * Convención (ver `src/CLAUDE.md`):
 * - PascalCase para nombres.
 * - Discriminated unions para enums con datos.
 * - Fechas como `string` (ISO 8601 UTC).
 */

// ---------------------------------------------------------------------------
// Sistema (Fase 0)
// ---------------------------------------------------------------------------

export interface SystemInfo {
  toolName: string;
  toolVersion: string;
  rustEdition: string;
  targetOs: string;
  targetArch: string;
}

export interface ProgressEvent {
  phase: string;
  percent: number;
}

// ---------------------------------------------------------------------------
// Errores serializables (Fase 0)
// ---------------------------------------------------------------------------

export type AppErrorKind =
  | 'io'
  | 'schema_mismatch'
  | 'integrity'
  | 'permission'
  | 'invalid_input'
  | 'internal';

export interface AppError {
  kind: AppErrorKind;
  code: string;
  message: string;
  context?: Record<string, unknown>;
}

// ---------------------------------------------------------------------------
// Modelo de dominio (placeholders para Fase 1+)
// ---------------------------------------------------------------------------

/** Modo de apertura de la base SQLite working. */
export type AnalysisMode = 'committed_only' | 'with_wal';

/** Dirección de un mensaje en una conversación. */
export type MessageDirection = 'incoming' | 'outgoing' | 'unknown';

/** Formato del valor crudo de un timestamp en una columna. */
export type TimestampRawFormat = 'mac_absolute' | 'unix_s' | 'unix_ms';
