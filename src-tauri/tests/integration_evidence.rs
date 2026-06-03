//! Integration tests for the evidence ingest pipeline.
//!
//! Tests use synthetic SQLite databases created on the fly with `rusqlite`.
//! No real forensic evidence is ever read in tests.

use rusqlite::Connection;
use std::io::Write;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;
use whatsforensics_lib::evidence::{header, ingest};

#[tokio::test]
async fn validates_real_sqlite_header() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.sqlite");
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE t (id INTEGER);").unwrap();
    }

    let h = header::validate(&db_path).await.unwrap();
    assert!(h.valid_magic);
    assert_eq!(h.page_size, 4096); // SQLite default
    assert!(h.file_size > 0);
}

#[tokio::test]
async fn rejects_non_sqlite_in_header() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("notdb.bin");
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(&[0u8; 200]).unwrap();
    drop(f);

    let h = header::validate(&path).await.unwrap();
    assert!(!h.valid_magic);
}

#[tokio::test]
async fn preview_detects_wal_sidecar() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("with_wal.db");
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE t (id INTEGER);
             INSERT INTO t VALUES (1);",
        )
        .unwrap();
        // Leak: prevent close-on-drop so the WAL sidecar persists.
        std::mem::forget(conn);
    }

    let p = ingest::preview(&db_path).await.unwrap();
    assert!(p.header.valid_magic);
    assert!(p.sidecars.wal.is_some(), "WAL sidecar should be detected");
}

#[tokio::test]
async fn ingest_double_copy_with_hash_verification() {
    let dir = tempdir().unwrap();
    let src_path = dir.path().join("source.db");
    {
        let conn = Connection::open(&src_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE t (id INTEGER);
             INSERT INTO t VALUES (1), (2), (3);",
        )
        .unwrap();
    }

    let evidence_dir = dir.path().join("case-X/evidence/E1");
    let report = ingest::ingest(
        &src_path,
        "E1",
        &evidence_dir,
        CancellationToken::new(),
        |_, _| {},
    )
    .await
    .unwrap();

    // Hashes are equal between original / pristine / working.
    assert_eq!(report.original_sha256, report.pristine_sha256);
    assert_eq!(report.original_sha256, report.working_sha256);

    let pristine = evidence_dir.join("pristine").join("source.db");
    let working = evidence_dir.join("working").join("source.db");
    assert!(pristine.exists());
    assert!(working.exists());
    assert_eq!(report.evidence_id, "E1");
}

#[tokio::test]
async fn ingest_copies_wal_sidecar_to_both_copies() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("wal_src.db");
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE t (id INTEGER);
             INSERT INTO t VALUES (1);",
        )
        .unwrap();
        std::mem::forget(conn);
    }

    let evidence_dir = dir.path().join("E1");
    let report = ingest::ingest(
        &db_path,
        "E1",
        &evidence_dir,
        CancellationToken::new(),
        |_, _| {},
    )
    .await
    .unwrap();

    assert!(report.has_wal);
    assert!(evidence_dir
        .join("pristine")
        .join("wal_src.db-wal")
        .exists());
    assert!(evidence_dir.join("working").join("wal_src.db-wal").exists());
    assert!(!report.sidecar_hashes.is_empty());
}

#[tokio::test]
async fn ingest_progress_events_are_emitted() {
    let dir = tempdir().unwrap();
    let src_path = dir.path().join("p.db");
    {
        let conn = Connection::open(&src_path).unwrap();
        conn.execute_batch("CREATE TABLE t (id INTEGER);").unwrap();
    }

    let evidence_dir = dir.path().join("E1");
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_cb = counter.clone();
    let _ = ingest::ingest(
        &src_path,
        "E1",
        &evidence_dir,
        CancellationToken::new(),
        move |_step, _hp| {
            counter_cb.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        },
    )
    .await
    .unwrap();
    // 3 hashes (original + pristine + working) × at least (start + end) per hash.
    assert!(counter.load(std::sync::atomic::Ordering::SeqCst) >= 6);
}

#[tokio::test]
async fn ingest_honors_cancellation() {
    let dir = tempdir().unwrap();
    let src_path = dir.path().join("p.db");
    {
        let conn = Connection::open(&src_path).unwrap();
        conn.execute_batch("CREATE TABLE t (id INTEGER);").unwrap();
    }

    let evidence_dir = dir.path().join("E1");
    let cancel = CancellationToken::new();
    cancel.cancel();

    let result = ingest::ingest(&src_path, "E1", &evidence_dir, cancel, |_, _| {}).await;

    assert!(result.is_err());
    assert_eq!(result.err().unwrap().code, "CANCELED");
}
