use std::fs;

use anyhow::Result;
use rusqlite::{Connection, params};
use tempfile::TempDir;
use vw_api_types::id::TenantScope;
use vw_shared::session::info::{
    Info as AgentSessionInfo, RevertInfo, ShareInfo, Summary, TimeInfo,
};

use super::*;
use crate::global_database::GlobalDatabase;
use crate::global_database::test_support::PostgresTestDatabase;
use crate::project::store::ProjectStore;

#[test]
fn session_scope_round_trips_and_clears() {
    let _global_state = GlobalStateGuard::new();

    set_session_scope(Some("/tmp/vw-scope"));
    assert_eq!(current_session_scope().as_deref(), Some("/tmp/vw-scope"));

    set_session_scope(None);
    assert_eq!(current_session_scope(), None);
}

#[tokio::test]
async fn tenant_scope_is_bound_to_current_future_and_restored_after_nested_scope() -> Result<()> {
    let default_tenant = TenantScope::default();
    let outer_tenant = TenantScope::new("outer-platform", "outer-company")?;
    let inner_tenant = TenantScope::new("inner-platform", "inner-company")?;

    assert_eq!(current_tenant_scope(), default_tenant);
    with_tenant_scope(outer_tenant.clone(), async {
        assert_eq!(current_tenant_scope(), outer_tenant);
        with_tenant_scope(inner_tenant.clone(), async {
            assert_eq!(current_tenant_scope(), inner_tenant);
        })
        .await;
        assert_eq!(current_tenant_scope(), outer_tenant);
    })
    .await;
    assert_eq!(current_tenant_scope(), default_tenant);
    Ok(())
}

#[test]
fn project_scope_trims_valid_ids_and_rejects_missing_ids() -> Result<()> {
    let tenant = TenantScope::new("scope-platform", "scope-company")?;
    let scope = project_scope(&tenant, Some("  project-one  "))?;

    assert_eq!(scope.tenant_scope(), &tenant);
    assert_eq!(scope.project_id().as_ref(), "project-one");
    for missing in [None, Some(""), Some(" \t ")] {
        let error = project_scope(&tenant, missing).expect_err("missing scope must be rejected");
        assert!(error.to_string().contains("non-empty project scope"));
    }
    Ok(())
}

static UI_STORE_GLOBAL_STATE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct GlobalStateGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl GlobalStateGuard {
    fn new() -> Self {
        let lock = UI_STORE_GLOBAL_STATE_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        set_session_scope(None);
        Self { _lock: lock }
    }
}

impl Drop for GlobalStateGuard {
    fn drop(&mut self) {
        set_session_scope(None);
    }
}

struct FacadeFixture {
    _temp: TempDir,
    _store_override: TestSessionStoreOverride,
    data_dir: PathBuf,
    database_path: PathBuf,
    project_id: String,
    project_store: ProjectStore,
}

impl FacadeFixture {
    fn new(project_id: &str) -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let database_path = vw_config_types::paths::global_database_path(temp.path());
        let database = GlobalDatabase::new(&database_path)?;
        let project_store = ProjectStore::new(database.clone())?;
        create_project(&project_store, temp.path(), project_id)?;
        let store_override = override_session_store_for_test(SessionStore::new(database)?);
        Ok(Self {
            data_dir: vw_config_types::paths::home_config_dir(temp.path()),
            database_path,
            project_id: project_id.to_owned(),
            project_store,
            _store_override: store_override,
            _temp: temp,
        })
    }

    fn scope(&self) -> Option<&str> {
        Some(&self.project_id)
    }

    fn add_project(&self, project_id: &str) -> Result<()> {
        create_project(&self.project_store, self._temp.path(), project_id)
    }

    fn legacy_database_path(&self) -> PathBuf {
        self.data_dir
            .join("storage/session/scoped")
            .join(path::session_scope_key(&self.project_id))
            .join("index.sqlite3")
    }

    fn seed_legacy_session(&self, session_id: &str) -> Result<()> {
        let legacy_path = self.legacy_database_path();
        fs::create_dir_all(legacy_path.parent().expect("legacy database parent"))?;
        let connection = Connection::open(legacy_path)?;
        connection.execute_batch(
            "CREATE TABLE session_index (
                 session_id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 created_ms INTEGER NOT NULL,
                 updated_ms INTEGER NOT NULL,
                 message_count INTEGER NOT NULL,
                 call_count INTEGER NOT NULL,
                 last_content TEXT
             );",
        )?;
        connection.execute(
            "INSERT INTO session_index (
                 session_id, title, created_ms, updated_ms, message_count, call_count, last_content
             ) VALUES (?1, 'legacy-only', 1, 2, 0, 0, NULL)",
            params![session_id],
        )?;
        Ok(())
    }
}

fn create_project(store: &ProjectStore, root: &Path, project_id: &str) -> Result<()> {
    create_project_for_tenant(store, &TenantScope::default(), root, project_id)
}

fn create_project_for_tenant(
    store: &ProjectStore,
    tenant: &TenantScope,
    root: &Path,
    project_id: &str,
) -> Result<()> {
    store.create(
        tenant,
        &project::Info {
            id: project_id.to_owned(),
            worktree: root.join(project_id).to_string_lossy().into_owned(),
            vcs: None,
            name: Some(project_id.to_owned()),
            icon: None,
            commands: None,
            time: project::TimeInfo { created: 1, updated: 2, initialized: None },
            sandboxes: Vec::new(),
        },
    )?;
    Ok(())
}

fn metadata_project(id: &str, worktree: &str) -> project::Info {
    project::Info {
        id: id.to_owned(),
        worktree: worktree.to_owned(),
        vcs: None,
        name: None,
        icon: None,
        commands: None,
        time: project::TimeInfo { created: 11, updated: 22, initialized: Some(33) },
        sandboxes: vec!["sandbox-a".to_owned()],
    }
}

#[test]
fn scope_project_metadata_uses_safe_scope_path_and_omits_absent_fields() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let scope = "../unsafe/project";
    let info = metadata_project("metadata-project", "/work/metadata-project");

    let metadata_path =
        write_scope_project_metadata(temp.path(), scope, &info).expect("metadata path");

    assert_eq!(
        metadata_path,
        temp.path()
            .join("storage/session/scoped")
            .join(path::session_scope_key(scope))
            .join("project.json")
    );
    let value: serde_json::Value = serde_json::from_slice(&fs::read(metadata_path)?)?;
    assert_eq!(value["scope"], scope);
    assert_eq!(value["scopeKey"], path::session_scope_key(scope));
    assert_eq!(value["projectId"], info.id);
    assert_eq!(value["directory"], info.worktree);
    assert_eq!(value["sandboxes"], serde_json::json!(["sandbox-a"]));
    assert_eq!(value["time"], serde_json::to_value(&info.time)?);
    assert!(value.get("name").is_none());
    assert!(value.get("vcs").is_none());
    Ok(())
}

#[test]
fn backfill_scope_project_metadata_writes_only_valid_project_files() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let session_data_dir = temp.path().join("session-data");
    let project_storage_dir = temp.path().join("project-storage");
    backfill_scope_project_metadata(&session_data_dir, &temp.path().join("missing-storage"));
    assert!(!session_data_dir.exists());

    fs::create_dir_all(&project_storage_dir)?;
    let valid = metadata_project("backfill-project", "/work/backfill-project");
    fs::write(project_storage_dir.join("valid.json"), serde_json::to_vec_pretty(&valid)?)?;
    fs::write(project_storage_dir.join("invalid.json"), b"{invalid json")?;
    fs::write(project_storage_dir.join("ignored.txt"), serde_json::to_vec_pretty(&valid)?)?;
    fs::create_dir(project_storage_dir.join("directory.json"))?;

    backfill_scope_project_metadata(&session_data_dir, &project_storage_dir);

    let metadata_path = scope_project_metadata_path(&session_data_dir, &valid.id)
        .expect("backfilled metadata path");
    let value: serde_json::Value = serde_json::from_slice(&fs::read(metadata_path)?)?;
    assert_eq!(value["projectId"], valid.id);
    let scoped_entries = fs::read_dir(session_data_dir.join("storage/session/scoped"))?
        .collect::<std::io::Result<Vec<_>>>()?;
    assert_eq!(scoped_entries.len(), 1);
    Ok(())
}

#[test]
fn facade_isolates_same_project_and_session_ids_across_tenants() -> Result<()> {
    let fixture = FacadeFixture::new("same-project")?;
    let tenant_a = TenantScope::new("platform-a", "company-a")?;
    let tenant_b = TenantScope::new("platform-b", "company-b")?;
    create_project_for_tenant(
        &fixture.project_store,
        &tenant_a,
        fixture._temp.path(),
        "same-project",
    )?;
    create_project_for_tenant(
        &fixture.project_store,
        &tenant_b,
        fixture._temp.path(),
        "same-project",
    )?;
    let mut session_a = test_session("same-session");
    session_a.title = "tenant A".to_owned();
    let mut session_b = test_session("same-session");
    session_b.title = "tenant B".to_owned();

    save_session_scoped_for_tenant(&tenant_a, &session_a, Some("same-project"))
        .expect("save tenant A session");
    save_session_scoped_for_tenant(&tenant_b, &session_b, Some("same-project"))
        .expect("save tenant B session");

    assert_eq!(
        load_session_scoped_for_tenant(&tenant_a, "same-session", Some("same-project"))
            .expect("load tenant A")
            .title,
        "tenant A"
    );
    assert_eq!(
        load_session_scoped_for_tenant(&tenant_b, "same-session", Some("same-project"))
            .expect("load tenant B")
            .title,
        "tenant B"
    );
    assert_eq!(
        load_session_any_for_tenant(&tenant_a, "same-session")
            .expect("find tenant A session")
            .title,
        "tenant A"
    );
    Ok(())
}

fn test_session(id: &str) -> ChatSession {
    ChatSession {
        id: id.to_string(),
        title: "Test session".to_string(),
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "hello".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some("msg-1".to_string())],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

fn test_agent_session(id: &str, project_id: &str, updated: u64) -> AgentSessionInfo {
    AgentSessionInfo {
        id: id.to_owned(),
        slug: format!("slug-{id}"),
        project_id: project_id.to_owned(),
        directory: format!("/work/{project_id}"),
        parent_id: Some("parent-session".to_owned()),
        summary: Some(Summary { additions: 4, deletions: 2, files: 1, diffs: None }),
        share: Some(ShareInfo { url: "https://example.test/session".to_owned() }),
        title: format!("Agent {id}"),
        version: "1".to_owned(),
        time: TimeInfo {
            created: updated.saturating_sub(1),
            updated,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: Some(RevertInfo {
            message_id: "message-1".to_owned(),
            part_id: None,
            snapshot: None,
            diff: None,
        }),
    }
}

#[test]
fn scoped_todos_round_trip_through_global_store() -> Result<()> {
    let fixture = FacadeFixture::new("ui-store-todos")?;
    let session_id = "ui-store-todos-session";
    let todos = vec![SessionTodoItem {
        id: "todo-1".to_string(),
        content: "cover ui store".to_string(),
        status: "pending".to_string(),
        priority: "medium".to_string(),
    }];
    save_session_scoped(&test_session(session_id), fixture.scope()).expect("session path");

    let path = save_session_todos_scoped(session_id, &todos, fixture.scope()).expect("todo path");

    assert_eq!(path, fixture.database_path);
    assert_eq!(
        serde_json::to_value(load_session_todos_scoped(session_id, fixture.scope()))?,
        serde_json::to_value(todos)?
    );
    assert!(!fixture.legacy_database_path().exists());
    Ok(())
}

#[test]
fn scoped_session_save_load_preview_append_and_delete_uses_global_database() -> Result<()> {
    let _global_state = GlobalStateGuard::new();
    let fixture = FacadeFixture::new("ui-store-session")?;
    let session_id = "ui-store-session-record";
    let scope = fixture.scope();
    let session = test_session(session_id);
    fixture.seed_legacy_session(session_id)?;
    assert!(load_session_scoped(session_id, scope).is_none());

    set_session_scope(scope);
    let path = save_session_scoped(&session, scope).expect("session path");
    assert_eq!(path, fixture.database_path);

    let loaded = load_session_scoped(session_id, scope).expect("loaded session");
    assert_eq!(loaded.title, "Test session");
    let preview = session_preview_meta(session_id).expect("preview meta");
    assert_eq!(preview.id, session_id);
    assert_eq!(preview.message_count, 1);

    let payload = serde_json::json!({"stream_id": 9, "delta": "first"});
    persist_ai_call_payload(session_id, 0, &payload, scope).expect("persist call");
    let replacement = serde_json::json!({"stream_id": 9, "delta": "replacement"});
    persist_ai_call_payload(session_id, 0, &replacement, scope).expect("replace call");
    let loaded = load_session_scoped(session_id, scope).expect("loaded session with call");
    assert_eq!(loaded.calls.len(), 1);
    assert_eq!(loaded.calls[0]["delta"], "replacement");

    delete_session_scoped(session_id, scope);
    assert!(load_session_scoped(session_id, scope).is_none());
    let legacy_connection = Connection::open(fixture.legacy_database_path())?;
    let legacy_title: String = legacy_connection.query_row(
        "SELECT title FROM session_index WHERE session_id = ?1",
        params![session_id],
        |row| row.get(0),
    )?;
    assert_eq!(legacy_title, "legacy-only");
    Ok(())
}

#[test]
fn native_session_operations_skip_missing_project_scope() -> Result<()> {
    let fixture = FacadeFixture::new("ui-store-missing-scope")?;
    let session = test_session("missing-scope");

    for missing_scope in [None, Some(""), Some(" \t ")] {
        assert!(save_session_scoped(&session, missing_scope).is_none());
        assert!(save_session_todos_scoped(&session.id, &[], missing_scope).is_none());
        assert!(load_session_scoped(&session.id, missing_scope).is_none());
        assert!(load_session_todos_scoped(&session.id, missing_scope).is_empty());
        assert!(load_sessions_scoped(missing_scope).is_empty());
    }
    assert!(!fixture.data_dir.join("storage/session/index.sqlite3").exists());
    Ok(())
}

#[test]
fn fallible_ui_save_rejects_missing_project_scope() -> Result<()> {
    let fixture = FacadeFixture::new("ui-store-fallible-save")?;
    let session = test_session("missing-fallible-scope");

    let error = save_session_ui_scoped_for_tenant(&TenantScope::default(), &session, None, true)
        .expect_err("missing scope must fail the UI save");

    assert!(error.to_string().contains("non-empty project scope"));
    assert!(!fixture.data_dir.join("storage/session/index.sqlite3").exists());
    Ok(())
}

#[test]
fn facade_lists_and_finds_sessions_and_agent_metadata_across_local_projects() -> Result<()> {
    let _global_state = GlobalStateGuard::new();
    let fixture = FacadeFixture::new("facade-project-a")?;
    fixture.add_project("facade-project-b")?;
    let scope_a = fixture.scope();
    let scope_b = Some("facade-project-b");

    save_session_scoped(&test_session("session-a"), scope_a).expect("project A session path");
    save_session_scoped(&test_session("session-b"), scope_b).expect("project B session path");
    assert_eq!(load_sessions_scoped(scope_a)[0].id, "session-a");
    assert_eq!(load_sessions_scoped(scope_b)[0].id, "session-b");
    assert_eq!(load_session_any("session-b").expect("cross-project session").id, "session-b");

    let agent_a = test_agent_session("agent-a", "facade-project-a", 10);
    let agent_b = test_agent_session("agent-b", "facade-project-b", 20);
    assert_eq!(
        save_agent_session_scoped(&agent_a, scope_a).expect("project A agent path"),
        fixture.database_path
    );
    save_agent_session_scoped(&agent_b, scope_b).expect("project B agent path");
    assert_eq!(
        load_agent_session_scoped("agent-a", scope_a).expect("scoped agent").project_id,
        "facade-project-a"
    );
    assert_eq!(load_agent_sessions_scoped(scope_b)[0].id, "agent-b");
    assert_eq!(load_agent_session_any("agent-b").expect("cross-project agent").id, "agent-b");
    assert_eq!(
        load_agent_sessions_all().iter().map(|info| info.id.as_str()).collect::<Vec<_>>(),
        vec!["agent-b", "agent-a"]
    );

    delete_agent_session_scoped("agent-a", scope_a);
    assert!(load_agent_session_scoped("agent-a", scope_a).is_none());
    assert!(load_agent_session_scoped("agent-b", scope_b).is_some());
    Ok(())
}

#[test]
fn agent_session_save_uses_info_project_id_when_scope_is_omitted() -> Result<()> {
    let fixture = FacadeFixture::new("agent-fallback-project")?;
    let agent = test_agent_session("agent-fallback", "agent-fallback-project", 10);

    let path = save_agent_session_scoped(&agent, None).expect("agent session path");

    assert_eq!(path, fixture.database_path);
    assert_eq!(
        load_agent_session_scoped(&agent.id, fixture.scope())
            .expect("agent session saved through fallback scope")
            .project_id,
        fixture.project_id
    );
    Ok(())
}

#[test]
fn ai_call_payload_uses_stream_argument_and_appends_zero_stream_payloads() -> Result<()> {
    let fixture = FacadeFixture::new("ai-call-project")?;
    let session_id = "ai-call-session";
    let mut session = test_session(session_id);
    session.calls.push(serde_json::json!({"stream_id": 17, "delta": "original"}));
    save_session_scoped(&session, fixture.scope()).expect("session path");

    let replacement = serde_json::json!({"delta": "replacement-from-argument"});
    persist_ai_call_payload(session_id, 17, &replacement, fixture.scope())
        .expect("replace call through stream argument");
    let zero_stream = serde_json::json!({"delta": "zero-stream"});
    persist_ai_call_payload(session_id, 0, &zero_stream, fixture.scope())
        .expect("append zero stream call");

    let loaded = load_session_scoped(session_id, fixture.scope()).expect("session with calls");
    assert_eq!(loaded.calls, vec![replacement, zero_stream]);
    Ok(())
}

#[test]
fn current_scope_wrappers_save_snapshot_append_call_and_delete_session() -> Result<()> {
    let _global_state = GlobalStateGuard::new();
    let fixture = FacadeFixture::new("current-scope-wrappers")?;
    let session_id = "current-scope-session";
    set_session_scope(fixture.scope());

    let path = save_session_snapshot(&test_session(session_id)).expect("snapshot path");
    assert_eq!(path, fixture.database_path);

    let payload = serde_json::json!({"kind": "tool", "result": "ok"});
    append_session_call(session_id, &payload).expect("append call");

    let loaded = load_session(session_id).expect("load through current scope");
    assert_eq!(loaded.calls, vec![payload]);

    delete_session(session_id);
    assert!(load_session(session_id).is_none());
    Ok(())
}

#[test]
fn current_scope_path_raw_todo_index_and_archive_wrappers_round_trip() -> Result<()> {
    let _global_state = GlobalStateGuard::new();
    let fixture = FacadeFixture::new("current-scope-storage-wrappers")?;
    let session_id = format!("current-scope-storage-{}", std::process::id());
    let session = test_session(&session_id);
    set_session_scope(fixture.scope());

    assert_eq!(session_file_path(&session_id), Some(fixture.database_path.clone()));

    let snapshot_path = save_session_step_snapshot(&session, 7, "finish").expect("snapshot path");
    assert_eq!(
        session_step_snapshot_file_path(&session_id, 7, "finish"),
        Some(snapshot_path.clone())
    );
    assert!(snapshot_path.exists());

    let payload = serde_json::json!({"request": "raw", "stream_id": 41});
    let raw_path =
        persist_llm_raw_step(&session_id, 8, &payload, fixture.scope()).expect("raw path");
    assert_eq!(raw_path, fixture.database_path);
    assert_eq!(load_llm_raw_step_scoped(&session_id, 8, fixture.scope())?, Some(payload));

    save_session(&session).expect("session path");
    let todos = vec![SessionTodoItem {
        id: "todo-current-scope".to_owned(),
        content: "cover current scope wrappers".to_owned(),
        status: "pending".to_owned(),
        priority: "medium".to_owned(),
    }];
    assert_eq!(save_session_todos(&session_id, &todos), Some(fixture.database_path.clone()));
    assert_eq!(
        serde_json::to_value(load_session_todos(&session_id))?,
        serde_json::to_value(&todos)?
    );

    let mut sessions = load_sessions();
    assert_eq!(sessions.len(), 1);
    sessions[0].title = "Reindexed by current scope".to_owned();
    save_sessions(&sessions);
    assert_eq!(load_sessions()[0].title, "Reindexed by current scope");

    let mut archived_ids = HashSet::new();
    archived_ids.insert(session_id.clone());
    save_archived_session_ids(&archived_ids);
    assert!(load_archived_session_ids().contains(&session_id));
    assert_eq!(
        resolve_session_scope_id(None, Some("  current-scope-storage-wrappers  ")).as_deref(),
        Some("current-scope-storage-wrappers")
    );
    assert_eq!(resolve_session_scope_id(None, Some("  ")), None);

    let _ = fs::remove_file(snapshot_path);
    Ok(())
}

#[test]
fn runtime_session_file_path_initializes_and_reuses_default_test_store() {
    let _global_state = GlobalStateGuard::new();

    let first = session_file_path("runtime-store-first").expect("first runtime store path");
    let second = session_file_path("runtime-store-second").expect("cached runtime store path");

    assert_eq!(first, second);
    let file_name = first.file_name().and_then(|name| name.to_str()).expect("database file name");
    assert!(file_name.starts_with("vw-agent-test-global-store-"));
    assert!(file_name.ends_with(".sqlite3"));
}

#[test]
fn unscoped_archive_wrappers_use_default_archive_key() {
    let _global_state = GlobalStateGuard::new();
    let mut ids = HashSet::new();
    ids.insert("unscoped-archive".to_owned());

    save_archived_session_ids(&ids);

    assert!(load_archived_session_ids().contains("unscoped-archive"));
    assert!(load_archived_session_ids_scoped(Some("other-scope")).is_empty());
}

#[test]
fn postgres_session_store_round_trips_todos_and_agent_metadata() -> Result<()> {
    let _global_state = GlobalStateGuard::new();
    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let database = postgres_fixture.database().clone();
    let project_store = ProjectStore::new(database.clone())?;
    create_project(&project_store, Path::new("/work"), "postgres-session-project")?;
    let store = SessionStore::new(database)?;
    let _store_override = override_session_store_for_test(store.clone());
    let scope =
        ProjectScope::new(TenantScope::default(), ProjectId::from("postgres-session-project"))?;
    let session = test_session("postgres-session");
    let todos = vec![SessionTodoItem {
        id: "postgres-todo".to_owned(),
        content: "round trip".to_owned(),
        status: "pending".to_owned(),
        priority: "medium".to_owned(),
    }];
    let agent = test_agent_session("postgres-agent", "postgres-session-project", 10);

    store.save_session(&scope, &session)?;
    store.save_session_todos(&scope, &session.id, &todos)?;
    store.save_agent_session(&scope, &agent)?;

    assert!(session_file_path(&session.id).is_none());
    assert!(
        save_session_todos_scoped(&session.id, &todos, Some("postgres-session-project")).is_some()
    );
    assert_eq!(
        serde_json::to_value(store.load_session_todos(&scope, &session.id)?)?,
        serde_json::to_value(todos)?
    );
    assert_eq!(
        serde_json::to_value(store.load_agent_session(&scope, &agent.id)?)?,
        serde_json::to_value(Some(agent))?
    );
    postgres_fixture.cleanup()?;
    Ok(())
}

#[test]
fn archived_session_ids_are_scoped_by_project_key() {
    let mut ids = std::collections::HashSet::new();
    ids.insert("archived-one".to_string());
    let scope = Some("project/archive-scope");

    save_archived_session_ids_scoped(&ids, scope);
    assert!(load_archived_session_ids_scoped(scope).contains("archived-one"));
    assert!(
        !load_archived_session_ids_scoped(Some("project/other-scope")).contains("archived-one")
    );
}
