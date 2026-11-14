use std::fs::File;

use rusqlite::Connection;
use serde_json::{Value, json};
use vw_agent_apps::usage::{
    cost::ModelPricing,
    cursors::{UsageCursorKey, initialize_usage_cursors, load_usage_cursor},
    gemini::{collect_gemini_session_files, parse_gemini_tokens, scan_gemini_file},
    store::{StoredUsageRecord, initialize_usage_store},
};
use vw_api_types::{id::{ProjectId, ProjectScope}, usage::*};

fn context() -> (Connection, ProjectScope, UsageCursorKey) {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut db).unwrap();
    initialize_usage_cursors(&db).unwrap();
    (db, ProjectScope::for_default_tenant(ProjectId("project".into())).unwrap(), UsageCursorKey {
        target_host: "host".into(), app: "gemini".into(),
        instance_id: "instance".into(), file_identity: "logical-session".into(),
    })
}

fn message(id: &str) -> Value {
    json!({"id":id,"type":"gemini","model":"models/gemini-2.5-pro",
        "timestamp":"2026-11-05T08:00:00+08:00",
        "tokens":{"input":100,"output":20,"cached":60,"thoughts":5}})
}

fn document(messages: Vec<Value>) -> String {
    json!({"sessionId":"session","messages":messages}).to_string()
}

fn records(db: &Connection) -> Vec<StoredUsageRecord> {
    db.prepare("SELECT payload_json FROM usage_records").unwrap()
        .query_map([], |row| row.get::<_, String>(0)).unwrap()
        .map(|row| serde_json::from_str(&row.unwrap()).unwrap()).collect()
}

#[test]
fn usage17_gemini_repeat_scan_imports_no_new_gemini_records() {
    let (mut db, scope, source) = context();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), document(vec![message("a")])).unwrap();
    let scan = |db: &mut Connection| scan_gemini_file(db, &scope, &source,
        &mut File::open(file.path()).unwrap(), |_| Ok(None)).unwrap();
    assert_eq!(scan(&mut db).imported, 1);
    let revision: i64 = db.query_row("SELECT revision FROM usage_statistics_revisions", [], |row| row.get(0)).unwrap();
    let repeat = scan(&mut db);
    assert!(repeat.unchanged);
    assert_eq!(repeat.imported, 0);
    // A rewritten snapshot imports only new stable request IDs.
    std::fs::write(file.path(), document(vec![message("a"), message("b")])).unwrap();
    let changed = scan(&mut db);
    assert_eq!((changed.imported, changed.replayed), (1, 1));
    assert_eq!(records(&db).len(), 2);
    let next: i64 = db.query_row("SELECT revision FROM usage_statistics_revisions", [], |row| row.get(0)).unwrap();
    assert_eq!(next, revision + 1);
}

#[test]
fn usage17_gemini_prices_total_input_and_separate_thoughts() {
    let (mut db, scope, source) = context();
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), document(vec![message("a")])).unwrap();
    scan_gemini_file(&mut db, &scope, &source, &mut File::open(file.path()).unwrap(), |candidate| {
        Ok((candidate == "gemini-2.5-pro").then(|| ModelPricing::from_strings("1", "2", "0.1", "0").unwrap()))
    }).unwrap();
    let record = &records(&db)[0].record;
    assert_eq!(record.cost.amount_usd.as_deref(), Some("0.000096"));
    assert_eq!(record.cost.source, UsageCostSource::Calculated);
    assert_eq!(record.models.request_model.as_deref(), Some("models/gemini-2.5-pro"));
    assert_eq!(record.models.pricing_model.as_deref(), Some("gemini-2.5-pro"));
    assert_eq!(record.started_at_unix_seconds.0, 1_793_836_800);
    assert_eq!(record.provider, UsageProvider::Unknown);
    assert_eq!(record.tokens.as_ref().unwrap().reasoning, Some(5));
}

#[test]
fn usage17_gemini_preserves_missing_counters_and_defers_missing_identity_or_time() {
    let tokens = parse_gemini_tokens(&json!({"cached":u64::MAX})).unwrap();
    assert_eq!(tokens.input, None);
    assert_eq!(tokens.cache_read, Some(u64::MAX));
    assert_eq!(tokens.input_semantics, UsageInputSemantics::Total);
    assert_eq!(tokens.reasoning_semantics, UsageReasoningSemantics::SeparateFromOutput);
    assert!(parse_gemini_tokens(&json!({"input":0,"output":0})).is_none());
    let (mut db, scope, source) = context();
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut missing_time = message("time");
    missing_time["timestamp"] = json!("invalid");
    let mut missing_id = message("id");
    missing_id["id"] = Value::Null;
    let mut cached = message("cache");
    cached["tokens"] = json!({"cached":5000});
    std::fs::write(file.path(), document(vec![missing_time, missing_id, cached])).unwrap();
    let outcome = scan_gemini_file(&mut db, &scope, &source,
        &mut File::open(file.path()).unwrap(), |_| Ok(None)).unwrap();
    assert_eq!((outcome.imported, outcome.deferred), (1, 2));
    assert_eq!(records(&db)[0].record.cost.source, UsageCostSource::Unknown);
}

#[test]
fn usage17_gemini_reuses_session_decoder_and_defers_partial_documents() {
    let (mut db, scope, source) = context();
    let file = tempfile::NamedTempFile::new().unwrap();
    // The last snapshot of one message wins before accounting; nested tool usage is ignored.
    let mut updated = message("a");
    updated["tokens"]["input"] = json!(120);
    let text = format!("{{\"sessionId\":\"session\"}}\n{}\n{}\n", message("a"), updated);
    std::fs::write(file.path(), &text).unwrap();
    let scan = |db: &mut Connection| scan_gemini_file(db, &scope, &source,
        &mut File::open(file.path()).unwrap(), |_| Ok(None));
    assert_eq!(scan(&mut db).unwrap().imported, 1);
    assert_eq!(records(&db)[0].record.tokens.as_ref().unwrap().input, Some(120));
    let before = load_usage_cursor(&db, &scope, &source).unwrap();
    std::fs::write(file.path(), format!("{text}{{\"id\":" )).unwrap();
    assert!(scan(&mut db).is_err());
    assert_eq!(load_usage_cursor(&db, &scope, &source).unwrap(), before);
    assert_eq!(records(&db).len(), 1);
}

#[test]
fn usage17_gemini_discovers_only_session_files_and_prefers_migrated_jsonl() {
    let root = tempfile::tempdir().unwrap();
    assert!(collect_gemini_session_files(root.path()).unwrap().is_empty());
    let chats = root.path().join("tmp/project/chats");
    std::fs::create_dir_all(&chats).unwrap();
    for name in ["session-a.json", "session-a.jsonl", "session-b.json", "session-c.json.tmp", "other.json"] {
        std::fs::write(chats.join(name), "{}").unwrap();
    }
    assert_eq!(collect_gemini_session_files(root.path()).unwrap(), vec![
        chats.join("session-a.jsonl"), chats.join("session-b.json"),
    ]);
}
