//! Validación del header SQLite y lectura de page_size.
//!
//! TODO fase 1: verificar magic bytes `"SQLite format 3\0"` (16 bytes) y
//! extraer `page_size` del offset 16–17 (big endian). No abre con SQLite.
