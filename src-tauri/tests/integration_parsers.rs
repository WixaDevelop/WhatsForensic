//! Integration tests for the WhatsApp iOS and Android parsers, plus the
//! CallHistory iOS parser. Synthetic fixtures only — no real evidence.

use rusqlite::Connection;
use tempfile::tempdir;
use whatsforensics_lib::db::{introspect, opener, opener::OpenMode};
use whatsforensics_lib::parsers;
use whatsforensics_lib::parsers::common_model::MessageDirection;
use whatsforensics_lib::parsers::traits::{Confidence, Parser};

fn build_whatsapp_ios(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE Z_PRIMARYKEY (Z_ENT INTEGER PRIMARY KEY);
         CREATE TABLE ZWACHATSESSION (
            Z_PK INTEGER PRIMARY KEY,
            ZSESSIONJID VARCHAR,
            ZPARTNERNAME VARCHAR,
            ZLASTMESSAGEDATE REAL
         );
         CREATE TABLE ZWAMESSAGE (
            Z_PK INTEGER PRIMARY KEY,
            ZCHATSESSION INTEGER,
            ZMESSAGETYPE INTEGER,
            ZISFROMME INTEGER,
            ZFROMJID VARCHAR,
            ZTOJID VARCHAR,
            ZTEXT VARCHAR,
            ZMESSAGEDATE REAL
         );
         INSERT INTO ZWACHATSESSION VALUES (1, '5491111@s.whatsapp.net', 'Test A', 727099500.0);
         INSERT INTO ZWAMESSAGE VALUES (1, 1, 0, 0, '5491111@s.whatsapp.net', '5490000@s.whatsapp.net', 'hola', 727099200.0);
         INSERT INTO ZWAMESSAGE VALUES (2, 1, 0, 1, '5490000@s.whatsapp.net', '5491111@s.whatsapp.net', 'que tal', 727099260.0);
         INSERT INTO ZWAMESSAGE VALUES (3, 1, 7, 0, '5491111@s.whatsapp.net', '5490000@s.whatsapp.net', NULL, 727099320.0);",
    ).unwrap();
}

fn build_whatsapp_android(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE chat (_id INTEGER PRIMARY KEY AUTOINCREMENT, jid TEXT, hidden INTEGER, subject TEXT);
         CREATE TABLE message (
            _id INTEGER PRIMARY KEY AUTOINCREMENT,
            chat_row_id INTEGER, key_id TEXT, key_remote_jid TEXT,
            key_from_me INTEGER, status INTEGER, data TEXT, timestamp INTEGER, message_type INTEGER
         );
         CREATE TABLE message_media (_id INTEGER PRIMARY KEY AUTOINCREMENT, message_row_id INTEGER, mime_type TEXT, file_path TEXT, file_size INTEGER);
         INSERT INTO chat (jid, hidden, subject) VALUES ('5491111@s.whatsapp.net', 0, NULL);
         INSERT INTO message (chat_row_id, key_id, key_remote_jid, key_from_me, status, data, timestamp, message_type)
            VALUES (1, 'K1', '5491111@s.whatsapp.net', 0, 13, 'hola', 1705320000000, 0),
                   (1, 'K2', '5491111@s.whatsapp.net', 1, 13, 'qué tal', 1705320060000, 0),
                   (1, 'K3', '5491111@s.whatsapp.net', 0, 13, NULL, 1705320120000, 15);",
    ).unwrap();
}

fn build_callhistory(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE ZCALLRECORD (
            Z_PK INTEGER PRIMARY KEY, ZDATE REAL, ZDURATION INTEGER,
            ZORIGINATED INTEGER, ZCALLTYPE INTEGER, ZADDRESS TEXT
         );
         INSERT INTO ZCALLRECORD VALUES (1, 727099200.0, 60, 1, 0, '+5491111111111');
         INSERT INTO ZCALLRECORD VALUES (2, 727099500.0, 0, 0, 4, '+5491111111111');",
    )
    .unwrap();
}

#[tokio::test]
async fn whatsapp_ios_parser_runs_against_synthetic_fixture() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ChatStorage.sqlite");
    build_whatsapp_ios(&path);

    let conn = opener::open(&path, OpenMode::CommittedOnly).await.unwrap();
    let parser = parsers::whatsapp_ios::WhatsAppIos::new().unwrap();
    let parsed = parser.parse(&conn).unwrap();
    assert_eq!(parsed.source_kind, "whatsapp_ios");
    assert_eq!(parsed.conversations.len(), 1);
    assert_eq!(parsed.messages.len(), 3);
    // Third message is type 7 = revoked per seed mapping.
    let revoked: Vec<_> = parsed
        .messages
        .iter()
        .filter(|m| m.is_possibly_revoked)
        .collect();
    assert_eq!(revoked.len(), 1);
    // Outgoing direction mapped from is_from_me=1
    let outgoing: Vec<_> = parsed
        .messages
        .iter()
        .filter(|m| matches!(m.direction, MessageDirection::Outgoing))
        .collect();
    assert_eq!(outgoing.len(), 1);
}

#[tokio::test]
async fn whatsapp_ios_parser_detection() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ChatStorage.sqlite");
    build_whatsapp_ios(&path);
    let conn = opener::open(&path, OpenMode::CommittedOnly).await.unwrap();
    let schema = introspect::snapshot(&conn).unwrap();
    let parser = parsers::whatsapp_ios::WhatsAppIos::new().unwrap();
    let confidence = parser.detect("ChatStorage.sqlite", &schema);
    assert!(matches!(confidence, Confidence::High));
}

#[tokio::test]
async fn whatsapp_android_parser_runs_against_synthetic_fixture() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("msgstore.db");
    build_whatsapp_android(&path);

    let conn = opener::open(&path, OpenMode::CommittedOnly).await.unwrap();
    let parser = parsers::whatsapp_android::WhatsAppAndroid::new().unwrap();
    let parsed = parser.parse(&conn).unwrap();
    assert_eq!(parsed.source_kind, "whatsapp_android");
    assert_eq!(parsed.conversations.len(), 1);
    assert_eq!(parsed.messages.len(), 3);
    // Message_type=15 maps to "deleted" with revocation_codes [15].
    let revoked: Vec<_> = parsed
        .messages
        .iter()
        .filter(|m| m.is_possibly_revoked)
        .collect();
    assert_eq!(revoked.len(), 1);
}

#[tokio::test]
async fn callhistory_parser_runs_against_synthetic_fixture() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("CallHistory.storedata");
    build_callhistory(&path);

    let conn = opener::open(&path, OpenMode::CommittedOnly).await.unwrap();
    let parser = parsers::callhistory_ios::CallHistoryIos::new().unwrap();
    let parsed = parser.parse(&conn).unwrap();
    assert_eq!(parsed.source_kind, "callhistory_ios");
    assert_eq!(parsed.calls.len(), 2);
    // call_type=4 → facetime_video per seed.
    let ft = parsed
        .calls
        .iter()
        .find(|c| c.call_type_raw == Some(4))
        .unwrap();
    assert_eq!(ft.call_type_interpreted.as_deref(), Some("facetime_video"));
}

#[tokio::test]
async fn detection_orders_by_confidence() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("ChatStorage.sqlite");
    build_whatsapp_ios(&path);
    let conn = opener::open(&path, OpenMode::CommittedOnly).await.unwrap();
    let schema = introspect::snapshot(&conn).unwrap();

    let parsers_vec = parsers::all_parsers().unwrap();
    let refs: Vec<&dyn parsers::traits::Parser> = parsers_vec.iter().map(|b| b.as_ref()).collect();
    let suggestions = parsers::source_detect::suggest("ChatStorage.sqlite", &schema, &refs);
    assert!(!suggestions.is_empty());
    assert_eq!(suggestions[0].key, "whatsapp_ios");
    assert!(matches!(suggestions[0].confidence, Confidence::High));
}
