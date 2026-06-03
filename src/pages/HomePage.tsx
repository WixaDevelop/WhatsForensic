/**
 * Pantalla inicial: ofrece crear caso o abrir uno existente.
 */

import { useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { normalizeAppError, openCase } from '../api/tauri';
import { useCaseStore } from '../state/caseStore';
import { es } from '../i18n/es';
import type { AppError } from '../types/domain';

export function HomePage() {
  const setScreen = useCaseStore((s) => s.setScreen);
  const setCurrentCase = useCaseStore((s) => s.setCurrentCase);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  async function handleOpen() {
    setError(null);
    try {
      const selected = await openDialog({ directory: true, title: es.home.chooseCaseFolder });
      if (!selected || typeof selected !== 'string') return;
      setBusy(true);
      const summary = await openCase(selected);
      setCurrentCase(summary);
      setScreen('workspace');
    } catch (e) {
      setError(normalizeAppError(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="container">
      <header>
        <h1>{es.app.title}</h1>
        <p className="subtitle">{es.app.subtitle}</p>
      </header>

      <section>
        <h2>{es.home.welcome}</h2>
        <div className="row">
          <button type="button" onClick={() => setScreen('createCase')} disabled={busy}>
            {es.home.createNew}
          </button>
          <button type="button" className="secondary" onClick={handleOpen} disabled={busy}>
            {busy ? '…' : es.home.openExisting}
          </button>
        </div>
        {error && (
          <p className="error">
            {es.errors.prefix} [{error.code}]: {error.message}
          </p>
        )}
      </section>
    </main>
  );
}
