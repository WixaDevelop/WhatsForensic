/**
 * Punto único de entrada al backend Tauri. Ningún componente debe importar
 * `@tauri-apps/api/core` directo. Toda llamada `invoke` se tipa aquí.
 *
 * Manejo de errores: el backend siempre serializa `AppError` con campos
 * estables. El frontend rutea por `code`, no por `message`.
 */

import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  AppError,
  CaseCreateInput,
  CaseSummary,
  EvidenceIngestResult,
  EvidencePreview,
  EvidenceSummary,
  IngestProgressEvent,
  IntegrityReport,
  OpenMode,
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
