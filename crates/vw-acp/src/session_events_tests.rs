//! 验证会话事件分段、锁新鲜度、统计与陈旧锁恢复行为。
//!
//! 临时主目录和最小会话记录 fixture 用于隔离事件与偏好状态。

use super::*;
use crate::session_event_log::{
    session_event_active_path, session_event_lock_path, session_event_log,
    session_event_segment_path,
};
use crate::types::{SESSION_RECORD_SCHEMA, SessionTokenUsage};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{SystemTime, UNIX_EPOCH};

struct EnvVarGuard {
    _lock: tokio::sync::MutexGuard<'static, ()>,
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EnvVarGuard {
    async fn set_home(home: &Path) -> Self {
        let lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
        let saved = save_home_env();
        unsafe { std::env::set_var("HOME", home) };
        unsafe { std::env::set_var("USERPROFILE", home) };
        unsafe { std::env::remove_var("HOMEDRIVE") };
        unsafe { std::env::remove_var("HOMEPATH") };
        Self { _lock: lock, saved }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..) {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

fn save_home_env() -> Vec<(&'static str, Option<std::ffi::OsString>)> {
    ["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"]
        .into_iter()
        .map(|key| (key, std::env::var_os(key)))
        .collect()
}

fn temp_home(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir()
        .join(format!("vw-acp-session-events-{name}-{}-{unique}", std::process::id()))
}

fn spawn_live_process() -> Child {
    Command::new("sleep").arg("5").spawn().expect("spawn live process")
}

fn acp_message(value: Value) -> AcpJsonRpcMessage {
    serde_json::from_value(value).expect("valid ACP JSON-RPC message")
}

fn request_message(id: impl Into<Value>, method: &str) -> AcpJsonRpcMessage {
    acp_message(json!({
        "jsonrpc": "2.0",
        "id": id.into(),
        "method": method,
        "params": {}
    }))
}

fn notification_message(method: &str) -> AcpJsonRpcMessage {
    acp_message(json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": {}
    }))
}

fn test_record(session_id: &str, home: &Path) -> SessionRecord {
    let timestamp = "2026-01-01T00:00:00Z".to_string();
    SessionRecord {
        schema: SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: session_id.to_string(),
        acp_session_id: format!("acp-{session_id}"),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: home.to_string_lossy().into_owned(),
        name: None,
        created_at: timestamp.clone(),
        last_used_at: timestamp.clone(),
        last_seq: 0,
        last_request_id: None,
        event_log: session_event_log(session_id, home),
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
        updated_at: timestamp,
        cumulative_token_usage: SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: None,
    }
}

// 下列行为用例覆盖规范化、状态同步、路径安全和陈旧数据恢复。
#[test]
fn parse_event_lock_payload_accepts_valid_payload_and_defaults_invalid() {
    let parsed = parse_event_lock_payload(r#"{"pid":42,"created_at":"2026-01-01T00:00:00Z"}"#);

    assert_eq!(parsed.pid, Some(42));
    assert_eq!(parsed.created_at.as_deref(), Some("2026-01-01T00:00:00Z"));

    let parsed = parse_event_lock_payload("not-json");

    assert_eq!(parsed.pid, None);
    assert_eq!(parsed.created_at, None);
}

#[test]
fn normalize_positive_i64_uses_fallback_for_missing_zero_and_negative_values() {
    assert_eq!(normalize_positive_i64(Some(9), 3), 9);
    assert_eq!(normalize_positive_i64(Some(0), 3), 3);
    assert_eq!(normalize_positive_i64(Some(-1), 3), 3);
    assert_eq!(normalize_positive_i64(None, 3), 3);
}

#[tokio::test]
async fn stat_size_and_count_existing_segments_handle_missing_and_existing_files() {
    let home = temp_home("segments");
    let session_id = "session/with space";
    let session_dir = ensure_session_dir(&home).await.expect("create session dir");
    let active_path = session_event_active_path(session_id, &home);
    let segment_path = session_event_segment_path(session_id, 1, &home);

    tokio::fs::write(&active_path, b"active").await.expect("write active");
    tokio::fs::write(&segment_path, b"segment").await.expect("write segment");

    assert!(session_dir.ends_with(
        PathBuf::from(vw_config_types::paths::HOME_CONFIG_DIR_NAME).join("acp").join("sessions")
    ));
    assert!(path_exists(&active_path).await);
    assert_eq!(stat_size(&active_path).await, 6);
    assert_eq!(stat_size(&home.join("missing")).await, 0);
    assert_eq!(count_existing_segments(session_id, 3, &home).await, 2);

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[test]
fn lock_age_ms_treats_missing_or_invalid_timestamps_as_stale() {
    assert_eq!(lock_age_ms(None), i128::MAX);
    assert_eq!(lock_age_ms(Some("not-a-date")), i128::MAX);
    assert!(lock_age_ms(Some("1970-01-01T00:00:00Z")) > EVENT_LOCK_STALE_MS);
}

#[tokio::test]
async fn remove_stale_event_lock_treats_missing_lock_as_recovered() {
    let home = temp_home("missing-lock");
    let lock_path = session_event_lock_path("missing-session", &home);

    assert!(remove_stale_event_lock(&lock_path).await);
    assert!(!path_exists(&lock_path).await);
}

#[tokio::test(flavor = "current_thread")]
async fn remove_stale_event_lock_keeps_unreadable_lock_path() {
    let home = temp_home("unreadable-lock");
    let _env = EnvVarGuard::set_home(&home).await;
    let lock_path = session_event_lock_path("directory-lock", &home);
    tokio::fs::create_dir_all(&lock_path).await.expect("create directory at lock path");

    assert!(!remove_stale_event_lock(&lock_path).await);
    assert!(path_exists(&lock_path).await);

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn remove_stale_event_lock_removes_dead_or_malformed_lock_and_keeps_live_lock() {
    let home = temp_home("stale-lock");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "lock-session";
    let lock_path = session_event_lock_path(session_id, &home);
    ensure_session_dir(&home).await.expect("create session dir");

    tokio::fs::write(&lock_path, b"{not-json").await.expect("write malformed lock");
    assert!(remove_stale_event_lock(&lock_path).await);
    assert!(!path_exists(&lock_path).await);

    let mut live_process = spawn_live_process();
    let live_payload = serde_json::to_vec(&EventLockPayload {
        pid: Some(live_process.id()),
        created_at: Some(iso_now()),
    })
    .expect("serialize live lock");
    tokio::fs::write(&lock_path, live_payload).await.expect("write live lock");
    assert!(!remove_stale_event_lock(&lock_path).await);
    assert!(path_exists(&lock_path).await);
    let _ = live_process.kill();
    let _ = live_process.wait();

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn acquire_events_lock_creates_lock_and_release_is_idempotent() {
    let home = temp_home("lock-release");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "release-session";
    let lock_path = session_event_lock_path(session_id, &home);

    let lock = acquire_events_lock(session_id, &home).await.expect("acquire lock");
    assert_eq!(lock.file_path, lock_path);
    assert!(path_exists(&lock.file_path).await);

    release_events_lock(&lock).await.expect("release lock");
    assert!(!path_exists(&lock.file_path).await);
    release_events_lock(&lock).await.expect("release missing lock");

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn release_events_lock_returns_remove_errors() {
    let home = temp_home("lock-release-error");
    let _env = EnvVarGuard::set_home(&home).await;
    let lock_path = session_event_lock_path("release-directory", &home);
    tokio::fs::create_dir_all(&lock_path).await.expect("create directory at lock path");
    let lock = LockHandle { file_path: lock_path };

    let error = release_events_lock(&lock).await.expect_err("directory lock release should fail");

    assert_ne!(error.kind(), ErrorKind::NotFound);

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn open_counts_existing_segments_when_record_has_no_segment_count() {
    let home = temp_home("open-counts");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "open-count-session";
    ensure_session_dir(&home).await.expect("create session dir");
    tokio::fs::write(session_event_segment_path(session_id, 1, &home), b"old\n")
        .await
        .expect("write existing segment");
    tokio::fs::write(session_event_active_path(session_id, &home), b"active\n")
        .await
        .expect("write active segment");
    let mut record = test_record(session_id, &home);
    record.event_log.segment_count = 0;

    let mut writer =
        SessionEventWriter::open(record, SessionEventWriterOptions::default()).await.expect("open");

    assert_eq!(writer.get_record().event_log.segment_count, 0);
    assert_eq!(writer.segment_count, 2);

    writer.close(SessionEventAppendOptions { checkpoint: false }).await.expect("close");
    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn append_messages_empty_batch_keeps_record_and_files_unchanged() {
    let home = temp_home("empty-append");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "empty-session";
    let record = test_record(session_id, &home);
    let active_path = session_event_active_path(session_id, &home);

    let mut writer =
        SessionEventWriter::open(record, SessionEventWriterOptions::default()).await.expect("open");
    writer
        .append_messages(&[], SessionEventAppendOptions { checkpoint: true })
        .await
        .expect("append empty batch");

    assert_eq!(writer.get_record().last_seq, 0);
    assert_eq!(writer.get_record().last_request_id, None);
    assert!(!path_exists(&active_path).await);

    writer.close(SessionEventAppendOptions { checkpoint: false }).await.expect("close");
    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn append_message_updates_record_writes_checkpoint_and_rejects_closed_writer() {
    let home = temp_home("append-checkpoint");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "checkpoint-session";
    let record = test_record(session_id, &home);
    let message = request_message("req-1", "session/prompt");

    let mut writer =
        SessionEventWriter::open(record, SessionEventWriterOptions::default()).await.expect("open");
    writer
        .append_message(&message, SessionEventAppendOptions { checkpoint: true })
        .await
        .expect("append message");

    let active_payload = tokio::fs::read_to_string(session_event_active_path(session_id, &home))
        .await
        .expect("read active event log");
    assert_eq!(active_payload.lines().count(), 1);
    assert!(active_payload.contains("\"req-1\""));
    assert_eq!(writer.get_record().last_seq, 1);
    assert_eq!(writer.get_record().last_request_id.as_deref(), Some("req-1"));
    assert_eq!(writer.get_record().event_log.segment_count, DEFAULT_EVENT_MAX_SEGMENTS);
    assert_eq!(writer.get_record().event_log.max_segment_bytes, DEFAULT_EVENT_SEGMENT_MAX_BYTES);
    assert!(writer.get_record().event_log.last_write_at.is_some());
    assert_eq!(writer.get_record().event_log.last_write_error, None);

    let checkpoint = resolve_session_record(session_id).await.expect("resolve checkpoint");
    assert_eq!(checkpoint.last_seq, 1);
    assert_eq!(checkpoint.last_request_id.as_deref(), Some("req-1"));

    writer.close(SessionEventAppendOptions { checkpoint: false }).await.expect("close");
    let error = writer
        .append_message(&message, SessionEventAppendOptions { checkpoint: false })
        .await
        .expect_err("closed writer should reject append");
    assert_eq!(error.kind(), ErrorKind::Other);
    assert!(error.to_string().contains("closed"));
    let error = writer.checkpoint().await.expect_err("closed writer should reject checkpoint");
    assert_eq!(error.kind(), ErrorKind::Other);

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn checkpoint_persists_current_record_before_close() {
    let home = temp_home("explicit-checkpoint");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "explicit-checkpoint-session";
    let record = test_record(session_id, &home);
    let message = request_message("req-checkpoint", "session/prompt");

    let mut writer =
        SessionEventWriter::open(record, SessionEventWriterOptions::default()).await.expect("open");
    writer
        .append_message(&message, SessionEventAppendOptions { checkpoint: false })
        .await
        .expect("append message");
    writer.checkpoint().await.expect("checkpoint");

    let checkpoint = resolve_session_record(session_id).await.expect("resolve checkpoint");
    assert_eq!(checkpoint.last_seq, 1);
    assert_eq!(checkpoint.last_request_id.as_deref(), Some("req-checkpoint"));

    writer.close(SessionEventAppendOptions { checkpoint: false }).await.expect("close");
    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn close_with_checkpoint_persists_record_and_is_idempotent() {
    let home = temp_home("close-checkpoint");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "close-checkpoint-session";
    let record = test_record(session_id, &home);
    let message = request_message("req-close", "session/prompt");

    let mut writer =
        SessionEventWriter::open(record, SessionEventWriterOptions::default()).await.expect("open");
    writer
        .append_message(&message, SessionEventAppendOptions { checkpoint: false })
        .await
        .expect("append message");
    writer.close(SessionEventAppendOptions { checkpoint: true }).await.expect("close");
    writer.close(SessionEventAppendOptions { checkpoint: true }).await.expect("close again");

    let checkpoint = resolve_session_record(session_id).await.expect("resolve checkpoint");
    assert_eq!(checkpoint.last_seq, 1);
    assert_eq!(checkpoint.last_request_id.as_deref(), Some("req-close"));
    assert!(!path_exists(&session_event_lock_path(session_id, &home)).await);

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn append_messages_tracks_numeric_request_id_and_ignores_notification_id() {
    let home = temp_home("numeric-id");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "numeric-session";
    let record = test_record(session_id, &home);
    let messages =
        vec![request_message(7, "session/prompt"), notification_message("session/update")];

    let mut writer =
        SessionEventWriter::open(record, SessionEventWriterOptions::default()).await.expect("open");
    writer
        .append_messages(&messages, SessionEventAppendOptions { checkpoint: false })
        .await
        .expect("append batch");

    assert_eq!(writer.get_record().last_seq, 2);
    assert_eq!(writer.get_record().last_request_id.as_deref(), Some("7"));

    writer.close(SessionEventAppendOptions { checkpoint: false }).await.expect("close");
    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn append_messages_rotates_segments_and_caps_retained_segment_count() {
    let home = temp_home("rotate");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "rotate-session";
    let record = test_record(session_id, &home);
    let messages = vec![
        request_message("req-1", "session/one"),
        request_message("req-2", "session/two"),
        request_message("req-3", "session/three"),
        request_message("req-4", "session/four"),
    ];

    let mut writer = SessionEventWriter::open(
        record,
        SessionEventWriterOptions { max_segment_bytes: Some(1), max_segments: Some(2) },
    )
    .await
    .expect("open");
    writer
        .append_messages(&messages, SessionEventAppendOptions { checkpoint: true })
        .await
        .expect("append rotating batch");
    writer.close(SessionEventAppendOptions { checkpoint: false }).await.expect("close");

    assert!(!path_exists(&session_event_segment_path(session_id, 3, &home)).await);
    assert!(path_exists(&session_event_segment_path(session_id, 2, &home)).await);
    assert!(path_exists(&session_event_segment_path(session_id, 1, &home)).await);
    assert!(path_exists(&session_event_active_path(session_id, &home)).await);

    let events = list_session_events(session_id).await.expect("list events");
    let ids = events
        .iter()
        .filter_map(|message| serde_json::to_value(message).ok())
        .filter_map(|value| value.get("id").cloned())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![json!("req-2"), json!("req-3"), json!("req-4")]);

    let checkpoint = resolve_session_record(session_id).await.expect("resolve checkpoint");
    assert_eq!(checkpoint.event_log.segment_count, 2);
    assert_eq!(checkpoint.event_log.max_segments, 2);
    assert_eq!(checkpoint.event_log.max_segment_bytes, 1);

    let _ = tokio::fs::remove_dir_all(&home).await;
}

#[tokio::test(flavor = "current_thread")]
async fn list_session_events_orders_segments_before_active_and_skips_invalid_lines() {
    let home = temp_home("list");
    let _env = EnvVarGuard::set_home(&home).await;
    let session_id = "list-session";
    ensure_session_dir(&home).await.expect("create session dir");

    let older = request_message("older", "session/older");
    let middle = request_message("middle", "session/middle");
    let newest = request_message("newest", "session/newest");
    let older_line = serde_json::to_string(&older).expect("serialize older");
    let middle_line = serde_json::to_string(&middle).expect("serialize middle");
    let newest_line = serde_json::to_string(&newest).expect("serialize newest");

    tokio::fs::write(
        session_event_segment_path(session_id, 2, &home),
        format!("{older_line}\nnot-json\n"),
    )
    .await
    .expect("write segment 2");
    tokio::fs::write(
        session_event_segment_path(session_id, 1, &home),
        format!("{middle_line}\n{{\"jsonrpc\":\"1.0\",\"id\":\"skip\",\"result\":{{}}}}\n"),
    )
    .await
    .expect("write segment 1");
    tokio::fs::write(session_event_active_path(session_id, &home), format!("\n{newest_line}\n"))
        .await
        .expect("write active");

    let events = list_session_events(session_id).await.expect("list events");
    let ids = events
        .iter()
        .filter_map(|message| serde_json::to_value(message).ok())
        .filter_map(|value| value.get("id").cloned())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![json!("older"), json!("middle"), json!("newest")]);

    let _ = tokio::fs::remove_dir_all(&home).await;
}
