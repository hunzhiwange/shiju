use super::*;

use crate::queue_paths::TEST_HOME_ENV_LOCK;
use std::collections::HashMap;

fn temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vw-acp-repository-{name}-{}", std::process::id()))
}

struct HomeEnvGuard {
    previous: Option<std::ffi::OsString>,
}

impl HomeEnvGuard {
    fn set(home_dir: &Path) -> Self {
        let previous = std::env::var_os("HOME");
        unsafe { std::env::set_var("HOME", home_dir) };
        Self { previous }
    }
}

impl Drop for HomeEnvGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
    }
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

fn session_dir_for(home_dir: &Path) -> PathBuf {
    session_base_dir(home_dir)
}

async fn clean_home(name: &str) -> PathBuf {
    let home_dir = temp_dir(name);
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
    tokio::fs::create_dir_all(&home_dir).await.expect("create home");
    home_dir
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

#[test]
fn percent_encode_component_keeps_safe_bytes_and_encodes_path_separators() {
    assert_eq!(percent_encode_component("abc-_.!~*'()"), "abc-_.!~*'()");
    assert_eq!(percent_encode_component("a/b c"), "a%2Fb%20c");
}

#[test]
fn session_file_path_percent_encodes_record_id() {
    let path = session_file_path(Path::new("/tmp/sessions"), "record/1");

    assert_eq!(path, Path::new("/tmp/sessions").join("record%2F1.json"));
}

#[test]
fn absolute_path_removes_dot_and_parent_components() {
    let project_dir = temp_dir("absolute-path").join("project");
    let input = project_dir.join(".").join("src").join("..").join("Cargo.toml");
    let absolute = absolute_path(input.to_str().expect("temp path is utf-8"));

    assert_eq!(absolute, project_dir.join("Cargo.toml"));
}

#[test]
fn normalize_name_trims_and_rejects_empty_input() {
    assert_eq!(normalize_name(Some("  main ")).as_deref(), Some("main"));
    assert_eq!(normalize_name(Some("  ")), None);
    assert_eq!(normalize_name(None), None);
}

#[tokio::test]
async fn load_session_index_entries_reads_entries_from_index() {
    let dir = temp_dir("entries");
    tokio::fs::create_dir_all(&dir).await.expect("create dir");
    let index = crate::SessionIndex {
        files: vec!["record.json".to_string()],
        entries: vec![crate::SessionIndexEntry {
            file: "record.json".to_string(),
            vwacp_record_id: "record".to_string(),
            acp_session_id: "acp".to_string(),
            agent_command: "agent".to_string(),
            cwd: "/tmp/project".to_string(),
            name: None,
            closed: false,
            last_used_at: "2026-01-01T00:00:00Z".to_string(),
        }],
    };
    let record = crate::SessionRecord {
        schema: crate::SESSION_RECORD_SCHEMA.to_string(),
        vwacp_record_id: "record".to_string(),
        acp_session_id: "acp".to_string(),
        agent_session_id: None,
        agent_command: "agent".to_string(),
        agent_config: None,
        cwd: "/tmp/project".to_string(),
        name: None,
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
        request_token_usage: std::collections::HashMap::new(),
        vwacp: None,
    };
    let record_payload = crate::serialize_session_record_for_disk(&record);
    tokio::fs::write(dir.join("record.json"), serde_json::to_vec(&record_payload).expect("json"))
        .await
        .expect("write record");
    crate::write_session_index(&dir, &index).await.expect("write index");

    let entries = load_session_index_entries(&dir).await.expect("load entries");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].vwacp_record_id, "record");

    let _ = tokio::fs::remove_dir_all(&dir).await;
}

#[test]
fn matches_session_entry_filters_cwd_name_and_closed_state() {
    let cwd = Path::new("/tmp/project");
    let mut entry = crate::SessionIndexEntry {
        file: "record.json".to_string(),
        vwacp_record_id: "record".to_string(),
        acp_session_id: "acp".to_string(),
        agent_command: "agent".to_string(),
        cwd: cwd.to_string_lossy().into_owned(),
        name: Some("main".to_string()),
        closed: false,
        last_used_at: "2026-01-01T00:00:00Z".to_string(),
    };

    assert!(matches_session_entry(&entry, cwd, Some("main"), false));
    assert!(!matches_session_entry(&entry, Path::new("/tmp/other"), Some("main"), false));
    assert!(!matches_session_entry(&entry, cwd, None, false));

    entry.closed = true;
    assert!(!matches_session_entry(&entry, cwd, Some("main"), false));
    assert!(matches_session_entry(&entry, cwd, Some("main"), true));
}

#[test]
fn find_git_repository_root_walks_to_parent_git_directory() {
    let dir = temp_dir("git-root");
    let project = dir.join("project");
    let nested = project.join("src").join("bin");
    std::fs::create_dir_all(project.join(".git")).expect("create git dir");
    std::fs::create_dir_all(&nested).expect("create nested dir");

    let root = find_git_repository_root(nested.to_str().expect("utf-8 path"));

    assert_eq!(root, Some(project));
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn write_session_record_persists_file_and_updates_index() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("write").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let cwd = absolute_path("/tmp/project");
    let mut session = record("record/with space", "agent", &cwd, Some("main"));
    session.last_used_at = "2026-01-03T00:00:00Z".to_string();

    write_session_record(&session).await.expect("write session");

    let session_dir = session_dir_for(&home_dir);
    let payload = tokio::fs::read_to_string(session_dir.join("record%2Fwith%20space.json"))
        .await
        .expect("read record");
    let index = crate::read_session_index(&session_dir).await.expect("read index");
    assert!(payload.ends_with('\n'));
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].vwacp_record_id, "record/with space");
    assert_eq!(index.files, vec!["record%2Fwith%20space.json".to_string()]);

    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn resolve_session_record_prefers_direct_record_file() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("resolve-direct").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let cwd = absolute_path("/tmp/project");
    let session = record("direct", "agent", &cwd, None);
    write_session_record(&session).await.expect("write session");

    let resolved = resolve_session_record("direct").await.expect("resolve direct");

    assert_eq!(resolved.vwacp_record_id, "direct");
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn resolve_session_record_uses_index_for_acp_id_and_suffix_matches() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("resolve-index").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let cwd = absolute_path("/tmp/project");
    let first = record("prefix-abc123", "agent", &cwd, None);
    let second = record("prefix-xyz789", "agent", &cwd, None);
    persist_index(&session_dir, &[first.clone(), second]).await;

    let by_acp = resolve_session_record(&first.acp_session_id).await.expect("resolve acp");
    let by_suffix = resolve_session_record("abc123").await.expect("resolve suffix");

    assert_eq!(by_acp.vwacp_record_id, "prefix-abc123");
    assert_eq!(by_suffix.vwacp_record_id, "prefix-abc123");
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn resolve_session_record_reports_ambiguous_suffix_and_missing_id() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("resolve-errors").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let cwd = absolute_path("/tmp/project");
    persist_index(
        &session_dir,
        &[record("one-shared", "agent", &cwd, None), record("two-shared", "agent", &cwd, None)],
    )
    .await;

    let ambiguous = resolve_session_record("shared").await.expect_err("ambiguous suffix");
    let missing = resolve_session_record("missing").await.expect_err("missing id");

    assert!(matches!(ambiguous, SessionRepositoryError::SessionResolution(_)));
    assert!(matches!(missing, SessionRepositoryError::SessionNotFound(_)));
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn list_sessions_filters_agent_and_sorts_by_last_used_descending() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("list").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let cwd = absolute_path("/tmp/project");
    let mut old = record("old", "agent", &cwd, None);
    old.last_used_at = "2026-01-01T00:00:00Z".to_string();
    let mut new = record("new", "agent", &cwd, None);
    new.last_used_at = "2026-01-03T00:00:00Z".to_string();
    let mut other = record("other", "other-agent", &cwd, None);
    other.last_used_at = "2026-01-04T00:00:00Z".to_string();
    persist_index(&session_dir, &[old, new, other]).await;

    let all = list_sessions().await.expect("list sessions");
    let agent_only = list_sessions_for_agent("agent").await.expect("list for agent");

    assert_eq!(
        all.iter().map(|session| session.vwacp_record_id.as_str()).collect::<Vec<_>>(),
        vec!["other", "new", "old"]
    );
    assert_eq!(
        agent_only.iter().map(|session| session.vwacp_record_id.as_str()).collect::<Vec<_>>(),
        vec!["new", "old"]
    );
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn list_sessions_for_agent_skips_unparseable_matching_records() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("list-agent-skip-bad").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let cwd = absolute_path("/tmp/project");
    let good = record("good", "agent", &cwd, None);
    write_record_file(&session_dir, &good).await;
    tokio::fs::write(session_dir.join("bad.json"), "{").await.expect("write invalid record");
    crate::write_session_index(
        &session_dir,
        &crate::SessionIndex {
            files: vec!["good.json".to_string(), "bad.json".to_string()],
            entries: vec![
                crate::to_session_index_entry(&good, "good.json"),
                crate::SessionIndexEntry {
                    file: "bad.json".to_string(),
                    vwacp_record_id: "bad".to_string(),
                    acp_session_id: "acp-bad".to_string(),
                    agent_command: "agent".to_string(),
                    cwd: cwd.to_string_lossy().into_owned(),
                    name: None,
                    closed: false,
                    last_used_at: "2026-01-02T00:00:00Z".to_string(),
                },
            ],
        },
    )
    .await
    .expect("write index");

    let sessions = list_sessions_for_agent("agent").await.expect("list agent sessions");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].vwacp_record_id, "good");
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn find_session_matches_normalized_cwd_name_and_closed_policy() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("find").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let cwd = absolute_path("/tmp/project/./child/..");
    let open = record("open", "agent", &cwd, Some("main"));
    let mut closed = record("closed", "agent", &cwd, Some("done"));
    closed.closed = Some(true);
    persist_index(&session_dir, &[open.clone(), closed.clone()]).await;

    let found = find_session(&FindSessionOptions {
        agent_command: "agent".to_string(),
        cwd: "/tmp/project".to_string(),
        name: Some(" main ".to_string()),
        include_closed: false,
    })
    .await
    .expect("find session");
    let hidden_closed = find_session(&FindSessionOptions {
        agent_command: "agent".to_string(),
        cwd: "/tmp/project".to_string(),
        name: Some("done".to_string()),
        include_closed: false,
    })
    .await
    .expect("find closed hidden");
    let shown_closed = find_session(&FindSessionOptions {
        agent_command: "agent".to_string(),
        cwd: "/tmp/project".to_string(),
        name: Some("done".to_string()),
        include_closed: true,
    })
    .await
    .expect("find closed shown");

    assert_eq!(found.map(|session| session.vwacp_record_id), Some("open".to_string()));
    assert!(hidden_closed.is_none());
    assert_eq!(shown_closed.map(|session| session.vwacp_record_id), Some("closed".to_string()));
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn find_session_by_directory_walk_stops_at_boundary() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("walk").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let session_dir = session_dir_for(&home_dir);
    tokio::fs::create_dir_all(&session_dir).await.expect("create session dir");
    let root = temp_dir("walk-project").join("repo");
    let nested = root.join("src").join("bin");
    let outside_boundary = nested.clone();
    let _ = tokio::fs::remove_dir_all(&root).await;
    tokio::fs::create_dir_all(&nested).await.expect("create nested");
    let session = record("root-session", "agent", &root, None);
    persist_index(&session_dir, &[session]).await;

    let found = find_session_by_directory_walk(&FindSessionByDirectoryWalkOptions {
        agent_command: "agent".to_string(),
        cwd: nested.to_string_lossy().into_owned(),
        name: None,
        boundary: Some(root.to_string_lossy().into_owned()),
    })
    .await
    .expect("walk to root");
    let blocked = find_session_by_directory_walk(&FindSessionByDirectoryWalkOptions {
        agent_command: "agent".to_string(),
        cwd: nested.to_string_lossy().into_owned(),
        name: None,
        boundary: Some(outside_boundary.to_string_lossy().into_owned()),
    })
    .await
    .expect("walk blocked");

    assert_eq!(found.map(|session| session.vwacp_record_id), Some("root-session".to_string()));
    assert!(blocked.is_none());
    let _ = tokio::fs::remove_dir_all(&root).await;
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}

#[tokio::test]
async fn close_session_marks_record_closed_and_rebuilds_index() {
    let _lock = TEST_HOME_ENV_LOCK.lock().await;
    let home_dir = clean_home("close").await;
    let _home = HomeEnvGuard::set(&home_dir);
    let cwd = absolute_path("/tmp/project");
    let mut session = record("close-me", "agent", &cwd, None);
    session.last_prompt_at = None;
    write_session_record(&session).await.expect("write session");

    let closed = close_session("close-me").await.expect("close session");
    let persisted = resolve_session_record("close-me").await.expect("resolve closed");

    assert_eq!(closed.closed, Some(true));
    assert!(closed.closed_at.is_some());
    assert!(closed.last_prompt_at.is_some());
    assert_eq!(closed.pid, None);
    assert_eq!(persisted.closed, Some(true));
    let _ = tokio::fs::remove_dir_all(&home_dir).await;
}
