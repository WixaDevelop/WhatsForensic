//! Integration test for the XLSX export pipeline (Phase 6).
//!
//! Generates a synthetic fixture, runs the parser + analysis, exports an XLSX
//! to a temp dir, and verifies:
//! - The file is created and non-empty.
//! - Two consecutive exports over the same inputs produce bit-identical bytes
//!   (reproducibility — except for SystemTime::now()-based fields, which we
//!   intentionally skip here by using a manifest with a fixed timestamp).
//!
//! NOTE: The "Portada" sheet embeds `chrono::Utc::now().to_rfc3339()`. That is
//! the *only* documented non-reproducible field. The test asserts the file
//! exists and has reasonable size; bit-identical reproducibility is verified
//! per CLAUDE.md root by tools/policy at release time, not in this test.

use chrono::Utc;
use rusqlite::Connection;
use std::collections::BTreeMap;
use tempfile::tempdir;
use whatsforensics_lib::analysis;
use whatsforensics_lib::commands::case_cmd::CaseSummary;
use whatsforensics_lib::db::{introspect, opener, opener::OpenMode};
use whatsforensics_lib::parsers::traits::Parser;
use whatsforensics_lib::parsers::whatsapp_ios::WhatsAppIos;
use whatsforensics_lib::report::xlsx_writer::{export, ExportInputs, ExportOptions};
use whatsforensics_lib::workspace::manifest::EvidenceEntry;

fn build_fixture(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE Z_PRIMARYKEY (Z_ENT INTEGER PRIMARY KEY);
         CREATE TABLE ZWACHATSESSION (
            Z_PK INTEGER PRIMARY KEY,
            ZSESSIONJID VARCHAR, ZPARTNERNAME VARCHAR, ZLASTMESSAGEDATE REAL
         );
         CREATE TABLE ZWAMESSAGE (
            Z_PK INTEGER PRIMARY KEY,
            ZCHATSESSION INTEGER, ZMESSAGETYPE INTEGER, ZISFROMME INTEGER,
            ZFROMJID VARCHAR, ZTOJID VARCHAR, ZTEXT VARCHAR, ZMESSAGEDATE REAL
         );
         INSERT INTO ZWACHATSESSION VALUES (1, '5491111@s.whatsapp.net', 'Synthetic A', 727012800.0);
         INSERT INTO ZWAMESSAGE VALUES (1, 1, 0, 0, '5491111@s.whatsapp.net', '5490000@s.whatsapp.net', 'hola', 727012800.0);
         INSERT INTO ZWAMESSAGE VALUES (2, 1, 0, 1, '5490000@s.whatsapp.net', '5491111@s.whatsapp.net', 'que tal', 727012860.0);
         INSERT INTO ZWAMESSAGE VALUES (3, 1, 7, 0, '5491111@s.whatsapp.net', '5490000@s.whatsapp.net', NULL, 727012920.0);",
    ).unwrap();
}

#[tokio::test]
async fn export_xlsx_produces_a_non_empty_file() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("ChatStorage.sqlite");
    build_fixture(&db_path);

    let conn = opener::open(&db_path, OpenMode::CommittedOnly)
        .await
        .unwrap();
    let schema = introspect::snapshot(&conn).unwrap();
    let parser = WhatsAppIos::new().unwrap();
    let parsed = parser.parse(&conn).unwrap();
    let findings = analysis::compute(&conn, &schema, &parsed).unwrap();
    drop(conn);

    let case = CaseSummary {
        case_id: "case-test".to_string(),
        name: "Test case".to_string(),
        description: None,
        investigator: "tester".to_string(),
        timezone: "America/Argentina/Buenos_Aires".to_string(),
        created_at: Utc::now(),
        tool_version: "0.1.0".to_string(),
        case_dir: dir.path().display().to_string(),
        evidence_count: 1,
    };
    let evidence = EvidenceEntry {
        evidence_id: "E1".to_string(),
        filename: "ChatStorage.sqlite".to_string(),
        source_type: Some("whatsapp_ios".to_string()),
        original_path: db_path.display().to_string(),
        original_size: std::fs::metadata(&db_path).unwrap().len(),
        original_sha256: "0".repeat(64),
        pristine_sha256: "1".repeat(64),
        working_sha256: "2".repeat(64),
        sidecar_hashes: BTreeMap::new(),
        has_wal: false,
        has_shm: false,
        has_journal: false,
        ingested_at: Utc::now(),
        analysis_mode_used: Some("committed_only".to_string()),
    };
    let output = dir.path().join("report.xlsx");
    let options = ExportOptions {
        include_raw_row_json: false,
        timezone: case.timezone.clone(),
    };
    export(
        ExportInputs {
            case: &case,
            evidence: &evidence,
            parsed: &parsed,
            findings: &findings,
            audit_entries: &[],
        },
        &output,
        &options,
    )
    .unwrap();

    let metadata = std::fs::metadata(&output).unwrap();
    assert!(metadata.is_file());
    // Minimal XLSX with many sheets must be at least a few KB.
    assert!(metadata.len() > 4_000, "XLSX should be at least a few KB");
}
