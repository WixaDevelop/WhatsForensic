//! Capa `report` — generación XLSX con `rust_xlsxwriter`.
//!
//! El reporte es bit-identical sobre la misma entrada y la misma versión de
//! herramienta. Embedde versión + dependencias críticas para reproducibilidad.
//!
//! Hojas en el orden definido en `docs/ARCHITECTURE.md` §10.

pub mod charts;
pub mod sheets;
pub mod xlsx_writer;
