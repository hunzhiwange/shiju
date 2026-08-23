//! 验证会话仓库的路径编码、词法边界与仓库根目录发现边界。
//!
//! 临时目录和最小记录 fixture 用于验证磁盘格式，不共享真实用户会话数据。

use super::*;

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vw-acp-repository-edge-{name}-{}", std::process::id()))
}

struct EnvVarGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &Path) -> Self {
        let keys = env_keys(key);
        let saved = keys.iter().map(|key| (*key, std::env::var_os(key))).collect();
        unsafe { std::env::set_var(key, value) };
        if key == "HOME" {
            unsafe { std::env::set_var("USERPROFILE", value) };
            unsafe { std::env::remove_var("HOMEDRIVE") };
            unsafe { std::env::remove_var("HOMEPATH") };
        }
        Self { saved }
    }

    fn clear(key: &'static str) -> Self {
        let keys = env_keys(key);
        let saved = keys.iter().map(|key| (*key, std::env::var_os(key))).collect();
        unsafe { std::env::remove_var(key) };
        if key == "HOME" {
            unsafe { std::env::remove_var("USERPROFILE") };
            unsafe { std::env::remove_var("HOMEDRIVE") };
            unsafe { std::env::remove_var("HOMEPATH") };
        }
        Self { saved }
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

fn env_keys(key: &'static str) -> Vec<&'static str> {
    if key == "HOME" { vec!["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"] } else { vec![key] }
}

fn record(id: &str, agent_command: &str, cwd: &Path, name: Option<&str>) -> crate::SessionRecord {
    crate::SessionRecord {
        schema: crate::SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: id.to_string(),
        acp_session_id: format!("acp-{id}"),
        agent_session_id: None,
        agent_command: agent_command.to_string(),
        agent_config: None,
        cwd: cwd.to_string_lossy().into_owned(),
        name: name.map(ToOwned::to_owned),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
        last_seq: 0,
        last_request_id: None,
        event_log: crate::SessionEventLog {
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
        cumulative_token_usage: crate::SessionTokenUsage::default(),
        request_token_usage: HashMap::new(),
        vwacp: None,
    }
}

async fn clean_home(name: &str) -> PathBuf {
    let home_dir = temp_dir(name);
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
    tokio::fs::create_dir_all(&home_dir).await.expect("create home");
    home_dir
}

fn session_dir_for(home_dir: &Path) -> PathBuf {
    session_base_dir(home_dir)
}

fn index_entry(file: &str) -> SessionIndexEntry {
    SessionIndexEntry {
        file: file.to_string(),
        vwacp_record_id: file.trim_end_matches(".json").to_string(),
        acp_session_id: format!("acp-{}", file.trim_end_matches(".json")),
        agent_command: "agent".to_string(),
        cwd: "/tmp/project".to_string(),
        name: None,
        closed: false,
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

async fn write_record_file(session_dir: &Path, session: &crate::SessionRecord) {
    let payload = crate::serialize_session_record_for_disk(session);
    let path = session_file_path(session_dir, &session.vwacp_record_id);
    tokio::fs::write(path, serde_json::to_vec(&payload).expect("json"))
        .await
        .expect("write record");
}

async fn persist_index(session_dir: &Path, sessions: &[crate::SessionRecord]) {
    let mut files = Vec::new();
    let mut entries = Vec::new();
    for session in sessions {
        let file_name = format!("{}.json", percent_encode_component(&session.vwacp_record_id));
        files.push(file_name.clone());
        entries.push(crate::to_session_index_entry(session, &file_name));
        write_record_file(session_dir, session).await;
    }
    crate::write_session_index(session_dir, &crate::SessionIndex { files, entries })
        .await
        .expect("write index");
}

// 下列行为用例覆盖合法往返、损坏输入、路径边界和索引恢复。
#[test]
fn percent_encode_component_encodes_reserved_ascii_and_unicode_bytes() {
    assert_eq!(percent_encode_component("雪+?"), "%E9%9B%AA%2B%3F");
}

#[test]
fn lexical_absolute_uses_current_dir_for_relative_paths() {
    let expected = std::env::current_dir().expect("current dir").join("b");

    assert_eq!(lexical_absolute(Path::new("a/../b")), expected);
}

#[test]
fn is_within_boundary_accepts_same_or_nested_paths_only() {
    let boundary = Path::new("/tmp/project");

    assert!(is_within_boundary(boundary, boundary));
    assert!(is_within_boundary(boundary, Path::new("/tmp/project/src")));
    assert!(!is_within_boundary(boundary, Path::new("/tmp/other")));
}

#[test]
fn find_git_repository_root_returns_none_without_git_directory() {
    let dir = temp_dir("git-none");
    let nested = dir.join("project").join("src");
    std::fs::create_dir_all(&nested).expect("create nested dir");

    let root = find_git_repository_root(nested.to_str().expect("utf-8 path"));

    assert_eq!(root, None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(not(windows))]
#[tokio::test]
async fn repository_reports_missing_home_environment() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let _home = EnvVarGuard::clear("HOME");

    let dir_error = default_repository_dir().expect_err("missing home dir");
    let list_error = list_sessions().await.expect_err("missing home list");

    assert!(matches!(dir_error, SessionRepositoryError::HomeDirUnavailable));
    assert!(matches!(list_error, SessionRepositoryError::HomeDirUnavailable));
}

#[tokio::test]
async fn load_session_index_entries_reports_missing_directory_io_error() {
    let dir = temp_dir("missing-index-dir");
    let _ = tokio::fs::remove_dir_all(&dir).await;

    let error = load_session_index_entries(&dir).await.expect_err("missing index dir");

    assert!(matches!(error, SessionRepositoryError::Io(_)));
}

#[tokio::test]
async fn load_record_from_index_entry_skips_unreadable_invalid_and_unparseable_files() {
    let dir = temp_dir("load-record-none");
    let _ = tokio::fs::remove_dir_all(&dir).await;
    tokio::fs::create_dir_all(&dir).await.expect("create dir");
    tokio::fs::write(dir.join("invalid.json"), "{").await.expect("write invalid");
    tokio::fs::write(
        dir.join("wrong-schema.json"),
        serde_json::json!({
            "schema": "other",
            "vwacp_record_id": "wrong",
            "acp_session_id": "acp-wrong"
        })
        .to_string(),
    )
    .await
    .expect("write wrong schema");

    assert!(load_record_from_index_entry(&dir, &index_entry("missing.json")).await.is_none());
    assert!(load_record_from_index_entry(&dir, &index_entry("invalid.json")).await.is_none());
    assert!(load_record_from_index_entry(&dir, &index_entry("wrong-schema.json")).await.is_none());

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[tokio::test]
async fn resolve_session_record_falls_back_from_invalid_direct_file_to_index() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("resolve-invalid-direct").await;
    let _home = EnvVarGuard::set("HOME", &home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    tokio::fs::write(session_file_path(&session_dir, "fallback"), "{")
        .await
        .expect("write invalid direct");
    let cwd = absolute_path("/tmp/project");
    let mut session = record("real-fallback", "agent", &cwd, None);
    session.acp_session_id = "fallback".to_string();
    persist_index(&session_dir, &[session]).await;

    let resolved = resolve_session_record("fallback").await.expect("resolve via index");

    assert_eq!(resolved.vwacp_record_id, "real-fallback");
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn resolve_session_record_reports_duplicate_exact_matches() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("resolve-duplicate-exact").await;
    let _home = EnvVarGuard::set("HOME", &home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let cwd = absolute_path("/tmp/project");
    let mut first = record("first", "agent", &cwd, None);
    let mut second = record("second", "agent", &cwd, None);
    first.acp_session_id = "shared-exact".to_string();
    second.acp_session_id = "shared-exact".to_string();
    persist_index(&session_dir, &[first, second]).await;

    let error = resolve_session_record("shared-exact").await.expect_err("duplicate exact");

    assert!(matches!(error, SessionRepositoryError::SessionResolution(_)));
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn list_sessions_skips_index_entries_without_parseable_records() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("list-skip-bad").await;
    let _home = EnvVarGuard::set("HOME", &home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let cwd = absolute_path("/tmp/project");
    let good = record("good", "agent", &cwd, None);
    write_record_file(&session_dir, &good).await;
    tokio::fs::write(session_dir.join("bad.json"), "{").await.expect("write invalid record");
    crate::write_session_index(
        &session_dir,
        &crate::SessionIndex {
            files: vec!["bad.json".to_string(), "good.json".to_string()],
            entries: vec![
                index_entry("bad.json"),
                crate::to_session_index_entry(&good, "good.json"),
            ],
        },
    )
    .await
    .expect("write index");

    let sessions = list_sessions().await.expect("list sessions");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].vwacp_record_id, "good");
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn find_session_returns_none_when_matching_index_file_is_invalid() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("find-invalid-record").await;
    let _home = EnvVarGuard::set("HOME", &home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    tokio::fs::write(session_dir.join("bad.json"), "{").await.expect("write invalid record");
    let cwd = absolute_path("/tmp/project");
    let mut entry = index_entry("bad.json");
    entry.cwd = cwd.to_string_lossy().into_owned();
    crate::write_session_index(
        &session_dir,
        &crate::SessionIndex { files: vec!["bad.json".to_string()], entries: vec![entry] },
    )
    .await
    .expect("write index");

    let found = find_session(&FindSessionOptions {
        agent_command: "agent".to_string(),
        cwd: "/tmp/project".to_string(),
        name: None,
        include_closed: false,
    })
    .await
    .expect("find session");

    assert!(found.is_none());
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn find_session_by_directory_walk_uses_start_when_boundary_is_outside_start() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("walk-outside-boundary").await;
    let _home = EnvVarGuard::set("HOME", &home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let root = temp_dir("walk-outside-project").join("repo");
    let nested = root.join("src").join("bin");
    let outside = temp_dir("walk-outside-other");
    let _ = tokio::fs::remove_dir_all(&root).await;
    let _ = tokio::fs::remove_dir_all(&outside).await;
    tokio::fs::create_dir_all(&nested).await.expect("create nested");
    tokio::fs::create_dir_all(&outside).await.expect("create outside");
    let session = record("root-session", "agent", &root, None);
    persist_index(&session_dir, &[session]).await;

    let found = find_session_by_directory_walk(&FindSessionByDirectoryWalkOptions {
        agent_command: "agent".to_string(),
        cwd: nested.to_string_lossy().into_owned(),
        name: None,
        boundary: Some(outside.to_string_lossy().into_owned()),
    })
    .await
    .expect("walk with outside boundary");

    assert!(found.is_none());
    let _ = tokio::fs::remove_dir_all(&root).await;
    let _ = tokio::fs::remove_dir_all(&outside).await;
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn write_session_record_replaces_existing_index_entry() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("write-replace").await;
    let _home = EnvVarGuard::set("HOME", &home_dir);
    let cwd = absolute_path("/tmp/project");
    let mut session = record("same", "agent", &cwd, None);
    write_session_record(&session).await.expect("write first");
    session.last_used_at = "2026-01-04T00:00:00Z".to_string();
    session.name = Some("renamed".to_string());

    write_session_record(&session).await.expect("write second");

    let index = crate::read_session_index(session_dir_for(&home_dir)).await.expect("read index");
    assert_eq!(index.files, vec!["same.json".to_string()]);
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].name.as_deref(), Some("renamed"));
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn close_session_preserves_prompt_time_and_clears_missing_process_pid() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("close-preserve-prompt").await;
    let _home = EnvVarGuard::set("HOME", &home_dir);
    let cwd = absolute_path("/tmp/project");
    let mut session = record("close-preserve", "agent", &cwd, None);
    session.last_prompt_at = Some("2026-01-02T00:00:00Z".to_string());
    session.pid = Some(2_000_000_000);
    write_session_record(&session).await.expect("write session");

    let closed = close_session("close-preserve").await.expect("close session");

    assert_eq!(closed.closed, Some(true));
    assert_eq!(closed.last_prompt_at.as_deref(), Some("2026-01-02T00:00:00Z"));
    assert_eq!(closed.pid, None);
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn close_session_returns_not_found_for_missing_id() {
    let _lock = crate::queue_paths::TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("close-missing").await;
    let _home = EnvVarGuard::set("HOME", &home_dir);

    let error = close_session("missing").await.expect_err("missing close");

    assert!(matches!(error, SessionRepositoryError::SessionNotFound(_)));
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}
