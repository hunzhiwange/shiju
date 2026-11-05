//! 验证 PostgreSQL 会话存储拒绝 SQLite 后端的边界。
//! 这些用例故意传入 SQLite，证明后端检查发生在建表和数据访问之前。

use super::*;
use crate::project::store::ProjectStore;
use vw_shared::project::{Info as ProjectInfo, TimeInfo as ProjectTimeInfo};

fn sqlite_database() -> (tempfile::TempDir, GlobalDatabase) {
    let home = tempfile::tempdir().expect("temporary database home");
    let database =
        GlobalDatabase::new(home.path().join("global.sqlite3")).expect("SQLite global database");
    (home, database)
}

fn project_scope() -> ProjectScope {
    ProjectScope::new(TenantScope::default(), ProjectId::from("postgres-session-tests"))
        .expect("valid project scope")
}

fn empty_session() -> ChatSession {
    ChatSession {
        id: "session-id".to_string(),
        title: "Session".to_string(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    }
}

fn assert_requires_postgres<T>(result: Result<T>) {
    let error = match result {
        Ok(_) => panic!("operation must reject a SQLite backend"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("SQLite global database does not expose a PostgreSQL connection"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn new_rejects_sqlite_backend_before_initializing_postgres_schema() {
    let (_home, database) = sqlite_database();

    let error = PostgresSessionStore::new(database)
        .expect_err("PostgreSQL store must reject a SQLite backend");

    assert!(
        error.to_string().contains("SQLite global database has no PostgreSQL schema transaction")
    );
}

#[test]
// 后端类型错误必须先于任何会话查询返回，避免在不兼容数据库上产生部分副作用。
fn session_operations_reject_sqlite_backend_before_accessing_session_data() {
    let (_home, database) = sqlite_database();
    let store = PostgresSessionStore { database };
    let scope = project_scope();
    let session = empty_session();

    assert_requires_postgres(store.save_session(&scope, &session));
    assert_requires_postgres(store.load_session(&scope, &session.id));
    assert_requires_postgres(store.load_session_message_page(&scope, &session.id, None, 64));
    assert_requires_postgres(store.apply_session_message_delta(
        &scope,
        &SessionMessageDelta {
            session_id: session.id.clone(),
            expected_updated_ms: 1,
            updated_ms: 2,
            upserts: Vec::new(),
            truncate_after: None,
        },
    ));
    assert_requires_postgres(store.load_session_any(scope.tenant_scope(), &session.id));
    assert_requires_postgres(store.list_sessions(&scope));
    assert_requires_postgres(store.load_session_meta(&scope, &session.id));
    assert_requires_postgres(store.replace_session_index(&scope, &[]));
    assert_requires_postgres(store.delete_session(&scope, &session.id));
    assert_requires_postgres(store.save_session_todos(&scope, &session.id, &[]));
    assert_requires_postgres(store.load_session_todos(&scope, &session.id));
}

#[test]
fn agent_session_operations_reject_sqlite_backend_before_querying_rows() {
    let (_home, database) = sqlite_database();
    let store = PostgresSessionStore { database };
    let scope = project_scope();
    let tenant = scope.tenant_scope();

    assert_requires_postgres(store.load_agent_session(&scope, "agent-session"));
    assert_requires_postgres(store.load_agent_sessions(&scope));
    assert_requires_postgres(store.load_agent_sessions_all(tenant));
    assert_requires_postgres(store.load_agent_session_any(tenant, "agent-session"));
    assert_requires_postgres(store.delete_agent_session(&scope, "agent-session"));
}

#[test]
fn stale_ui_content_save_preserves_newer_messages() -> Result<()> {
    let postgres_fixture = crate::global_database::test_support::postgres_test_database!();
    let database = postgres_fixture.database().clone();
    let tenant = TenantScope::new("postgres-platform", "postgres-company")?;
    let scope = ProjectScope::new(tenant.clone(), ProjectId::from("stale-ui-content"))?;
    ProjectStore::new(database.clone())?.create(
        &tenant,
        &ProjectInfo {
            id: "stale-ui-content".to_owned(),
            worktree: "/work/stale-ui-content".to_owned(),
            vcs: None,
            name: Some("Stale UI content".to_owned()),
            icon: None,
            commands: None,
            time: ProjectTimeInfo { created: 1, updated: 2, initialized: None },
            sandboxes: Vec::new(),
        },
    )?;
    let store = PostgresSessionStore::new(database)?;
    let mut newer = empty_session();
    newer.updated_ms = 200;
    newer.messages.push(ChatMessage {
        role: super::super::ChatRole::User,
        content: "new tail".to_owned(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    newer.message_ids.push(Some("message-new".to_owned()));
    store.save_session(&scope, &newer)?;

    let mut stale = empty_session();
    stale.updated_ms = 100;
    let error = store
        .save_session_ui_content(&scope, &stale)
        .expect_err("an older PostgreSQL UI snapshot must be rejected");

    assert!(error.to_string().contains("stale session UI content"));
    assert_eq!(
        serde_json::to_value(store.load_session(&scope, &newer.id)?)?,
        serde_json::to_value(Some(newer))?
    );
    postgres_fixture.cleanup()?;
    Ok(())
}

#[test]
fn dynamic_tools_persistence_failed_write_keeps_running_catalog() -> Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    crate::session::dynamic_tool_store_tests::failed_write(fixture.database().clone())?;
    fixture.cleanup()
}

#[test]
fn dynamic_tools_persistence_success_and_explicit_empty() -> Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    crate::session::dynamic_tool_store_tests::success_and_empty(fixture.database().clone())?;
    fixture.cleanup()
}

#[test]
fn dynamic_tools_persistence_rejects_untrusted_and_invalid() -> Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    crate::session::dynamic_tool_store_tests::rejects_untrusted_and_invalid(
        fixture.database().clone(),
    )?;
    fixture.cleanup()
}

#[test]
fn dynamic_tools_persistence_scopes_and_stale_versions_are_isolated() -> Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    crate::session::dynamic_tool_store_tests::isolation_and_stale(fixture.database().clone())?;
    fixture.cleanup()
}

#[test]
fn dynamic_tools_persistence_concurrent_replacements_follow_commit_order() -> Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    crate::session::dynamic_tool_store_tests::concurrent(fixture.database().clone())?;
    fixture.cleanup()
}

#[test]
fn dynamic_tools_persistence_native_index_preserves_existing_ui() -> Result<()> {
    let fixture = crate::global_database::test_support::PostgresTestDatabase::from_environment()?;
    let (store, scope) = crate::session::dynamic_tool_store_tests::fixture(
        fixture.database().clone(),
        "native-index",
    )?;
    let info: AgentSessionInfo = serde_json::from_value(serde_json::json!({
        "id":"native-only", "slug":"native", "projectID":scope.project_id().as_ref(),
        "directory":"/work/native-index/project", "title":"Native title", "version":"1",
        "time":{"created":1,"updated":2}
    }))?;
    store.save_agent_session(&scope, &info)?;
    assert!(store.load_session_meta(&scope, &info.id)?.is_none());
    store.ensure_native_session_index(&scope, &info)?;
    assert_eq!(store.load_session_meta(&scope, &info.id)?.unwrap().title, "Native title");
    let mut ui = empty_session();
    ui.id = info.id.clone();
    ui.title = "UI title".into();
    ui.updated_ms = 42;
    store.save_session(&scope, &ui)?;
    store.ensure_native_session_index(&scope, &info)?;
    let meta = store.load_session_meta(&scope, &info.id)?.unwrap();
    assert_eq!(meta.title, "UI title");
    assert_eq!(meta.updated_ms, 42);
    fixture.cleanup()
}
