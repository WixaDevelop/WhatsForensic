/**
 * Estado global de la aplicación. Zustand.
 *
 * En Fase 0 sólo contiene un flag de readiness del backend. Fase 1 incorpora
 * `currentCase`, `evidences` y métodos de refresh.
 */

import { create } from 'zustand';

interface CaseStore {
  backendReady: boolean;
  setBackendReady: (ready: boolean) => void;
}

export const useCaseStore = create<CaseStore>((set) => ({
  backendReady: false,
  setBackendReady: (backendReady) => set({ backendReady }),
}));
