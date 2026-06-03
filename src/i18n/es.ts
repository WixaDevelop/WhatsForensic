/**
 * Diccionario de textos en español. Centralizado desde el inicio aunque haya
 * un único idioma, para evitar refactor masivo cuando se agregue otro.
 *
 * Convención: claves anidadas por sección (`app.title`, `errors.io`, ...).
 */

export const es = {
  app: {
    title: 'WhatsForensics',
    subtitle: 'Análisis forense de bases SQLite móviles',
    phase: 'Fase 0 — Fundaciones',
  },
  system: {
    sectionTitle: 'Verificación del backend',
    requestInfo: 'Solicitar información del sistema',
    fields: {
      toolName: 'Herramienta',
      toolVersion: 'Versión',
      rustEdition: 'Edición Rust',
      targetOs: 'Sistema operativo',
      targetArch: 'Arquitectura',
    },
  },
  progress: {
    sectionTitle: 'Patrón de progreso (Channel)',
    runDemo: 'Ejecutar demo',
    currentLabel: 'Progreso',
  },
  errors: {
    prefix: 'Error',
  },
} as const;
