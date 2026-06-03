//! Señales de eliminación/revocación. Categorías A y B **estrictamente separadas**.
//!
//! TODO fase 5:
//! - **A** — revocaciones declaradas por la app (códigos de tipo específicos,
//!   mapeados por TOML versionado).
//! - **B** — anomalías estructurales (cuerpo vacío + metadatos, FK rotas,
//!   filas presentes solo en WAL, tablas auxiliares).
//!
//! Cada hallazgo lleva `evidence_strength` ∈ {weak, moderate, strong} con
//! criterios documentados en `docs/METHODOLOGY.md` §5.
