/**
 * Formulario de creación de caso.
 */

import { useEffect, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { createCase, getDefaultWorkspaceRoot, normalizeAppError } from '../api/tauri';
import { useCaseStore } from '../state/caseStore';
import { es } from '../i18n/es';
import type { AppError } from '../types/domain';

export function CaseSetupPage() {
  const setScreen = useCaseStore((s) => s.setScreen);
  const setCurrentCase = useCaseStore((s) => s.setCurrentCase);

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [investigator, setInvestigator] = useState('');
  const [timezone, setTimezone] = useState('America/Argentina/Buenos_Aires');
  const [workspaceRoot, setWorkspaceRoot] = useState('');
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void getDefaultWorkspaceRoot().then((p) => {
      if (p) setWorkspaceRoot(p);
    });
  }, []);

  async function pickWorkspaceRoot() {
    const selected = await openDialog({ directory: true });
    if (selected && typeof selected === 'string') {
      setWorkspaceRoot(selected);
    }
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const summary = await createCase({
        name,
        description: description.trim() || null,
        investigator,
        timezone,
        workspaceRoot,
      });
      setCurrentCase(summary);
      setScreen('workspace');
    } catch (err) {
      setError(normalizeAppError(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main className="container">
      <header>
        <h1>{es.caseSetup.title}</h1>
      </header>

      <section>
        <form onSubmit={submit}>
          <label>
            <span>{es.caseSetup.name}</span>
            <input
              required
              value={name}
              placeholder={es.caseSetup.namePlaceholder}
              onChange={(e) => setName(e.target.value)}
            />
          </label>

          <label>
            <span>{es.caseSetup.description}</span>
            <textarea
              value={description}
              placeholder={es.caseSetup.descriptionPlaceholder}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
            />
          </label>

          <label>
            <span>{es.caseSetup.investigator}</span>
            <input
              required
              value={investigator}
              placeholder={es.caseSetup.investigatorPlaceholder}
              onChange={(e) => setInvestigator(e.target.value)}
            />
          </label>

          <label>
            <span>{es.caseSetup.timezone}</span>
            <input required value={timezone} onChange={(e) => setTimezone(e.target.value)} />
            <small>{es.caseSetup.timezoneHint}</small>
          </label>

          <label>
            <span>{es.caseSetup.workspaceRoot}</span>
            <div className="row inline">
              <input
                required
                value={workspaceRoot}
                onChange={(e) => setWorkspaceRoot(e.target.value)}
                placeholder="/srv/workspaces"
              />
              <button type="button" className="secondary" onClick={pickWorkspaceRoot}>
                {es.caseSetup.pickWorkspace}
              </button>
            </div>
            <small>{es.caseSetup.workspaceRootHint}</small>
          </label>

          <div className="row">
            <button type="submit" disabled={busy}>
              {busy ? '…' : es.caseSetup.createButton}
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => setScreen('home')}
              disabled={busy}
            >
              {es.caseSetup.cancel}
            </button>
          </div>

          {error && (
            <p className="error">
              {es.errors.prefix} [{error.code}]: {error.message}
            </p>
          )}
        </form>
      </section>
    </main>
  );
}
