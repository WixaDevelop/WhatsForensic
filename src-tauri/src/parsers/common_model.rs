//! Modelo común al que cada parser traduce su fuente.
//!
//! TODO fase 3: estructs `Conversation`, `Message`, `Call`, `Attachment`,
//! `SchemaSnapshot`. Cada `Message`/`Call` lleva `raw_row` (fila SQLite original
//! serializada) — esencial forense, no eliminar para "ahorrar espacio".
