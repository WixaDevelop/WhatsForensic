import { useEffect } from 'react';
import { getCurrentCase, listEvidence, normalizeAppError } from './api/tauri';
import { useCaseStore } from './state/caseStore';
import { HomePage } from './pages/HomePage';
import { CaseSetupPage } from './pages/CaseSetupPage';
import { WorkspacePage } from './pages/WorkspacePage';
import './App.css';

function App() {
  const screen = useCaseStore((s) => s.screen);
  const currentCase = useCaseStore((s) => s.currentCase);
  const setCurrentCase = useCaseStore((s) => s.setCurrentCase);
  const setEvidences = useCaseStore((s) => s.setEvidences);
  const setScreen = useCaseStore((s) => s.setScreen);

  useEffect(() => {
    void (async () => {
      try {
        const c = await getCurrentCase();
        if (c) {
          setCurrentCase(c);
          setScreen('workspace');
          const es = await listEvidence();
          setEvidences(es);
        }
      } catch (e) {
        console.error('initial sync failed', normalizeAppError(e));
      }
    })();
  }, [setCurrentCase, setEvidences, setScreen]);

  if (screen === 'createCase') return <CaseSetupPage />;
  if (screen === 'workspace' && currentCase) return <WorkspacePage />;
  return <HomePage />;
}

export default App;
