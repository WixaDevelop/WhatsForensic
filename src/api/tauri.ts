/**
 * Punto único de entrada al backend Tauri. Ningún componente debe importar
 * `@tauri-apps/api/core` directo. Toda llamada `invoke` se tipa aquí.
 *
 * Manejo de errores: el backend siempre serializa `AppError` con campos
 * estables. El frontend rutea por `code`, no por `message`.
 */

import { Channel, invoke } from '@tauri-apps/api/core';
import type { AppError, ProgressEvent, SystemInfo } from '../types/domain';

/** Información estática de build del backend. */
export async function getSystemInfo(): Promise<SystemInfo> {
  return invoke<SystemInfo>('system_info');
}

/**
 * Demo del patrón Channel para streaming de progreso. El callback recibe cada
 * evento emitido por el backend.
 *
 * En fases posteriores este patrón se reutiliza para hashing de evidencia,
 * parseo y exportación.
 */
export async function runProgressDemo(onProgress: (event: ProgressEvent) => void): Promise<void> {
  const channel = new Channel<ProgressEvent>();
  channel.onmessage = onProgress;
  await invoke('progress_demo', { onEvent: channel });
}

/**
 * Normaliza un error desconocido a `AppError`. Cualquier excepción que cruza
 * un `invoke` debería tener forma de `AppError`, pero defendemos el caso de
 * un fallo de transporte u otro escenario raro.
 */
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
