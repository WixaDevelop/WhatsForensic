# CLAUDE.md — Frontend React + TypeScript (src)

Convenciones específicas del frontend. Las reglas generales están en el `CLAUDE.md` del root.

-----

## Convención de idioma

- **Código TS/TSX:** inglés. Nombres de componentes, hooks, tipos, props, variables.
- **Comentarios `//`:** inglés cuando explican mecánica.
- **JSDoc `/** */`:** español cuando documentan componentes/hooks de cara al desarrollador del equipo.
- **Strings visibles al usuario:** español. Idealmente centralizados en archivos de i18n desde el inicio, aunque solo haya un idioma activo.
- **Console logs:** inglés.

Ejemplo:

```tsx
/**
 * Muestra el progreso de hashing de una evidencia, escuchando el Channel
 * `evidence:ingest` del backend.
 */
export function HashProgressPanel({ evidenceId }: HashProgressPanelProps) {
  const [percent, setPercent] = useState(0);

  useEffect(() => {
    // Subscribe to Tauri channel for streaming progress.
    const unlisten = subscribeToIngestProgress(evidenceId, (p) => {
      setPercent(p.percent);
    });
    return () => unlisten();
  }, [evidenceId]);

  return <div>Calculando hash: {percent}%</div>;
}
```

-----

## Estructura

```
src/
├── api/
│   ├── tauri.ts          # invoke wrappers, tipados
│   └── events.ts         # Channel subscriptions
├── types/
│   └── domain.ts         # Tipos espejados de Rust
├── pages/                # Top-level routes
├── components/           # Reutilizables
├── state/
│   └── caseStore.ts      # Zustand
├── i18n/
│   └── es.ts             # Strings en español
└── styles/
```

-----

## Tipos compartidos con Rust

- Fase 0–1: tipos TS escritos a mano en `types/domain.ts`, espejados de los structs Rust serializables.
- Cualquier cambio en un struct serializable del backend obliga a actualizar el tipo TS correspondiente.
- Cuando se evalúe `ts-rs` o `tauri-specta` al final de fase 1, migrar gradualmente.

Convenciones:

- Nombres en PascalCase.
- Discriminated unions para enums con datos (igual que `serde` con `tag="kind"`).
- Fechas como `string` (ISO 8601 UTC).

```ts
export type AnalysisMode = 'committed_only' | 'with_wal';

export type MessageDirection = 'incoming' | 'outgoing' | 'unknown';

export interface ParsedMessage {
  id: string;
  conversationId: string;
  sourcePk: number;
  timestampUtc: string;
  timestampRaw: number;
  timestampRawFormat: 'mac_absolute' | 'unix_s' | 'unix_ms';
  sender: string | null;
  direction: MessageDirection;
  body: string | null;
  messageTypeRaw: number;
  messageTypeInterpreted: string;
  isPossiblyDeleted: boolean;
  isPossiblyRevoked: boolean;
  rawRow: Record<string, unknown>;
}
```

-----

## Llamadas al backend

- Toda llamada a `invoke` pasa por `api/tauri.ts`. Nunca importar `@tauri-apps/api/core` directamente desde componentes.
- Tipar el retorno explícitamente.
- Manejar `AppError` mapeando por `code` (estable), no por `message` (humano).

```ts
// api/tauri.ts
import { invoke } from '@tauri-apps/api/core';
import type { EvidencePreview, AppError } from '../types/domain';

export async function previewEvidence(path: string): Promise<EvidencePreview> {
  try {
    return await invoke<EvidencePreview>('evidence_preview', { path });
  } catch (err) {
    throw normalizeAppError(err);
  }
}
```

-----

## Estado

- Zustand para estado global (caso actual, evidencia seleccionada, resultados cacheados).
- Estado local de componentes con `useState`.
- No introducir Redux, MobX, Recoil. Si Zustand se queda corto, discutirlo antes de migrar.

```ts
interface CaseStore {
  currentCase: CaseSummary | null;
  setCurrentCase: (c: CaseSummary | null) => void;
  evidences: EvidenceSummary[];
  refreshEvidences: () => Promise<void>;
}
```

-----

## Componentes

- Functional components con hooks. No clases.
- Props tipadas con interfaces, no `type` (excepto cuando hay union).
- Un componente por archivo. Nombre del archivo igual al componente: `EvidenceList.tsx`.
- Componentes con lógica de datos delegan presentación a sub-componentes “tontos”.

-----

## Lenguaje prudente en UI

La regla del root sobre vocabulario forense aplica también a la UI:

- **Permitido:** “señal compatible con”, “indicio”, “anomalía detectada”, “requiere correlación”, “posible”.
- **Prohibido:** “borrado”, “eliminado” (como afirmación), “prueba”, “demuestra”, “confirma”.

Tooltips y textos de ayuda deben explicar las limitaciones de cada hallazgo.

-----

## Tablas y datasets grandes

- Tabla virtualizada obligatoria cuando el dataset puede superar 10.000 filas. Recomendado TanStack Table + TanStack Virtual.
- Paginación en backend, no en frontend, para datasets de mensajes.
- Filtros aplicados en backend vía comandos tipados.

-----

## Manejo de errores en UI

- Errores de `AppError.kind = "schema_mismatch"`: mostrar como warning con detalle de qué columnas faltaron. No bloquean uso, permiten análisis parcial.
- Errores de `AppError.kind = "integrity"`: críticos. Mostrar modal bloqueante.
- Errores de `AppError.kind = "io"`: snackbar con botón “reintentar” cuando aplique.
- Nunca mostrar el stack trace o el mensaje en inglés interno al usuario.

-----

## Estilo

- CSS Modules o Tailwind. Decidir al inicio de fase 0 y no mezclar.
- Tema neutro, profesional. No emojis en UI forense.
- Tablas con líneas claras, fuente monoespaciada para hashes y datos crudos.

-----

## Comandos del proyecto

```bash
npm run dev          # Vite dev server (sin Tauri)
npm run tauri dev    # App completa
npm run build        # Build frontend
npm run typecheck    # tsc --noEmit
npm run lint         # ESLint
npm run format       # Prettier
```

-----

## Dependencias

- `package-lock.json` versionado.
- Mínimo de dependencias. Cada nueva requiere justificación.
- Crates clave esperadas:
  - `react`, `react-dom`
  - `@tauri-apps/api` (versión correspondiente a Tauri v2)
  - `zustand`
  - `@tanstack/react-table` y `@tanstack/react-virtual` (cuando se introduzcan tablas grandes)
  - `recharts` (para charts en UI; los charts del XLSX los hace el backend)
- TypeScript estricto: `strict: true` en `tsconfig.json`.
