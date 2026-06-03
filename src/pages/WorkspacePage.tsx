/**
 * Workspace del caso abierto. Muestra metadata + lista de evidencias + ingesta.
 */

import { useCallback, useEffect, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  closeCase,
  ingestEvidence,
  listEvidence,
  normalizeAppError,
  previewEvidence,
} from '../api/tauri';
import { useCaseStore } from '../state/caseStore';
import { es } from '../i18n/es';
import type { AppError, EvidencePreview, HashProgress, IngestStep } from '../types/domain';
import { Bytes } from '../components/Bytes';
import { Hash } from '../components/Hash';
import { formatBytes } from '../utils/format';

export function WorkspacePage() {
  const currentCase = useCaseStore((s) => s.currentCase);
  const evidences = useCaseStore((s) => s.evidences);
  const setEvidences = useCaseStore((s) => s.setEvidences);
  const setScreen = useCaseStore((s) => s.setScreen);
  const setCurrentCase = useCaseStore((s) => s.setCurrentCase);

  const [preview, setPreview] = useState<EvidencePreview | null>(null);
  const [declaredType, setDeclaredType] = useState('');
  const [progress, setProgress] = useState<{ step: IngestStep; hp: HashProgress } | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const es = await listEvidence();
      setEvidences(es);
    } catch (err) {
      setError(normalizeAppError(err));
    }
  }, [setEvidences]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function pickAndPreview() {
    setError(null);
    try {
      const selected = await openDialog({
        multiple: false,
        filters: [
          {
            name: 'SQLite / forensic',
            extensions: ['db', 'sqlite', 'sqlite3', 'storedata'],
          },
          { name: 'Todos', extensions: ['*'] },
        ],
      });
      if (!selected || typeof selected !== 'string') return;
      setBusy(true);
      const p = await previewEvidence(selected);
      setPreview(p);
    } catch (err) {
      setError(normalizeAppError(err));
    } finally {
      setBusy(false);
    }
  }

  async function confirmIngest() {
    if (!preview) return;
    setError(null);
    setBusy(true);
    setProgress(null);
    try {
      await ingestEvidence(preview.originalPath, declaredType.trim() || null, (evt) =>
        setProgress({ step: evt.step, hp: evt.progress }),
      );
      setPreview(null);
      setDeclaredType('');
      setProgress(null);
      await refresh();
    } catch (err) {
      setError(normalizeAppError(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleClose() {
    await closeCase();
    setCurrentCase(null);
    setEvidences([]);
    setScreen('home');
  }

  if (!currentCase) {
    return null;
  }

  const sidecarBadges = (e: { hasWal: boolean; hasShm: boolean; hasJournal: boolean }) =>
    [e.hasWal && 'WAL', e.hasShm && 'SHM', e.hasJournal && 'JNL'].filter(Boolean).join(', ') ||
    es.common.none;

  return (
    <main className="container">
      <header className="case-header">
        <div>
          <h1>{currentCase.name}</h1>
          <p className="subtitle">
            {currentCase.investigator} · {currentCase.timezone} ·{' '}
            <code className="hash" title={currentCase.caseId}>
              {currentCase.caseId.slice(0, 8)}
            </code>
          </p>
        </div>
        <button type="button" className="secondary" onClick={handleClose}>
          {es.workspace.close}
        </button>
      </header>

      <section>
        <div className="section-head">
          <h2>{es.workspace.sectionEvidence}</h2>
          <button type="button" onClick={pickAndPreview} disabled={busy}>
            {es.workspace.addEvidence}
          </button>
        </div>

        {evidences.length === 0 ? (
          <p className="muted">{es.workspace.noEvidence}</p>
        ) : (
          <table className="evidence-table">
            <thead>
              <tr>
                <th>{es.workspace.columns.filename}</th>
                <th>{es.workspace.columns.size}</th>
                <th>{es.workspace.columns.sha256}</th>
                <th>{es.workspace.columns.sidecars}</th>
                <th>{es.workspace.columns.ingestedAt}</th>
              </tr>
            </thead>
            <tbody>
              {evidences.map((e) => (
                <tr key={e.evidenceId}>
                  <td>{e.filename}</td>
                  <td>
                    <Bytes value={e.originalSize} />
                  </td>
                  <td>
                    <Hash value={e.pristineSha256} />
                  </td>
                  <td className="mono small">{sidecarBadges(e)}</td>
                  <td className="mono small">
                    {new Date(e.ingestedAt).toISOString().replace('T', ' ').slice(0, 19)}Z
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {preview && (
        <section className="preview-card">
          <h2>{es.ingest.title}</h2>
          <table className="kv">
            <tbody>
              <tr>
                <td>Path</td>
                <td className="mono small">{preview.originalPath}</td>
              </tr>
              <tr>
                <td>{es.ingest.fileSize}</td>
                <td>
                  <Bytes value={preview.size} />
                </td>
              </tr>
              <tr>
                <td>Header</td>
                <td>
                  {preview.header.validMagic ? (
                    <span className="ok">{es.ingest.headerValid}</span>
                  ) : (
                    <span className="warn">{es.ingest.headerInvalid}</span>
                  )}
                </td>
              </tr>
              <tr>
                <td>{es.ingest.pageSize}</td>
                <td className="mono">{preview.header.pageSize}</td>
              </tr>
              <tr>
                <td>{es.ingest.sidecarsDetected}</td>
                <td className="mono small">
                  {[
                    preview.sidecars.wal && 'WAL',
                    preview.sidecars.shm && 'SHM',
                    preview.sidecars.journal && 'JOURNAL',
                  ]
                    .filter(Boolean)
                    .join(', ') || es.common.none}
                </td>
              </tr>
            </tbody>
          </table>

          <label>
            <span>{es.ingest.declaredType}</span>
            <input
              value={declaredType}
              onChange={(e) => setDeclaredType(e.target.value)}
              placeholder={es.ingest.declaredTypePlaceholder}
            />
          </label>

          <div className="row">
            <button
              type="button"
              onClick={confirmIngest}
              disabled={busy || !preview.header.validMagic}
            >
              {busy ? es.ingest.inProgress : es.ingest.confirmIngest}
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => {
                setPreview(null);
                setDeclaredType('');
              }}
              disabled={busy}
            >
              {es.ingest.cancel}
            </button>
          </div>

          {progress && (
            <div className="progress-block">
              <p className="muted small">
                {es.ingest.stepLabels[progress.step]} — {formatBytes(progress.hp.bytesDone)} /{' '}
                {formatBytes(progress.hp.bytesTotal)}
              </p>
              <div className="progress-bar">
                <div className="progress-bar-fill" style={{ width: `${progress.hp.percent}%` }} />
              </div>
              <p className="progress-line">{progress.hp.percent}%</p>
            </div>
          )}
        </section>
      )}

      {error && (
        <p className="error">
          {es.errors.prefix} [{error.code}]: {error.message}
        </p>
      )}
    </main>
  );
}
