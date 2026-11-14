//! Authored for later validation; no native client or network is required.
use std::{fs::{File, OpenOptions}, io::Write};
use rusqlite::Connection;
use serde_json::json;
use vw_agent_apps::usage::{cursors::{UsageCursorKey, initialize_usage_cursors, load_usage_cursor}, grokbuild::{scan_grokbuild_file, collect_grokbuild_session_files}, store::{initialize_usage_store, StoredUsageRecord}};
use vw_api_types::id::{ProjectId, ProjectScope};

fn context() -> (Connection, ProjectScope, UsageCursorKey) {
    let mut db = Connection::open_in_memory().unwrap();
    initialize_usage_store(&mut db).unwrap();
    initialize_usage_cursors(&db).unwrap();
    (db, ProjectScope::for_default_tenant(ProjectId("project".into())).unwrap(), UsageCursorKey {
        target_host: "host".into(), app: "grokbuild".into(), instance_id: "instance".into(), file_identity: "session".into(),
    })
}
fn event(prompt: &str, input: u64) -> String {
    format!("{}\n", json!({"method":"_x.ai/session/update", "timestamp":1000,
        "params":{"update":{"sessionUpdate":"turn_completed","prompt_id":prompt,
        "usage":{"modelUsage":{"grok-build":{"inputTokens":input,"outputTokens":10,"reasoningTokens":2}}}}}}))
}
#[test]
fn usage17_grokbuild_unsettled_update_is_deferred_until_stable() {
    let (mut db, scope, key) = context();
    let temp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(temp.path(), event("prompt", 100)).unwrap();
    let mut file = File::open(temp.path()).unwrap();
    let young = scan_grokbuild_file(&mut db, &scope, &key, &mut file, 1059, |_| Ok(None)).unwrap();
    assert!(young.deferred);
    assert_eq!(young.imported, 0);
    assert_eq!(load_usage_cursor(&db, &scope, &key).unwrap().unwrap().byte_offset, 0);
    assert_eq!(scan_grokbuild_file(&mut db, &scope, &key, &mut file, 1060, |_| Ok(None)).unwrap().imported, 1);
    assert_eq!(scan_grokbuild_file(&mut db, &scope, &key, &mut file, 1061, |_| Ok(None)).unwrap().imported, 0);
    let mut append = OpenOptions::new().append(true).open(temp.path()).unwrap();
    // Same-prompt cumulative replay is empty; an increase contributes only its delta.
    write!(append, "{}{}{}", event("prompt", 100), event("prompt", 150), event("next", 100)).unwrap();
    assert_eq!(scan_grokbuild_file(&mut db, &scope, &key, &mut file, 1061, |_| Ok(None)).unwrap().imported, 2);
    let rows: Vec<StoredUsageRecord> = db.prepare("SELECT payload_json FROM usage_records").unwrap()
        .query_map([], |row| row.get::<_, String>(0)).unwrap()
        .map(|row| serde_json::from_str(&row.unwrap()).unwrap()).collect();
    assert_eq!(rows.iter().map(|r| r.record.tokens.as_ref().unwrap().input.unwrap()).sum::<u64>(), 250);
}
#[test]
fn usage17_grokbuild_oversized_line_preserves_following_event() {
    let (mut db, scope, key) = context();
    let mut temp = tempfile::NamedTempFile::new().unwrap();
    temp.write_all(&vec![b'x'; 8 * 1024 * 1024 + 1]).unwrap();
    writeln!(temp).unwrap();
    temp.write_all(event("prompt", 100).as_bytes()).unwrap();
    let outcome = scan_grokbuild_file(&mut db, &scope, &key, &mut File::open(temp.path()).unwrap(), 1060, |_| Ok(None)).unwrap();
    assert_eq!(outcome.skipped_lines, 1);
    assert_eq!(outcome.imported, 1);
}
#[test]
fn usage17_grokbuild_collection_rejects_relative_and_linked_roots() {
    assert!(collect_grokbuild_session_files(std::path::Path::new("untrusted")).is_err());
    #[cfg(unix)] {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("sessions")).unwrap();
        assert!(collect_grokbuild_session_files(temp.path()).is_err());
    }
}
