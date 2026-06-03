/**
 * Estado global de la aplicación. Zustand.
 *
 * Refleja el caso actualmente abierto, sus evidencias, y la pantalla actual.
 * El estado real de fondo (manifest, audit log) vive en el backend; este store
 * es proyección para la UI.
 */

import { create } from 'zustand';
import type { CaseSummary, EvidenceSummary } from '../types/domain';

export type Screen = 'home' | 'createCase' | 'workspace';

interface CaseStore {
  screen: Screen;
  setScreen: (s: Screen) => void;

  currentCase: CaseSummary | null;
  setCurrentCase: (c: CaseSummary | null) => void;

  evidences: EvidenceSummary[];
  setEvidences: (es: EvidenceSummary[]) => void;
}

export const useCaseStore = create<CaseStore>((set) => ({
  screen: 'home',
  setScreen: (screen) => set({ screen }),
  currentCase: null,
  setCurrentCase: (currentCase) => set({ currentCase }),
  evidences: [],
  setEvidences: (evidences) => set({ evidences }),
}));
