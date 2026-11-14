//! Deferred Kimi validation fixtures. No tests were executed during implementation.
use std::{fs::OpenOptions, io::Write};
use rusqlite::Connection;
use serde_json::{Value, json};
use vw_agent_apps::usage::{cost::ModelPricing, cursors::{UsageCursorKey, initialize_usage_cursors},
    kimi::scan_kimi_file, store::{StoredUsageRecord, initialize_usage_store}};
use vw_api_types::{id::{ProjectId, ProjectScope}, usage::UsageCostSource};

fn message() -> Value {
    json!({"type":"agent.message.appended","time":1700000900000i64,
        "message":{"message":{"role":"assistant"},"meta":{"messageId":"one",
        "model":{"provider":"agent-loop","model":"agent-loop"},
        "usage":{"inputOther":10,"output":20,"inputCacheRead":30,"inputCacheCreation":40}}}})
}
fn setup() -> (Connection, ProjectScope, UsageCursorKey) {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut db).unwrap();
    initialize_usage_cursors(&db).unwrap();
    (db, ProjectScope::for_default_tenant(ProjectId("project".into())).unwrap(),
        UsageCursorKey { target_host:"host".into(), app:"kimi".into(),
            instance_id:"instance".into(), file_identity:"wire".into() })
}
fn retained(db: &Connection) -> StoredUsageRecord {
    let payload: String = db.query_row("SELECT payload_json FROM usage_records", [], |r| r.get(0)).unwrap();
    serde_json::from_str(&payload).unwrap()
}

#[test]
fn usage17_kimi_committed_record_refresh_does_not_duplicate() {
    let (mut db, scope, key) = setup();
    let mut wire = tempfile::tempfile().unwrap();
    for event in [
        json!({"type":"context.append_loop_event","time":1700000000000i64,
            "event":{"type":"step.begin","uuid":"one","turnId":"0","step":1}}),
        json!({"type":"llm.request","time":1700000000000i64,"kind":"loop",
            "agentId":"main","turnStep":"0.1","model":"actual"}), message(),
    ] { writeln!(wire, "{event}").unwrap(); }
    let first = scan_kimi_file(&mut db, &scope, &key, &mut wire, "session", "main", |_| Ok(None)).unwrap();
    assert_eq!(first.imported, 1);
    assert_eq!(retained(&db).record.cost.source, UsageCostSource::Unknown);
    writeln!(wire, "{}", json!({"type":"context.append_loop_event","time":1700000990000i64,
        "event":{"type":"step.end","uuid":"one","messageId":"one",
            "llmFirstTokenLatencyMs":1000,"llmStreamDurationMs":2000}})).unwrap();
    let second = scan_kimi_file(&mut db, &scope, &key, &mut wire, "session", "main",
        |_| Ok(Some(ModelPricing::from_strings("1", "2", "3", "4").unwrap()))).unwrap();
    assert_eq!((second.imported, second.refreshed), (0, 1));
    let record = retained(&db).record;
    assert_eq!(record.started_at_unix_seconds.0, 1700000003);
    assert_eq!(record.models.response_model.as_deref(), Some("actual"));
    assert_eq!(record.cost.amount_usd.as_deref(), Some("0.0003"));
    let tokens = record.tokens.unwrap();
    assert_eq!((tokens.input, tokens.output, tokens.cache_read, tokens.cache_creation),
        (Some(10), Some(20), Some(30), Some(40)));
    assert_eq!(record.performance.latency_ms.unwrap().value, 3000.0);
    assert_eq!(record.performance.first_token_ms.unwrap().value, 1000.0);
    let third = scan_kimi_file(&mut db, &scope, &key, &mut wire, "session", "main",
        |_| Ok(Some(ModelPricing::from_strings("1", "2", "3", "4").unwrap()))).unwrap();
    assert_eq!((third.imported, third.refreshed, third.replayed), (0, 0, 1));
    db.execute("DELETE FROM usage_records", []).unwrap();
    assert_eq!(scan_kimi_file(&mut db, &scope, &key, &mut wire, "session", "main", |_| Ok(None)).unwrap().imported, 0);
}

#[test]
fn usage17_kimi_uncommitted_and_incomplete_tail_are_not_billed() {
    let (mut db, scope, key) = setup();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wire.jsonl");
    let mut wire = OpenOptions::new().read(true).write(true).create_new(true).open(path).unwrap();
    writeln!(wire, "{}", json!({"type":"usage.record","time":1700000000000i64,
        "usage":{"inputOther":100,"output":200}})).unwrap();
    write!(wire, "{}", message()).unwrap();
    assert_eq!(scan_kimi_file(&mut db, &scope, &key, &mut wire, "session", "main", |_| Ok(None)).unwrap().imported, 0);
    // Scanning repositions the shared handle; append explicitly, as a real writer does.
    use std::io::{Seek, SeekFrom};
    wire.seek(SeekFrom::End(0)).unwrap();
    writeln!(wire).unwrap();
    assert_eq!(scan_kimi_file(&mut db, &scope, &key, &mut wire, "session", "main", |_| Ok(None)).unwrap().imported, 1);
}
