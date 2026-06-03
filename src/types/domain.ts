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
// Schema introspection (Fase 2)
// ---------------------------------------------------------------------------

export type OpenMode = 'committed_only' | 'with_wal';

export interface ColumnInfo {
  cid: number;
  name: string;
  type: string;
  notnull: boolean;
  pk: boolean;
  defaultValue?: string | null;
}

export interface TableInfo {
  name: string;
  kind: string;
  sql: string;
  columns: ColumnInfo[];
  rowCount: number;
}

export interface IndexInfo {
  name: string;
  table: string;
  unique: boolean;
  sql?: string | null;
}

export interface SchemaSnapshot {
  sqliteVersion: string;
  userVersion: number;
  applicationId: number;
  pageCount: number;
  pageSize: number;
  journalMode: string;
  tables: Record<string, TableInfo>;
  indexes: IndexInfo[];
}

// ---------------------------------------------------------------------------
// Parsers (Fase 3+4)
// ---------------------------------------------------------------------------

export type AnalysisMode = OpenMode;
export type MessageDirection = 'incoming' | 'outgoing' | 'unknown';
export type TimestampRawFormat = 'mac_absolute' | 'unix_s' | 'unix_ms';

export type ParserConfidence = 'none' | 'low' | 'medium' | 'high';

export interface ParserDescriptor {
  key: string;
  displayName: string;
}

export interface ParserMatch {
  key: string;
  displayName: string;
  confidence: ParserConfidence;
}

// ---------------------------------------------------------------------------
// Analysis (Fase 5)
// ---------------------------------------------------------------------------

export interface AnalysisRunSummary {
  runId: string;
  evidenceId: string;
  parserKey: string;
  mode: AnalysisMode;
  sourceKind: string;
  schemaVersionUsed: string;
  schemaVerified: boolean;
  conversationCount: number;
  messageCount: number;
  callCount: number;
  warningCount: number;
  revokedCount: number;
  gapCount: number;
  deletedHintCount: number;
}

export interface ParsedMessage {
  id: string;
  conversationId: string;
  sourcePk: number;
  timestampUtc?: string | null;
  timestampRaw?: number | null;
  timestampRawFormat?: TimestampRawFormat | null;
  sender?: string | null;
  direction: MessageDirection;
  body?: string | null;
  messageTypeRaw?: number | null;
  messageTypeInterpreted?: string | null;
  messageTypeVerified: boolean;
  isPossiblyRevoked: boolean;
  rawRow: Record<string, unknown>;
}

export interface PagedMessages {
  total: number;
  page: number;
  pageSize: number;
  messages: ParsedMessage[];
}

export interface Conversation {
  id: string;
  displayName?: string | null;
  sourceTable: string;
  sourcePk: number;
  firstSeenUtc?: string | null;
  lastSeenUtc?: string | null;
  messageCount: number;
}

export interface Gap {
  table: string;
  column: string;
  rangeStart: number;
  rangeEnd: number;
  size: number;
  prevPk?: number | null;
  nextPk?: number | null;
  source: 'pk_sequence' | 'sqlite_sequence_tail';
  interpretationNote: string;
}

export type HintCategory = 'a' | 'b';
export type EvidenceStrength = 'weak' | 'moderate' | 'strong';

export interface DeletedHint {
  category: HintCategory;
  messageId: string;
  kind: string;
  evidenceStrength: EvidenceStrength;
  note: string;
}

export interface ExportXlsxResult {
  outputPath: string;
  bytesWritten: number;
}
