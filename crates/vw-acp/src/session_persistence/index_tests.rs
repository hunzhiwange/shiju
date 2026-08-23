//! 验证会话索引条目解析、排序读写以及从记录文件重建索引。

use super::*;
use crate::{SESSION_RECORD_SCHEMA, SessionEventLog, SessionTokenUsage};

fn test_record(id: &str, last_used_at: &str) -> SessionRecord {
    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: id.to_string(),
        acp_session_id: format!("acp-{id}"),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: Some("main".to_string()),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: last_used_at.to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: SessionEventLog {
            active_path: "/tmp/active.ndjson".to_string(),
            segment_count: 1,
            max_segment_bytes: 1024,
            max_segments: 3,
            last_write_at: None,
            last_write_error: None,
        },
        closed: Some(false),
        closed_at: None,
        pid: None,
        agent_started_at: None,
        last_prompt_at: None,
        last_agent_exit_code: None,
        last_agent_exit_signal: None,
        last_agent_exit_at: None,
        last_agent_disconnect_reason: None,
        protocol_version: None,
        agent_capabilities: None,
        title: None,
        messages: Vec::new(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: std::collections::HashMap::new(),
        vwacp: None,
    }
}

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "vw-acp-index-{name}-{}-{}",
        std::process::id(),
        temp_suffix()
    ))
}

async fn create_clean_dir(name: &str) -> PathBuf {
    let dir = temp_dir(name);
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.expect("create dir");
    dir
}

async fn write_record_file(dir: &Path, file_name: &str, record: &SessionRecord) {
    let payload = crate::serialize_session_record_for_disk(record);
    tokio::fs::write(dir.join(file_name), serde_json::to_vec(&payload).expect("json"))
        .await
        .expect("write record");
}

#[test]
fn parse_index_entry_requires_typed_required_fields_and_allows_missing_name() {
    let entry = parse_index_entry(&serde_json::json!({
        "file": "record.json",
        "vwacpRecordId": "record",
        "acpSessionId": "acp",
        "agentCommand": "agent",
        "cwd": "/tmp/project",
        "closed": false,
        "lastUsedAt": "2026-01-02T00:00:00Z"
    }))
    .expect("valid entry");

    assert_eq!(entry.name, None);
    assert_eq!(entry.file, "record.json");
    assert!(parse_index_entry(&serde_json::json!({"file": 1})).is_none());
    assert!(parse_index_entry(&serde_json::json!(true)).is_none());
    assert_eq!(
        parse_index_entry(&serde_json::json!({
            "file": "named.json",
            "vwacpRecordId": "named",
            "acpSessionId": "acp",
            "agentCommand": "agent",
            "cwd": "/tmp/project",
            "closed": true,
            "lastUsedAt": "2026-01-02T00:00:00Z",
            "name": "named session"
        }))
        .expect("named entry")
        .name
        .as_deref(),
        Some("named session")
    );
    assert_eq!(
        parse_index_entry(&serde_json::json!({
            "file": "unnamed.json",
            "vwacpRecordId": "unnamed",
            "acpSessionId": "acp",
            "agentCommand": "agent",
            "cwd": "/tmp/project",
            "closed": true,
            "lastUsedAt": "2026-01-02T00:00:00Z",
            "name": null
        }))
        .expect("null name entry")
        .name,
        None
    );
    assert!(
        parse_index_entry(&serde_json::json!({
            "file": "record.json",
            "vwacpRecordId": "record",
            "acpSessionId": "acp",
            "agentCommand": "agent",
            "cwd": "/tmp/project",
            "closed": false,
            "lastUsedAt": "2026-01-02T00:00:00Z",
            "name": 1
        }))
        .is_none()
    );
}

#[test]
fn to_session_index_entry_copies_stable_record_fields() {
    let record = test_record("record-1", "2026-01-02T00:00:00Z");
    let entry = to_session_index_entry(&record, "record-1.json");

    assert_eq!(entry.file, "record-1.json");
    assert_eq!(entry.vwacp_record_id, "record-1");
    assert_eq!(entry.acp_session_id, "acp-record-1");
    assert_eq!(entry.name.as_deref(), Some("main"));
    assert!(!entry.closed);

    let mut open_record = test_record("open-record", "2026-01-02T00:00:00Z");
    open_record.closed = None;
    let open_entry = to_session_index_entry(&open_record, "open-record.json");
    assert!(!open_entry.closed);
}

#[tokio::test]
async fn write_and_read_session_index_sorts_files_and_entries() {
    let dir = create_clean_dir("roundtrip").await;
    let older = to_session_index_entry(&test_record("old", "2026-01-01T00:00:00Z"), "b.json");
    let newer = to_session_index_entry(&test_record("new", "2026-01-02T00:00:00Z"), "a.json");
    let index = SessionIndex {
        files: vec!["b.json".to_string(), "a.json".to_string()],
        entries: vec![older, newer],
    };

    write_session_index(&dir, &index).await.expect("write index");
    let payload = tokio::fs::read_to_string(session_index_path(&dir)).await.expect("read index");
    let read = read_session_index(&dir).await.expect("parse index");

    assert!(payload.contains(SESSION_INDEX_SCHEMA));
    assert_eq!(read.files, vec!["a.json".to_string(), "b.json".to_string()]);
    assert_eq!(read.entries[0].vwacp_record_id, "new");

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn read_session_index_rejects_invalid_payload_shapes() {
    let dir = create_clean_dir("invalid-index").await;
    let cases = [
        "not json".to_string(),
        serde_json::json!(["not", "object"]).to_string(),
        serde_json::json!({"schema": "other", "files": [], "entries": []}).to_string(),
        serde_json::json!({"schema": SESSION_INDEX_SCHEMA, "entries": []}).to_string(),
        serde_json::json!({"schema": SESSION_INDEX_SCHEMA, "files": []}).to_string(),
        serde_json::json!({"schema": SESSION_INDEX_SCHEMA, "files": {}, "entries": []}).to_string(),
        serde_json::json!({"schema": SESSION_INDEX_SCHEMA, "files": [], "entries": {}}).to_string(),
        serde_json::json!({
            "schema": SESSION_INDEX_SCHEMA,
            "files": ["record.json", 1],
            "entries": []
        })
        .to_string(),
        serde_json::json!({
            "schema": SESSION_INDEX_SCHEMA,
            "files": [],
            "entries": [{
                "file": "record.json",
                "vwacpRecordId": "record",
                "acpSessionId": "acp",
                "agentCommand": "agent",
                "cwd": "/tmp/project",
                "closed": "no",
                "lastUsedAt": "2026-01-02T00:00:00Z"
            }]
        })
        .to_string(),
    ];

    for payload in cases {
        tokio::fs::write(session_index_path(&dir), payload).await.expect("write index");
        assert!(read_session_index(&dir).await.is_none());
    }

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn read_session_index_returns_none_when_file_is_missing() {
    let dir = create_clean_dir("missing-index").await;

    assert!(read_session_index(&dir).await.is_none());

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn rebuild_session_index_records_valid_json_files_and_skips_invalid_entries() {
    let dir = create_clean_dir("rebuild").await;
    tokio::fs::create_dir_all(dir.join("nested.json")).await.expect("create nested dir");
    tokio::fs::write(session_index_path(&dir), "{}").await.expect("write old index");
    tokio::fs::write(dir.join("broken.json"), "{").await.expect("write invalid json");
    tokio::fs::write(dir.join("notes.txt"), "ignored").await.expect("write ignored file");
    tokio::fs::write(
        dir.join("wrong-schema.json"),
        serde_json::json!({
            "schema": "other",
            "vwacp_record_id": "wrong",
            "acp_session_id": "acp-wrong",
            "agent_command": "agent",
            "cwd": "/tmp/project",
            "created_at": "2026-01-01T00:00:00Z",
            "last_used_at": "2026-01-02T00:00:00Z",
            "last_seq": 0,
            "event_log": {
                "active_path": "/tmp/active.ndjson",
                "segment_count": 1,
                "max_segment_bytes": 1024,
                "max_segments": 3
            },
            "messages": [],
            "updated_at": "2026-01-01T00:00:00Z",
            "cumulative_token_usage": {},
            "request_token_usage": {}
        })
        .to_string(),
    )
    .await
    .expect("write wrong schema");
    write_record_file(&dir, "b.json", &test_record("record-b", "2026-01-02T00:00:00Z")).await;
    write_record_file(&dir, "a.json", &test_record("record-a", "2026-01-03T00:00:00Z")).await;

    let index = rebuild_session_index(&dir).await.expect("rebuild index");
    let persisted = read_session_index(&dir).await.expect("read rebuilt index");

    assert_eq!(
        index.files,
        vec![
            "a.json".to_string(),
            "b.json".to_string(),
            "broken.json".to_string(),
            "wrong-schema.json".to_string()
        ]
    );
    assert_eq!(index.entries.len(), 2);
    assert_eq!(persisted.entries[0].vwacp_record_id, "record-a");
    assert_eq!(persisted.entries[1].vwacp_record_id, "record-b");

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[cfg(all(unix, not(target_os = "macos")))]
#[tokio::test]
async fn rebuild_session_index_skips_non_utf8_json_file_names() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = create_clean_dir("non-utf8-file").await;
    let file_name = OsString::from_vec(vec![0xff, b'.', b'j', b's', b'o', b'n']);
    tokio::fs::write(dir.join(file_name), b"{}").await.expect("write non-utf8 file");

    let index = rebuild_session_index(&dir).await.expect("rebuild index");

    assert_eq!(index.files.len(), 1);
    assert!(index.entries.is_empty());

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn rebuild_session_index_returns_read_dir_error_for_missing_directory() {
    let dir = temp_dir("missing-rebuild-dir");
    let _ = tokio::fs::remove_dir_all(&dir).await;

    let error = rebuild_session_index(&dir).await.expect_err("missing directory should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[tokio::test]
async fn write_session_index_returns_error_when_directory_is_missing() {
    let dir = temp_dir("missing-write-dir");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    let index = SessionIndex { files: Vec::new(), entries: Vec::new() };

    let error = write_session_index(&dir, &index).await.expect_err("missing directory should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[tokio::test]
async fn load_or_rebuild_session_index_reuses_current_index() {
    let dir = create_clean_dir("load-current").await;
    write_record_file(&dir, "record.json", &test_record("record", "2026-01-01T00:00:00Z")).await;
    let cached_entry = SessionIndexEntry {
        file: "record.json".to_string(),
        vwacp_record_id: "cached-record".to_string(),
        acp_session_id: "cached-acp".to_string(),
        agent_command: "cached-agent".to_string(),
        cwd: "/tmp/cached".to_string(),
        name: None,
        closed: true,
        last_used_at: "2026-01-03T00:00:00Z".to_string(),
    };
    write_session_index(
        &dir,
        &SessionIndex { files: vec!["record.json".to_string()], entries: vec![cached_entry] },
    )
    .await
    .expect("write cached index");

    let index = load_or_rebuild_session_index(&dir).await.expect("load index");

    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].vwacp_record_id, "cached-record");

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn load_or_rebuild_session_index_rebuilds_when_cached_file_order_differs() {
    let dir = create_clean_dir("load-reordered").await;
    write_record_file(&dir, "a.json", &test_record("record-a", "2026-01-02T00:00:00Z")).await;
    write_record_file(&dir, "b.json", &test_record("record-b", "2026-01-01T00:00:00Z")).await;
    tokio::fs::write(
        session_index_path(&dir),
        serde_json::json!({
            "schema": SESSION_INDEX_SCHEMA,
            "files": ["b.json", "a.json"],
            "entries": [{
                "file": "b.json",
                "vwacpRecordId": "cached-b",
                "acpSessionId": "cached-acp",
                "agentCommand": "cached-agent",
                "cwd": "/tmp/cached",
                "closed": false,
                "lastUsedAt": "2026-01-03T00:00:00Z"
            }]
        })
        .to_string(),
    )
    .await
    .expect("write reordered index");

    let index = load_or_rebuild_session_index(&dir).await.expect("rebuild reordered index");

    assert_eq!(index.files, vec!["a.json".to_string(), "b.json".to_string()]);
    assert_eq!(index.entries.len(), 2);
    assert!(index.entries.iter().any(|entry| entry.vwacp_record_id == "record-a"));
    assert!(index.entries.iter().any(|entry| entry.vwacp_record_id == "record-b"));

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn load_or_rebuild_session_index_rebuilds_when_file_list_changes() {
    let dir = create_clean_dir("load-stale").await;
    write_record_file(&dir, "old.json", &test_record("old", "2026-01-01T00:00:00Z")).await;
    write_session_index(
        &dir,
        &SessionIndex {
            files: vec!["old.json".to_string()],
            entries: vec![to_session_index_entry(
                &test_record("old", "2026-01-01T00:00:00Z"),
                "old.json",
            )],
        },
    )
    .await
    .expect("write stale index");
    write_record_file(&dir, "new.json", &test_record("new", "2026-01-02T00:00:00Z")).await;

    let index = load_or_rebuild_session_index(&dir).await.expect("rebuild stale index");

    assert_eq!(index.files, vec!["new.json".to_string(), "old.json".to_string()]);
    assert_eq!(index.entries.len(), 2);

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn load_or_rebuild_session_index_returns_read_dir_error_for_missing_directory() {
    let dir = temp_dir("missing-load-dir");
    let _ = tokio::fs::remove_dir_all(&dir).await;

    let error =
        load_or_rebuild_session_index(&dir).await.expect_err("missing directory should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}
