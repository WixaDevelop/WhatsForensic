//! Integration tests for the safe SQLite access layer.
//!
//! Empirically verifies the behavior of `immutable=1`: in `CommittedOnly` mode,
//! rows that are only in the WAL must **not** be visible. In `WithWal` mode,
//! they must be visible. This corresponds to docs/ARCHITECTURE.md §17 and
//! docs/METHODOLOGY.md §3.

use rusqlite::Connection;
use tempfile::tempdir;
use whatsforensics_lib::db::{introspect, opener, opener::OpenMode, safe_query};

/// Construye una DB con 3 filas committed a la principal y 3 filas que quedan
/// en el WAL sin checkpoint.
fn build_wal_fixture(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA wal_autocheckpoint = 0;
         CREATE TABLE t (id INTEGER PRIMARY KEY AUTOINCREMENT, label TEXT);
         INSERT INTO t (label) VALUES ('A'), ('B'), ('C');",
    )
    .unwrap();
    conn.query_row("PRAGMA wal_checkpoint(FULL);", [], |_| Ok(()))
        .unwrap();
    conn.execute_batch("INSERT INTO t (label) VALUES ('D'), ('E'), ('F');")
        .unwrap();
    std::mem::forget(conn);
}

#[tokio::test]
async fn committed_only_hides_wal_rows() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal.db");
    build_wal_fixture(&path);

    let conn = opener::open(&path, OpenMode::CommittedOnly).await.unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
        .unwrap();
    // En el modo committed_only, las filas D, E, F (en el WAL) no son visibles.
    assert_eq!(
        count, 3,
        "Solo deben verse A, B, C (committed a la principal)"
    );
}

#[tokio::test]
async fn with_wal_shows_all_rows() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wal.db");
    build_wal_fixture(&path);

    let conn = opener::open(&path, OpenMode::WithWal).await.unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 6, "Deben verse A..F (main + WAL)");
}

#[tokio::test]
async fn introspect_returns_tables_and_columns() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("schema.db");
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE messages (
                id INTEGER PRIMARY KEY,
                body TEXT NOT NULL,
                ts INTEGER
            );
            CREATE INDEX idx_messages_ts ON messages(ts);
            INSERT INTO messages (id, body, ts) VALUES (1, 'hi', 100), (2, 'bye', 200);",
        )
        .unwrap();
    }

    let conn = opener::open(&path, OpenMode::CommittedOnly).await.unwrap();
    let snapshot = introspect::snapshot(&conn).unwrap();
    let table = snapshot.tables.get("messages").expect("messages table");
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.columns[0].name, "id");
    assert!(table.columns[0].pk);
    assert!(table.columns[1].notnull);
    assert_eq!(table.row_count, 2);
    assert!(snapshot.indexes.iter().any(|i| i.name == "idx_messages_ts"));
}

#[tokio::test]
async fn rejects_invalid_sqlite_header() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("notdb.bin");
    std::fs::write(&path, vec![0u8; 200]).unwrap();
    let r = opener::open(&path, OpenMode::CommittedOnly).await;
    assert!(r.is_err());
    assert_eq!(r.err().unwrap().code, "INVALID_SQLITE_HEADER");
}

#[test]
fn safe_query_accepts_select_pragma_with() {
    assert!(safe_query::validate("SELECT * FROM t").is_ok());
    assert!(safe_query::validate("PRAGMA table_info(t)").is_ok());
    assert!(safe_query::validate("WITH x AS (SELECT 1) SELECT * FROM x").is_ok());
}

#[test]
fn safe_query_rejects_mutations_and_multi() {
    assert!(safe_query::validate("INSERT INTO t VALUES (1)").is_err());
    assert!(safe_query::validate("DROP TABLE t").is_err());
    assert!(safe_query::validate("DELETE FROM t").is_err());
    assert!(safe_query::validate("UPDATE t SET a=1").is_err());
    assert!(safe_query::validate("SELECT 1; SELECT 2").is_err());
}
