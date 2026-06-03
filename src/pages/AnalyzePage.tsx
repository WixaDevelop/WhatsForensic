/**
 * Análisis de una evidencia: elige parser + modo, corre `analysis_run`,
 * muestra resumen y permite exportar XLSX.
 */

import { useCallback, useEffect, useState } from 'react';
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import {
  detectParsers,
  exportXlsx,
  listParsers,
  normalizeAppError,
  queryMessages,
  runAnalysis,
} from '../api/tauri';
import { useCaseStore } from '../state/caseStore';
import type {
  AnalysisRunSummary,
  AppError,
  OpenMode,
  PagedMessages,
  ParserDescriptor,
  ParserMatch,
} from '../types/domain';
import { Hash } from '../components/Hash';

const PAGE_SIZE = 50;

export function AnalyzePage() {
  const evidenceId = useCaseStore((s) => s.analyzingEvidenceId);
  const evidences = useCaseStore((s) => s.evidences);
  const setScreen = useCaseStore((s) => s.setScreen);

  const evidence = evidences.find((e) => e.evidenceId === evidenceId);

  const [parsers, setParsers] = useState<ParserDescriptor[]>([]);
  const [suggestions, setSuggestions] = useState<ParserMatch[]>([]);
  const [selectedParser, setSelectedParser] = useState<string>('');
  const [mode, setMode] = useState<OpenMode>('committed_only');
  const [summary, setSummary] = useState<AnalysisRunSummary | null>(null);
  const [messages, setMessages] = useState<PagedMessages | null>(null);
  const [filter, setFilter] = useState<string>('');
  const [page, setPage] = useState<number>(0);
  const [includeRaw, setIncludeRaw] = useState<boolean>(false);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState<boolean>(false);

  useEffect(() => {
    void listParsers().then(setParsers);
  }, []);

  const detectAvailable = useCallback(async () => {
    if (!evidenceId) return;
    setBusy(true);
    setError(null);
    try {
      const list = await detectParsers(evidenceId, mode);
      setSuggestions(list);
      if (list.length > 0 && !selectedParser) setSelectedParser(list[0].key);
    } catch (e) {
      setError(normalizeAppError(e));
    } finally {
      setBusy(false);
    }
  }, [evidenceId, mode, selectedParser]);

  useEffect(() => {
    void detectAvailable();
  }, [detectAvailable]);

  async function handleRun() {
    if (!evidenceId || !selectedParser) return;
    setBusy(true);
    setError(null);
    setMessages(null);
    try {
      const s = await runAnalysis(evidenceId, selectedParser, mode);
      setSummary(s);
      const msgs = await queryMessages(s.runId, 0, PAGE_SIZE);
      setMessages(msgs);
      setPage(0);
    } catch (e) {
      setError(normalizeAppError(e));
    } finally {
      setBusy(false);
    }
  }

  async function reloadMessages(p: number, q?: string) {
    if (!summary) return;
    setBusy(true);
    try {
      const msgs = await queryMessages(summary.runId, p, PAGE_SIZE, q);
      setMessages(msgs);
      setPage(p);
    } catch (e) {
      setError(normalizeAppError(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleExport() {
    if (!summary) return;
    setError(null);
    try {
      const target = await saveDialog({
        defaultPath: `reporte-${summary.parserKey}-${Date.now()}.xlsx`,
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      });
      if (!target) return;
      const result = await exportXlsx(summary.runId, target, includeRaw);
      alert(`Reporte guardado: ${result.outputPath} (${result.bytesWritten} bytes)`);
    } catch (e) {
      setError(normalizeAppError(e));
    }
  }

  if (!evidence) {
    return (
      <main className="container">
        <p>Sin evidencia seleccionada.</p>
        <button type="button" onClick={() => setScreen('workspace')}>
          Volver
        </button>
      </main>
    );
  }

  return (
    <main className="container">
      <header className="case-header">
        <div>
          <h1>Análisis de evidencia</h1>
          <p className="subtitle">
            {evidence.filename} · <code className="hash">{evidence.evidenceId.slice(0, 8)}</code>
          </p>
        </div>
        <button type="button" className="secondary" onClick={() => setScreen('workspace')}>
          Volver al workspace
        </button>
      </header>

      <section>
        <h2>Configuración</h2>
        <label>
          <span>Modo de apertura</span>
          <select value={mode} onChange={(e) => setMode(e.target.value as OpenMode)}>
            <option value="committed_only">Solo committed (immutable=1, sin WAL)</option>
            <option value="with_wal">Incluye WAL (puede modificar la working)</option>
          </select>
        </label>
        <label>
          <span>Parser</span>
          <select value={selectedParser} onChange={(e) => setSelectedParser(e.target.value)}>
            <option value="">— elegí un parser —</option>
            {parsers.map((p) => {
              const match = suggestions.find((s) => s.key === p.key);
              const tag = match ? ` (${match.confidence})` : '';
              return (
                <option key={p.key} value={p.key}>
                  {p.displayName}
                  {tag}
                </option>
              );
            })}
          </select>
        </label>
        <div className="row">
          <button type="button" onClick={handleRun} disabled={busy || !selectedParser}>
            {busy ? 'Procesando…' : 'Ejecutar análisis'}
          </button>
        </div>
      </section>

      {summary && (
        <section>
          <h2>Resumen</h2>
          {!summary.schemaVerified && (
            <p className="error">
              Aviso: el parser usó un mapping seed con verified=false. Las interpretaciones de tipo
              son orientativas.
            </p>
          )}
          <table className="kv">
            <tbody>
              <tr>
                <td>Run ID</td>
                <td>
                  <Hash value={summary.runId} />
                </td>
              </tr>
              <tr>
                <td>Fuente detectada</td>
                <td>{summary.sourceKind}</td>
              </tr>
              <tr>
                <td>Mapping</td>
                <td>{summary.schemaVersionUsed}</td>
              </tr>
              <tr>
                <td>Conversaciones</td>
                <td>{summary.conversationCount}</td>
              </tr>
              <tr>
                <td>Mensajes</td>
                <td>{summary.messageCount}</td>
              </tr>
              <tr>
                <td>Llamadas</td>
                <td>{summary.callCount}</td>
              </tr>
              <tr>
                <td>Señal de revocación</td>
                <td>{summary.revokedCount}</td>
              </tr>
              <tr>
                <td>Gaps detectados</td>
                <td>{summary.gapCount}</td>
              </tr>
              <tr>
                <td>Hallazgos A+B</td>
                <td>{summary.deletedHintCount}</td>
              </tr>
              <tr>
                <td>Warnings</td>
                <td>{summary.warningCount}</td>
              </tr>
            </tbody>
          </table>

          <div className="row" style={{ marginTop: '1rem' }}>
            <button type="button" onClick={handleExport}>
              Exportar XLSX
            </button>
            <label
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                marginBottom: 0,
              }}
            >
              <input
                type="checkbox"
                checked={includeRaw}
                onChange={(e) => setIncludeRaw(e.target.checked)}
              />
              <span style={{ marginBottom: 0 }}>Incluir raw_row JSON</span>
            </label>
          </div>
        </section>
      )}

      {messages && (
        <section>
          <h2>Mensajes ({messages.total})</h2>
          <div className="row inline" style={{ marginBottom: '1rem' }}>
            <input
              placeholder="Filtrar por texto / sender…"
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            />
            <button
              type="button"
              className="secondary"
              onClick={() => reloadMessages(0, filter || undefined)}
              disabled={busy}
            >
              Filtrar
            </button>
          </div>
          {messages.messages.length === 0 ? (
            <p className="muted">Sin resultados.</p>
          ) : (
            <table className="evidence-table">
              <thead>
                <tr>
                  <th>PK</th>
                  <th>Fecha UTC</th>
                  <th>Dirección</th>
                  <th>Sender</th>
                  <th>Tipo</th>
                  <th>Señal rev.</th>
                  <th>Cuerpo</th>
                </tr>
              </thead>
              <tbody>
                {messages.messages.map((m) => (
                  <tr key={m.id}>
                    <td className="mono small">{m.sourcePk}</td>
                    <td className="mono small">
                      {m.timestampUtc ? m.timestampUtc.replace('T', ' ').slice(0, 19) : '—'}
                    </td>
                    <td className="small">{m.direction}</td>
                    <td className="mono small">{m.sender ?? ''}</td>
                    <td className="small">
                      {m.messageTypeInterpreted ?? ''}
                      {!m.messageTypeVerified && ' *'}
                    </td>
                    <td>{m.isPossiblyRevoked ? 'Sí' : ''}</td>
                    <td>
                      {m.body ? (m.body.length > 80 ? m.body.slice(0, 80) + '…' : m.body) : '—'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          <div className="row" style={{ marginTop: '1rem' }}>
            <button
              type="button"
              className="secondary"
              onClick={() => reloadMessages(Math.max(0, page - 1), filter || undefined)}
              disabled={busy || page === 0}
            >
              ◀ Anterior
            </button>
            <span style={{ alignSelf: 'center' }}>
              Página {page + 1} / {Math.max(1, Math.ceil(messages.total / PAGE_SIZE))}
            </span>
            <button
              type="button"
              className="secondary"
              onClick={() => reloadMessages(page + 1, filter || undefined)}
              disabled={busy || (page + 1) * PAGE_SIZE >= messages.total}
            >
              Siguiente ▶
            </button>
          </div>
        </section>
      )}

      {error && (
        <p className="error">
          Error [{error.code}]: {error.message}
        </p>
      )}
    </main>
  );
}
