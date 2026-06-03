import { useState } from 'react';
import { getSystemInfo, normalizeAppError, runProgressDemo } from './api/tauri';
import type { AppError, SystemInfo } from './types/domain';
import { es } from './i18n/es';
import './App.css';

function App() {
  const [info, setInfo] = useState<SystemInfo | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<AppError | null>(null);

  async function handleSystemCheck() {
    setError(null);
    try {
      setInfo(await getSystemInfo());
    } catch (e) {
      setError(normalizeAppError(e));
    }
  }

  async function handleProgressDemo() {
    setError(null);
    setProgress(0);
    try {
      await runProgressDemo((evt) => setProgress(evt.percent));
    } catch (e) {
      setError(normalizeAppError(e));
    }
  }

  return (
    <main className="container">
      <header>
        <h1>{es.app.title}</h1>
        <p className="subtitle">{es.app.subtitle}</p>
        <span className="phase">{es.app.phase}</span>
      </header>

      <section>
        <h2>{es.system.sectionTitle}</h2>
        <button type="button" onClick={handleSystemCheck}>
          {es.system.requestInfo}
        </button>
        {info && (
          <table>
            <tbody>
              <tr>
                <td>{es.system.fields.toolName}</td>
                <td>{info.toolName}</td>
              </tr>
              <tr>
                <td>{es.system.fields.toolVersion}</td>
                <td>{info.toolVersion}</td>
              </tr>
              <tr>
                <td>{es.system.fields.rustEdition}</td>
                <td>{info.rustEdition}</td>
              </tr>
              <tr>
                <td>{es.system.fields.targetOs}</td>
                <td>{info.targetOs}</td>
              </tr>
              <tr>
                <td>{es.system.fields.targetArch}</td>
                <td>{info.targetArch}</td>
              </tr>
            </tbody>
          </table>
        )}
      </section>

      <section>
        <h2>{es.progress.sectionTitle}</h2>
        <button type="button" onClick={handleProgressDemo}>
          {es.progress.runDemo}
        </button>
        {progress !== null && (
          <p className="progress-line">
            {es.progress.currentLabel}: {progress}%
          </p>
        )}
      </section>

      {error && (
        <p className="error">
          {es.errors.prefix} [{error.code}]: {error.message}
        </p>
      )}
    </main>
  );
}

export default App;
