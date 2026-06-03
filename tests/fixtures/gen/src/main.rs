//! Generador de fixtures SQLite sintéticos.
//!
//! Produce cuatro bases sintéticas que emulan patrones forensemente relevantes:
//!
//! 1. `mock-whatsapp-ios.sqlite`  — esquema estilo Core Data (Z_PK, ZWAMESSAGE,
//!    ZWACHATSESSION) con timestamps Mac Absolute.
//! 2. `mock-whatsapp-android.db`  — esquema estilo Android (tablas `chat`,
//!    `message`, `message_media`) con timestamps Unix milisegundos.
//! 3. `mock-with-wal.db`          — DB con filas en la principal Y filas en el
//!    WAL sin checkpoint, para probar el modo "incluye WAL".
//! 4. `mock-with-gaps.db`         — DB con discontinuidades en PKs
//!    autoincrementales, para probar la detección de gaps.
//!
//! **Importante:** los datos son completamente sintéticos. Ningún número de
//! teléfono, identificador o contenido corresponde a evidencia real.
//!
//! Uso:
//! ```text
//! cargo run -- [output_dir]
//! ```
//!
//! Default `output_dir`: `../output/` (relativo al crate).

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("../output"));

    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating output dir {}", out_dir.display()))?;

    let mut produced: Vec<PathBuf> = Vec::new();

    let p = out_dir.join("mock-whatsapp-ios.sqlite");
    generate_whatsapp_ios(&p)?;
    produced.push(p);

    let p = out_dir.join("mock-whatsapp-android.db");
    generate_whatsapp_android(&p)?;
    produced.push(p);

    let p = out_dir.join("mock-with-wal.db");
    generate_with_wal(&p)?;
    produced.push(p);

    let p = out_dir.join("mock-with-gaps.db");
    generate_with_gaps(&p)?;
    produced.push(p);

    println!("Generated {} synthetic fixtures:", produced.len());
    for p in produced {
        println!("  - {}", p.display());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Borra archivo y sus sidecars (-wal, -shm, -journal) si existen.
fn rm_with_sidecars(path: &Path) {
    let _ = std::fs::remove_file(path);
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        for suffix in ["-wal", "-shm", "-journal"] {
            let _ = std::fs::remove_file(dir.join(format!("{name}{suffix}")));
        }
    }
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

/// Base mínima al estilo iOS Core Data + WhatsApp.
fn generate_whatsapp_ios(path: &Path) -> Result<()> {
    rm_with_sidecars(path);
    let conn = Connection::open(path)?;

    conn.execute_batch(
        r#"
        CREATE TABLE Z_PRIMARYKEY (
            Z_ENT INTEGER PRIMARY KEY,
            Z_NAME VARCHAR,
            Z_SUPER INTEGER,
            Z_MAX INTEGER
        );

        CREATE TABLE Z_METADATA (
            Z_VERSION INTEGER PRIMARY KEY,
            Z_UUID VARCHAR(255),
            Z_PLIST BLOB
        );

        CREATE TABLE ZWACHATSESSION (
            Z_PK INTEGER PRIMARY KEY,
            Z_ENT INTEGER,
            Z_OPT INTEGER,
            ZSESSIONJID VARCHAR,
            ZPARTNERNAME VARCHAR,
            ZLASTMESSAGEDATE REAL
        );

        CREATE TABLE ZWAMESSAGE (
            Z_PK INTEGER PRIMARY KEY,
            Z_ENT INTEGER,
            Z_OPT INTEGER,
            ZCHATSESSION INTEGER,
            ZMESSAGETYPE INTEGER,
            ZISFROMME INTEGER,
            ZFROMJID VARCHAR,
            ZTOJID VARCHAR,
            ZTEXT VARCHAR,
            ZMESSAGEDATE REAL,
            FOREIGN KEY (ZCHATSESSION) REFERENCES ZWACHATSESSION(Z_PK)
        );
        "#,
    )?;

    // Mac Absolute timestamp: seconds since 2001-01-01 UTC.
    // 2024-01-15 12:00:00 UTC = 727_099_200 (Mac Absolute).
    let base_mac: f64 = 727_099_200.0;

    conn.execute(
        "INSERT INTO ZWACHATSESSION (Z_PK, Z_ENT, Z_OPT, ZSESSIONJID, ZPARTNERNAME, ZLASTMESSAGEDATE) \
         VALUES (1, 7, 5, ?, ?, ?)",
        params![
            "5491111111111@s.whatsapp.net",
            "Synthetic Contact A",
            base_mac + 300.0
        ],
    )?;

    for i in 1..=5i64 {
        let is_from_me = (i % 2 == 0) as i64;
        let (from, to) = if is_from_me == 1 {
            (
                "5490000000000@s.whatsapp.net",
                "5491111111111@s.whatsapp.net",
            )
        } else {
            (
                "5491111111111@s.whatsapp.net",
                "5490000000000@s.whatsapp.net",
            )
        };
        conn.execute(
            "INSERT INTO ZWAMESSAGE \
             (Z_PK, Z_ENT, Z_OPT, ZCHATSESSION, ZMESSAGETYPE, ZISFROMME, ZFROMJID, ZTOJID, ZTEXT, ZMESSAGEDATE) \
             VALUES (?, 9, 5, 1, 0, ?, ?, ?, ?, ?)",
            params![
                i,
                is_from_me,
                from,
                to,
                format!("Synthetic message {i}"),
                base_mac + (i as f64) * 60.0
            ],
        )?;
    }

    Ok(())
}

/// Base mínima al estilo WhatsApp Android (`msgstore.db`).
fn generate_whatsapp_android(path: &Path) -> Result<()> {
    rm_with_sidecars(path);
    let conn = Connection::open(path)?;

    conn.execute_batch(
        r#"
        CREATE TABLE chat (
            _id INTEGER PRIMARY KEY AUTOINCREMENT,
            jid VARCHAR,
            hidden INTEGER,
            subject VARCHAR
        );

        CREATE TABLE message (
            _id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_row_id INTEGER,
            key_id VARCHAR,
            key_remote_jid VARCHAR,
            key_from_me INTEGER,
            status INTEGER,
            data TEXT,
            timestamp INTEGER,
            message_type INTEGER,
            FOREIGN KEY (chat_row_id) REFERENCES chat(_id)
        );

        CREATE TABLE message_media (
            _id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_row_id INTEGER,
            mime_type VARCHAR,
            file_path VARCHAR,
            file_size INTEGER,
            FOREIGN KEY (message_row_id) REFERENCES message(_id)
        );
        "#,
    )?;

    conn.execute(
        "INSERT INTO chat (jid, hidden, subject) VALUES (?, 0, NULL)",
        params!["5491111111111@s.whatsapp.net"],
    )?;

    // Unix milliseconds. 2024-01-15 12:00:00 UTC = 1_705_320_000_000 ms.
    let base_ms: i64 = 1_705_320_000_000;

    for i in 1..=5i64 {
        conn.execute(
            "INSERT INTO message \
             (chat_row_id, key_id, key_remote_jid, key_from_me, status, data, timestamp, message_type) \
             VALUES (1, ?, ?, ?, 13, ?, ?, 0)",
            params![
                format!("KEYID{i:09X}"),
                "5491111111111@s.whatsapp.net",
                (i % 2 == 0) as i64,
                format!("Synthetic Android message {i}"),
                base_ms + i * 60_000
            ],
        )?;
    }

    Ok(())
}

/// Base con filas comprometidas en la principal Y filas que quedan en el WAL
/// sin checkpoint. Útil para validar la diferencia entre modos de apertura.
///
/// Importante: SQLite hace checkpoint final al cerrar la última conexión y luego
/// puede truncar/eliminar el WAL. Para que los archivos `-wal` y `-shm` queden
/// en disco con las filas no-checkpointed, **no** dejamos correr el destructor
/// de la conexión: usamos `std::mem::forget`. El handle del archivo se libera
/// igualmente cuando el proceso termina.
fn generate_with_wal(path: &Path) -> Result<()> {
    rm_with_sidecars(path);
    let conn = Connection::open(path)?;

    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA wal_autocheckpoint = 0;")?;
    conn.execute_batch(
        r#"
        CREATE TABLE items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            value TEXT,
            ts INTEGER
        );
        "#,
    )?;

    for i in 1..=3i64 {
        conn.execute(
            "INSERT INTO items (value, ts) VALUES (?, ?)",
            params![
                format!("Item {i} (committed to main)"),
                1_705_320_000 + i * 60
            ],
        )?;
    }
    // Checkpoint forzado para vaciar las filas 1..=3 a la DB principal.
    conn.query_row("PRAGMA wal_checkpoint(FULL);", [], |_| Ok(()))?;

    for i in 4..=6i64 {
        conn.execute(
            "INSERT INTO items (value, ts) VALUES (?, ?)",
            params![format!("Item {i} (still in WAL)"), 1_705_320_000 + i * 60],
        )?;
    }
    // NO checkpoint — las filas 4..=6 permanecen en el WAL.

    // Evitar el cierre limpio (que truncaría/eliminaría el WAL al ser el último
    // writer). Esto deja `-wal` y `-shm` en disco para los tests del modo
    // "incluye WAL". Es seguro: el SO libera el fd al terminar el proceso.
    std::mem::forget(conn);
    Ok(())
}

/// Base con gaps deliberados en una PK autoincremental. La detección de gaps
/// debería identificar los IDs faltantes (4, 7, 8).
fn generate_with_gaps(path: &Path) -> Result<()> {
    rm_with_sidecars(path);
    let conn = Connection::open(path)?;

    conn.execute_batch(
        r#"
        CREATE TABLE items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            label TEXT,
            ts INTEGER
        );
        "#,
    )?;

    let present: [i64; 7] = [1, 2, 3, 5, 6, 9, 10];
    for &id in &present {
        conn.execute(
            "INSERT INTO items (id, label, ts) VALUES (?, ?, ?)",
            params![id, format!("Item {id}"), 1_705_320_000 + id * 60],
        )?;
    }
    // Force sqlite_sequence to reflect the max so the gap analysis can
    // contrast `sqlite_sequence.seq` with observed PKs.
    conn.execute(
        "INSERT OR REPLACE INTO sqlite_sequence(name, seq) VALUES('items', 10)",
        [],
    )?;

    Ok(())
}
