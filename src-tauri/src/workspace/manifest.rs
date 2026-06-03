//! Manifest `case.json` con file locking cross-platform.
//!
//! TODO fase 1: lectura/escritura atómica con `fs2::FileExt::try_lock_exclusive`
//! para impedir dos instancias sobre el mismo caso. Schema versionado para
//! migración futura.
