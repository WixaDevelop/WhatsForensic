/**
 * Punto único de entrada al backend Tauri. Ningún componente debe importar
 * `@tauri-apps/api/core` directo. Toda llamada `invoke` se tipa aquí.
 *
 * Manejo de errores: el backend siempre serializa `AppError` con campos
 * estables. El frontend rutea por `code`, no por `message`.
 */

import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  AnalysisRunSummary,
  AppError,
  CaseCreateInput,
  CaseSummary,
  DeletedHint,
  EvidenceIngestResult,
  EvidencePreview,
  EvidenceSummary,
  ExportXlsxResult,
  Gap,
  IngestProgressEvent,
  IntegrityReport,
  OpenMode,
  PagedMessages,
  ParserDescriptor,
  ParserMatch,
  ProgressEvent,
  SchemaSnapshot,
  SystemInfo,
} from '../types/domain';

// ---------------------------------------------------------------------------
// Sistema
// ---------------------------------------------------------------------------

export async function getSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>('system_info');
}

export async function runProgressDemo(onProgress: (event: ProgressEvent) => void): Promise<void> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  await invoke('progress_demo', { onEvent: channel });
}

// ---------------------------------------------------------------------------
// Caso
// ---------------------------------------------------------------------------

export async function createCase(input: CaseCreateInput): Promise<CaseSummary> {
  return invoke<CaseSummary>('case_create', { input });
}

export async function openCase(caseDir: string): Promise<CaseSummary> {
  return invoke<CaseSummary>('case_open', { input: { caseDir } });
}

export async function closeCase(): Promise<void> {
  await invoke('case_close');
}

export async function getCurrentCase(): Promise<CaseSummary | null> {
  return invoke<CaseSummary | null>('case_get_current');
}

export async function getDefaultWorkspaceRoot(): Promise<string | null> {
  return invoke<string | null>('default_workspace_root');
}

// ---------------------------------------------------------------------------
// Evidencia
// ---------------------------------------------------------------------------

export async function previewEvidence(path: string): Promise<EvidencePreview> {
  return invoke<EvidencePreview>('evidence_preview', { input: { path } });
}

export async function ingestEvidence(
  path: string,
  declaredType: string | null,
  onProgress: (evt: IngestProgressEvent) => void,
): Promise<EvidenceIngestResult> {
  const channel = new Channel<IngestProgressEvent>();
  channel.onmessage = onProgress;
  return invoke<EvidenceIngestResult>('evidence_ingest', {
    input: { path, declaredType },
    onEvent: channel,
  });
}

export async function listEvidence(): Promise<EvidenceSummary[]> {
  return invoke<EvidenceSummary[]>('evidence_list');
}

export async function verifyEvidence(evidenceId: string): Promise<IntegrityReport> {
  return invoke<IntegrityReport>('evidence_verify', { input: { evidenceId } });
}

export async function cancelTask(runId: string): Promise<boolean> {
  return invoke<boolean>('task_cancel', { runId });
}

// ---------------------------------------------------------------------------
// Schema (Fase 2)
// ---------------------------------------------------------------------------

export async function introspectEvidence(
  evidenceId: string,
  mode: OpenMode,
): Promise<SchemaSnapshot> {
  return invoke<SchemaSnapshot>('evidence_introspect', { input: { evidenceId, mode } });
}

// ---------------------------------------------------------------------------
// Analysis (Fases 3–5)
// ---------------------------------------------------------------------------

export async function listParsers(): Promise<ParserDescriptor[]> {
  return invoke<ParserDescriptor[]>('analysis_list_parsers');
}

export async function detectParsers(evidenceId: string, mode: OpenMode): Promise<ParserMatch[]> {
  return invoke<ParserMatch[]>('analysis_detect_parsers', { input: { evidenceId, mode } });
}

export async function runAnalysis(
  evidenceId: string,
  parserKey: string,
  mode: OpenMode,
): Promise<AnalysisRunSummary> {
  return invoke<AnalysisRunSummary>('analysis_run', {
    input: { evidenceId, parserKey, mode },
  });
}

export async function queryMessages(
  runId: string,
  page: number,
  pageSize: number,
  filter?: string,
): Promise<PagedMessages> {
  return invoke<PagedMessages>('analysis_query_messages', {
    request: { runId, page, pageSize, filter: filter ?? null },
  });
}

export async function getGaps(runId: string): Promise<Gap[]> {
  return invoke<Gap[]>('analysis_get_gaps', { runId });
}

export async function getDeletedHints(runId: string): Promise<DeletedHint[]> {
  return invoke<DeletedHint[]>('analysis_get_deleted_hints', { runId });
}

// ---------------------------------------------------------------------------
// Export (Fase 6)
// ---------------------------------------------------------------------------

export async function exportXlsx(
  runId: string,
  outputPath: string,
  includeRawRowJson: boolean,
): Promise<ExportXlsxResult> {
  return invoke<ExportXlsxResult>('export_xlsx', {
    input: { runId, outputPath, includeRawRowJson },
  });
}

// ---------------------------------------------------------------------------
// Manejo de errores
// ---------------------------------------------------------------------------

export function normalizeAppError(err: unknown): AppError {
  if (err && typeof err === 'object' && 'kind' in err && 'code' in err) {
    return err as AppError;
  }
  return {
    kind: 'internal',
    code: 'UNKNOWN',
    message: typeof err === 'string' ? err : 'Error desconocido',
  };
}
