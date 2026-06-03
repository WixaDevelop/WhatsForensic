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
// Errores serializables
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
// Caso (Fase 1)
// ---------------------------------------------------------------------------

export interface CaseSummary {
  caseId: string;
  name: string;
  description?: string | null;
  investigator: string;
  timezone: string;
  createdAt: string;
  toolVersion: string;
  caseDir: string;
  evidenceCount: number;
}

export interface CaseCreateInput {
  name: string;
  description?: string | null;
  investigator: string;
  timezone: string;
  workspaceRoot: string;
}

// ---------------------------------------------------------------------------
// Evidencia (Fase 1)
// ---------------------------------------------------------------------------

export interface SqliteHeader {
  validMagic: boolean;
  pageSize: number;
  fileSize: number;
}

export interface SidecarSet {
  wal?: string | null;
  shm?: string | null;
  journal?: string | null;
}

export interface EvidencePreview {
  originalPath: string;
  size: number;
  header: SqliteHeader;
  sidecars: SidecarSet;
}

export interface EvidenceSummary {
  evidenceId: string;
  filename: string;
  sourceType?: string | null;
  originalSize: number;
  originalSha256: string;
  pristineSha256: string;
  workingSha256: string;
  hasWal: boolean;
  hasShm: boolean;
  hasJournal: boolean;
  ingestedAt: string;
}

export interface HashProgress {
  bytesDone: number;
  bytesTotal: number;
  percent: number;
}

export type IngestStep =
  | 'hashing_original'
  | 'hashing_pristine'
  | 'hashing_working'
  | 'hashing_sidecar';

export interface IngestProgressEvent {
  runId: string;
  step: IngestStep;
  progress: HashProgress;
}

export interface EvidenceIngestResult {
  runId: string;
  evidenceId: string;
}

export interface IntegrityReport {
  evidenceId: string;
  pristineMatches: boolean;
  workingMatches: boolean;
  expectedPristineSha256: string;
  actualPristineSha256: string;
  expectedWorkingSha256: string;
  actualWorkingSha256: string;
}

// ---------------------------------------------------------------------------
// Modelo de dominio (placeholders para Fase 2+)
// ---------------------------------------------------------------------------

export type AnalysisMode = 'committed_only' | 'with_wal';
export type MessageDirection = 'incoming' | 'outgoing' | 'unknown';
export type TimestampRawFormat = 'mac_absolute' | 'unix_s' | 'unix_ms';
