//! Conversión y detección de timestamps.
//!
//! - **Mac Absolute / Cocoa**: segundos (puede ser float) desde 2001-01-01 UTC.
//!   Epoch Unix correspondiente: 978_307_200.
//! - **Unix segundos**: típicamente `< 2e9`.
//! - **Unix milisegundos**: típicamente `> 1e12`.
//!
//! La heurística de detección se aplica por valor pero el formato real depende
//! del contexto (la columna SQLite). El parser de cada fuente debería declarar
//! qué formato espera por columna; la heurística sirve solo de fallback.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// Constante: segundos entre Unix epoch (1970-01-01) y Mac Absolute epoch (2001-01-01).
pub const MAC_ABSOLUTE_EPOCH_OFFSET: i64 = 978_307_200;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimestampFormat {
    MacAbsolute,
    UnixS,
    UnixMs,
    Unknown,
}

/// Convierte un valor crudo al `DateTime<Utc>` correspondiente, dado un formato
/// declarado.
pub fn convert(raw: f64, format: TimestampFormat) -> Option<DateTime<Utc>> {
    match format {
        TimestampFormat::MacAbsolute => {
            let unix_secs = raw as i64 + MAC_ABSOLUTE_EPOCH_OFFSET;
            Utc.timestamp_opt(unix_secs, 0).single()
        }
        TimestampFormat::UnixS => Utc.timestamp_opt(raw as i64, 0).single(),
        TimestampFormat::UnixMs => {
            let secs = (raw as i64) / 1000;
            let nanos = ((raw as i64) % 1000) as u32 * 1_000_000;
            Utc.timestamp_opt(secs, nanos).single()
        }
        TimestampFormat::Unknown => None,
    }
}

/// Heurística de detección a partir del valor crudo.
///
/// Rangos (excluyentes en orden):
/// - `> 1e12` → UnixMs (muy confiable)
/// - `> 1e9`  → UnixS (Unix-s para fechas >= 2001)
/// - `> 1e8`  → MacAbsolute (Mac Absolute para fechas >= ~2004)
/// - resto    → Unknown
///
/// Esta heurística es ambigua cuando el valor cae en zona límite (ej. Mac
/// Absolute de 1971 vs Unix-s de 2031). En la práctica el parser debe declarar
/// el formato esperado por columna; la heurística es sólo fallback.
pub fn detect_format(raw: f64) -> TimestampFormat {
    if raw > 1e12 {
        TimestampFormat::UnixMs
    } else if raw > 1e9 {
        TimestampFormat::UnixS
    } else if raw > 1e8 {
        TimestampFormat::MacAbsolute
    } else {
        TimestampFormat::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn naive(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, mi, s)
            .unwrap()
    }

    #[test]
    fn mac_absolute_conversion() {
        // Mac Absolute 0.0 = 2001-01-01T00:00:00Z
        let dt = convert(0.0, TimestampFormat::MacAbsolute).unwrap();
        assert_eq!(dt.naive_utc(), naive(2001, 1, 1, 0, 0, 0));
        // 2024-01-15T12:00:00Z = Mac Absolute 727_012_800
        let dt2 = convert(727_012_800.0, TimestampFormat::MacAbsolute).unwrap();
        assert_eq!(dt2.naive_utc(), naive(2024, 1, 15, 12, 0, 0));
    }

    #[test]
    fn unix_ms_conversion() {
        let dt = convert(1_705_320_000_000.0, TimestampFormat::UnixMs).unwrap();
        assert_eq!(dt.naive_utc(), naive(2024, 1, 15, 12, 0, 0));
    }

    #[test]
    fn detects_formats() {
        assert_eq!(detect_format(1_705_320_000_000.0), TimestampFormat::UnixMs);
        assert_eq!(detect_format(1_705_320_000.0), TimestampFormat::UnixS);
        assert_eq!(detect_format(727_012_800.0), TimestampFormat::MacAbsolute);
        assert_eq!(detect_format(0.0), TimestampFormat::Unknown);
    }
}
