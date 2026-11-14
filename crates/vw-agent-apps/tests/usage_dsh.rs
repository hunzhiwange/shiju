//! Deferred validation: these tests were authored but not executed in task 28.
use std::{fs::{File, OpenOptions}, io::Write};
use rusqlite::Connection;
use serde_json::json;
use vw_agent_apps::usage::{
    cursors::{UsageCursorKey, initialize_usage_cursors, load_usage_cursor},
    dsh::scan_dsh_file,
    store::{StoredUsageRecord, initialize_usage_store},
};
use vw_api_types::id::{ProjectId, ProjectScope};

fn open(path: &std::path::Path) -> Connection {
    let mut db = Connection::open(path).unwrap();
    initialize_usage_store(&mut db).unwrap();
    initialize_usage_cursors(&db).unwrap();
    db
}
fn source() -> UsageCursorKey {
    UsageCursorKey { target_host: "host".into(), app: "dsh".into(), instance_id: "shared-home".into(), file_identity: "session-log".into() }
}
fn scope() -> ProjectScope {
    ProjectScope::for_default_tenant(ProjectId("project".into())).unwrap()
}
fn start() -> String {
    format!("{}\n", json!({"type":"step/start","time":1700000000000i64,"data":{"turn":1,"step":1}}))
}
fn message() -> String {
    format!("{}\n", json!({"type":"assistant/message","time":1700000003000i64,"data":{
        "turn":1,"step":1,"message":{"id":"m1","source":{"model":"global:deepseek-flash"}},
        "usage":{"inputTokens":10,"outputTokens":5,"cacheReadTokens":20,"cacheWriteTokens":2},
        "stream":[{"time0":1700000000200i64}]}}))
}
#[test]
fn usage17_dsh_unfinished_step_resumes_after_restart() {
    for compressed in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("session.v4.jsonl");
        let database = temp.path().join("ledger.db");
        let encode = |s: &str| if compressed { zstd::stream::encode_all(s.as_bytes(), 1).unwrap() } else { s.as_bytes().to_vec() };
        std::fs::write(&log, encode(&start())).unwrap();
        let mut db = open(&database);
        let mut file = File::open(&log).unwrap();
        assert_eq!(scan_dsh_file(&mut db, &scope(), &source(), "session", &mut file, compressed).unwrap().records, 0);
        let cursor = load_usage_cursor(&db, &scope(), &source()).unwrap().unwrap();
        assert!(cursor.parser_state.unwrap().contains("1:1"));
        drop(db);
        let bytes = encode(&message());
        let split = bytes.len() / 2;
        let mut writer = OpenOptions::new().append(true).open(&log).unwrap();
        writer.write_all(&bytes[..split]).unwrap();
        let mut db = open(&database);
        assert_eq!(scan_dsh_file(&mut db, &scope(), &source(), "session", &mut file, compressed).unwrap().records, 0);
        assert_eq!(load_usage_cursor(&db, &scope(), &source()).unwrap().unwrap().byte_offset, cursor.byte_offset);
        writer.write_all(&bytes[split..]).unwrap();
        assert_eq!(scan_dsh_file(&mut db, &scope(), &source(), "session", &mut file, compressed).unwrap().records, 1);
        // A second entry point bound to the same authorized home shares the cursor.
        let mut desktop_file = File::open(&log).unwrap();
        assert_eq!(scan_dsh_file(&mut db, &scope(), &source(), "session", &mut desktop_file, compressed).unwrap().records, 0);
        let payload: String = db.query_row("SELECT payload_json FROM usage_records", [], |r| r.get(0)).unwrap();
        let record: StoredUsageRecord = serde_json::from_str(&payload).unwrap();
        assert_eq!(record.record.performance.latency_ms.unwrap().value, 3000.0);
        assert_eq!(record.record.performance.first_token_ms.unwrap().value, 200.0);
        assert_eq!(record.record.tokens.unwrap().input, Some(10));
        assert!(record.record.cost.amount_usd.is_none());
        assert!(!load_usage_cursor(&db, &scope(), &source()).unwrap().unwrap().parser_state.unwrap().contains("1:1"));
    }
}
#[test]
fn usage17_dsh_invalid_message_rolls_back_step_and_cursor() {
    let temp = tempfile::tempdir().unwrap();
    let log = temp.path().join("session.v4.jsonl");
    std::fs::write(&log, format!("{}{{invalid}}\n", start())).unwrap();
    let mut db = open(&temp.path().join("ledger.db"));
    assert!(scan_dsh_file(&mut db, &scope(), &source(), "session", &mut File::open(log).unwrap(), false).is_err());
    assert!(load_usage_cursor(&db, &scope(), &source()).unwrap().is_none());
    assert_eq!(db.query_row("SELECT COUNT(*) FROM usage_records", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
}
