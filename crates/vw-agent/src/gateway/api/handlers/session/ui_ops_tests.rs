use super::*;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::session::ui_store;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, HeaderValue};
use rusqlite::{Connection, params};
use std::path::Path as FsPath;
use std::sync::Once;
use tempfile::TempDir;
use vw_api_types::session::{
    GatewaySessionScopeBody, GatewaySessionTodoItem, GatewaySessionTodoPutBody,
};
use vw_api_types::todo::{TodoPriority, TodoStatus};
use vw_shared::session::info::{Info as AgentSessionInfo, TimeInfo as AgentSessionTimeInfo};

static TEST_HOME: Once = Once::new();
static UI_OPS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn ensure_test_home() {
    TEST_HOME.call_once(|| {
        let dir = ui_ops_test_home();
        std::fs::create_dir_all(&dir).expect("test home should be created");
        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", dir);
        }
    });
}

fn ui_ops_test_home() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("vw-ui-ops-tests-{}", std::process::id()))
}

fn query_for(directory: &FsPath) -> InstanceQuery {
    InstanceQuery { directory: Some(directory.to_string_lossy().to_string()) }
}

fn headers_for(directory: &FsPath) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-vibewindow-directory",
        HeaderValue::from_str(&directory.to_string_lossy()).expect("valid directory header"),
    );
    headers
}

fn session(id: &str, title: &str) -> ui_models::ChatSession {
    scoped_session(id, title, "hello", 20)
}

fn scoped_session(id: &str, title: &str, content: &str, updated_ms: u64) -> ui_models::ChatSession {
    ui_models::ChatSession {
        id: id.to_string(),
        title: title.to_string(),
        messages: vec![ui_models::ChatMessage {
            role: ui_models::ChatRole::User,
            content: content.to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }],
        message_ids: vec![Some("message-1".to_string())],
        calls: vec![serde_json::json!({"tool": "noop"})],
        steps: vec![ui_models::ChatSessionStep {
            index: 0,
            started_ms: 1,
            finished_ms: Some(2),
            start_snapshot_path: None,
            finish_snapshot_path: None,
            usage: ui_models::TokenUsage {
                input_tokens: 3,
                output_tokens: 5,
                cached_tokens: 0,
                reasoning_tokens: 1,
            },
            cost_usd: Some(0.01),
            finish_reason: Some("stop".to_string()),
            model: Some("test-model".to_string()),
            tool_timings: Vec::new(),
        }],
        created_ms: 10,
        updated_ms,
    }
}

fn agent_session(id: &str, project_id: &str, title: &str, updated: u64) -> AgentSessionInfo {
    AgentSessionInfo {
        id: id.to_string(),
        slug: format!("slug-{project_id}"),
        project_id: project_id.to_string(),
        directory: format!("/work/{project_id}"),
        parent_id: None,
        summary: None,
        share: None,
        title: title.to_string(),
        version: "test".to_string(),
        time: AgentSessionTimeInfo {
            created: updated.saturating_sub(1),
            updated,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

fn old_project_database_path(scope: &str) -> std::path::PathBuf {
    vw_config_types::paths::home_config_dir(&crate::global::paths().home)
        .join("storage/session/scoped")
        .join(vw_shared::session::path::session_scope_key(scope))
        .join("index.sqlite3")
}

#[test]
fn ui_handlers_are_available() {
    let _ = session_ui_get;
    let _ = session_ui_save;
    let _ = session_ui_previews;
    let _ = session_ui_preview_meta;
    let _ = session_archived_get;
    let _ = session_archived_put;
    let _ = session_path_get;
    let _ = session_scope_get;
    let _ = session_scope_put;
    let _ = session_ui_get_any;
    let _ = session_step_llm_raw_get;
}

#[tokio::test(flavor = "multi_thread")]
async fn session_step_llm_raw_get_returns_requested_step_and_null_for_missing() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();
    let body = scoped_session("single-step-raw", "Single step", "hello", 20);
    let Json(saved) = session_ui_save(
        Path(body.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(body.clone()),
    )
    .await
    .expect("session should save");
    assert!(saved);
    let scope = ui_store::resolve_session_scope_id(Some(&project_path.to_string_lossy()), None)
        .expect("project directory should resolve to a scope");
    ui_store::persist_llm_raw_step(&body.id, 7, &serde_json::json!({"step": 7}), Some(&scope))
        .expect("step 7 raw payload should persist");
    ui_store::persist_llm_raw_step(&body.id, 8, &serde_json::json!({"step": 8}), Some(&scope))
        .expect("step 8 raw payload should persist");

    let Json(with_raw_indices) =
        session_ui_get(Path(body.id.clone()), Query(query_for(project_path)), HeaderMap::new())
            .await
            .expect("session UI should load raw step indices");
    assert_eq!(
        with_raw_indices.expect("session exists").llm_raw_step_indices,
        BTreeSet::from([7, 8])
    );

    let Json(found) = session_step_llm_raw_get(
        Path((body.id.clone(), 7)),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("requested raw payload should load");
    let Json(missing) = session_step_llm_raw_get(
        Path((body.id, 9)),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("missing raw payload should return null");

    assert_eq!(found, Some(serde_json::json!({"step": 7})));
    assert_eq!(missing, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn session_step_llm_raw_get_isolates_project_scope() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project_a = TempDir::new().expect("project A dir");
    let project_b = TempDir::new().expect("project B dir");
    let session_id = "scope-isolated-raw";
    for (project, value) in [(&project_a, "project-a"), (&project_b, "project-b")] {
        let body = scoped_session(session_id, value, value, 20);
        let Json(saved) = session_ui_save(
            Path(body.id.clone()),
            Query(query_for(project.path())),
            HeaderMap::new(),
            Json(body),
        )
        .await
        .expect("scoped session should save");
        assert!(saved);
        let scope =
            ui_store::resolve_session_scope_id(Some(&project.path().to_string_lossy()), None)
                .expect("project directory should resolve to a scope");
        ui_store::persist_llm_raw_step(
            session_id,
            3,
            &serde_json::json!({"project": value}),
            Some(&scope),
        )
        .expect("scoped raw payload should persist");
    }

    let Json(raw_a) = session_step_llm_raw_get(
        Path((session_id.to_string(), 3)),
        Query(query_for(project_a.path())),
        HeaderMap::new(),
    )
    .await
    .expect("project A raw payload should load");
    let Json(raw_b) = session_step_llm_raw_get(
        Path((session_id.to_string(), 3)),
        Query(query_for(project_b.path())),
        HeaderMap::new(),
    )
    .await
    .expect("project B raw payload should load");

    assert_eq!(raw_a, Some(serde_json::json!({"project": "project-a"})));
    assert_eq!(raw_b, Some(serde_json::json!({"project": "project-b"})));
}

#[tokio::test(flavor = "multi_thread")]
async fn session_step_llm_raw_get_isolates_tenants() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project = TempDir::new().expect("shared project dir");
    let session_id = "tenant-isolated-raw";
    let tenant_a =
        vw_api_types::id::TenantScope::new("platform-raw-a", "company-raw-a").expect("tenant A");
    let tenant_b =
        vw_api_types::id::TenantScope::new("platform-raw-b", "company-raw-b").expect("tenant B");

    for (tenant, value) in [(tenant_a.clone(), "tenant-a"), (tenant_b.clone(), "tenant-b")] {
        ui_store::with_tenant_scope(tenant.clone(), async {
            let body = scoped_session(session_id, value, value, 20);
            let Json(saved) = session_ui_save(
                Path(body.id.clone()),
                Query(query_for(project.path())),
                HeaderMap::new(),
                Json(body),
            )
            .await
            .expect("tenant session should save");
            assert!(saved);
            let scope = ui_store::resolve_session_scope_id_for_tenant(
                &tenant,
                Some(&project.path().to_string_lossy()),
                None,
            )
            .expect("tenant project scope should resolve");
            ui_store::persist_llm_raw_step(
                session_id,
                4,
                &serde_json::json!({"tenant": value}),
                Some(&scope),
            )
            .expect("tenant raw payload should persist");
        })
        .await;
    }

    let raw_a = ui_store::with_tenant_scope(
        tenant_a,
        session_step_llm_raw_get(
            Path((session_id.to_string(), 4)),
            Query(query_for(project.path())),
            HeaderMap::new(),
        ),
    )
    .await
    .expect("tenant A raw payload should load")
    .0;
    let raw_b = ui_store::with_tenant_scope(
        tenant_b,
        session_step_llm_raw_get(
            Path((session_id.to_string(), 4)),
            Query(query_for(project.path())),
            HeaderMap::new(),
        ),
    )
    .await
    .expect("tenant B raw payload should load")
    .0;

    assert_eq!(raw_a, Some(serde_json::json!({"tenant": "tenant-a"})));
    assert_eq!(raw_b, Some(serde_json::json!({"tenant": "tenant-b"})));
}

#[tokio::test(flavor = "multi_thread")]
async fn session_step_llm_raw_get_maps_storage_error_to_internal_error() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();
    let scope = ui_store::resolve_session_scope_id(Some(&project_path.to_string_lossy()), None)
        .expect("project directory should resolve to a scope");
    let session_id = format!("invalid-single-step-raw-{}", std::process::id());
    let body = scoped_session(&session_id, "Invalid JSON", "hello", 20);
    let Json(saved) = session_ui_save(
        Path(session_id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(body),
    )
    .await
    .expect("session should save");
    assert!(saved);
    ui_store::persist_llm_raw_step(
        &session_id,
        7,
        &serde_json::json!({"valid": true}),
        Some(&scope),
    )
    .expect("raw payload should persist before injection");
    let tenant = ui_store::current_tenant_scope();
    let connection =
        Connection::open(ui_store::session_file_path(&session_id).expect("session store path"))
            .expect("open session store database");
    connection
        .execute(
            "UPDATE session_step_llm_raw
             SET payload_json = ?1
             WHERE platform_id = ?2 AND company_id = ?3 AND project_id = ?4
               AND session_id = ?5 AND step_index = ?6",
            params![
                "{definitely not json",
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                &scope,
                &session_id,
                7i64,
            ],
        )
        .expect("raw payload should be corrupted for the error case");

    let error = session_step_llm_raw_get(
        Path((session_id, 7)),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect_err("invalid raw payload should fail");

    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_snapshot_handlers_round_trip_scoped_sessions_and_metadata() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project_a = TempDir::new().expect("project A dir");
    let project_b = TempDir::new().expect("project B dir");
    let project_a_path = project_a.path();
    let project_b_path = project_b.path();
    let body_a = scoped_session("ui-round-trip", "Project A", "project-a", 20);
    let body_b = scoped_session("ui-round-trip", "Project B", "project-b", 30);

    let Json(saved_a) = session_ui_save(
        Path(body_a.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(body_a.clone()),
    )
    .await
    .expect("project A session save should succeed");
    let Json(saved_b) = session_ui_save(
        Path(body_b.id.clone()),
        Query(query_for(project_b_path)),
        HeaderMap::new(),
        Json(body_b.clone()),
    )
    .await
    .expect("project B session save should succeed");
    assert!(saved_a && saved_b);

    let Json(loaded_a) =
        session_ui_get(Path(body_a.id.clone()), Query(query_for(project_a_path)), HeaderMap::new())
            .await
            .expect("project A session get should succeed");
    let Json(loaded_b) =
        session_ui_get(Path(body_b.id.clone()), Query(query_for(project_b_path)), HeaderMap::new())
            .await
            .expect("project B session get should succeed");
    assert_eq!(loaded_a.expect("project A session").messages[0].content, "project-a");
    assert_eq!(loaded_b.expect("project B session").messages[0].content, "project-b");

    let Json(previews_a) =
        session_ui_previews(Query(InstanceQuery { directory: None }), headers_for(project_a_path))
            .await
            .expect("project A previews should load");
    let Json(previews_b) =
        session_ui_previews(Query(InstanceQuery { directory: None }), headers_for(project_b_path))
            .await
            .expect("project B previews should load");
    assert_eq!(previews_a[0].title, body_a.title);
    assert_eq!(previews_a[0].last_content.as_deref(), Some("project-a"));
    assert_eq!(previews_b[0].title, body_b.title);
    assert_eq!(previews_b[0].last_content.as_deref(), Some("project-b"));

    let scope_a = ui_store::resolve_session_scope_id(Some(&project_a_path.to_string_lossy()), None)
        .expect("project A directory should resolve to a scope");
    let scope_b = ui_store::resolve_session_scope_id(Some(&project_b_path.to_string_lossy()), None)
        .expect("project B directory should resolve to a scope");
    assert_ne!(scope_a, scope_b);

    let todo_a = GatewaySessionTodoItem {
        id: "todo-shared".to_string(),
        content: "project-a-todo".to_string(),
        status: TodoStatus::Pending,
        priority: TodoPriority::Medium,
    };
    let todo_b = GatewaySessionTodoItem { content: "project-b-todo".to_string(), ..todo_a.clone() };
    let Json(saved_todos_a) = super::super::session_todo_put(
        Path(body_a.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: vec![todo_a.clone()] }),
    )
    .await
    .expect("project A todos should save");
    let Json(saved_todos_b) = super::super::session_todo_put(
        Path(body_b.id.clone()),
        Query(query_for(project_b_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: vec![todo_b.clone()] }),
    )
    .await
    .expect("project B todos should save");
    assert!(saved_todos_a && saved_todos_b);
    let Json(todos_a) = super::super::session_todo_get(
        Path(body_a.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
    )
    .await
    .expect("project A todos should load");
    let Json(todos_b) = super::super::session_todo_get(
        Path(body_b.id.clone()),
        Query(query_for(project_b_path)),
        HeaderMap::new(),
    )
    .await
    .expect("project B todos should load");
    let mut expected_todo_a = todo_a;
    expected_todo_a.id = "1".to_string();
    let mut expected_todo_b = todo_b;
    expected_todo_b.id = "1".to_string();
    assert_eq!(todos_a, vec![expected_todo_a]);
    assert_eq!(todos_b, vec![expected_todo_b]);

    let agent_a = agent_session("agent-shared", &scope_a, "Agent A", 20);
    let agent_b = agent_session("agent-shared", &scope_b, "Agent B", 30);
    ui_store::save_agent_session_scoped(&agent_a, Some(&scope_a))
        .expect("project A agent metadata path");
    ui_store::save_agent_session_scoped(&agent_b, Some(&scope_b))
        .expect("project B agent metadata path");
    assert_eq!(
        ui_store::load_agent_session_scoped("agent-shared", Some(&scope_a))
            .expect("project A agent metadata")
            .title,
        "Agent A"
    );
    assert_eq!(
        ui_store::load_agent_session_scoped("agent-shared", Some(&scope_b))
            .expect("project B agent metadata")
            .title,
        "Agent B"
    );

    ui_store::persist_llm_raw_step(
        &body_a.id,
        0,
        &serde_json::json!({"messages": [{"role": "user", "content": "hello"}]}),
        Some(&scope_a),
    )
    .expect("raw payload should persist");
    let Json(with_raw) =
        session_ui_get(Path(body_a.id.clone()), Query(query_for(project_a_path)), HeaderMap::new())
            .await
            .expect("raw payload should load");
    let steps = with_raw.expect("session with raw").steps;
    assert_eq!(
        steps[0].llm_raw.as_ref().expect("step raw payload")["messages"][0]["content"],
        "hello"
    );
    ui_store::set_session_scope(Some(&scope_b));

    let Json(meta) =
        session_ui_preview_meta(Path(body_b.id.clone())).await.expect("preview meta should load");
    assert_eq!(meta.expect("preview meta").title, body_b.title);

    let Json(path) =
        session_path_get(Path(body_b.id.clone())).await.expect("session path should load");
    let path = std::path::PathBuf::from(path.expect("session path"));
    assert!(path.exists());
    let connection = rusqlite::Connection::open(&path).expect("open shared database");
    let scoped_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM session_index
             WHERE platform_id = ?1 AND company_id = ?2 AND session_id = ?3",
            rusqlite::params![
                vw_api_types::id::DEFAULT_PLATFORM_ID,
                vw_api_types::id::DEFAULT_COMPANY_ID,
                &body_a.id,
            ],
            |row| row.get(0),
        )
        .expect("count scoped session rows");
    assert_eq!(scoped_rows, 2);
    assert!(!old_project_database_path(&scope_a).exists());
    assert!(!old_project_database_path(&scope_b).exists());

    let Json(any_session) =
        session_ui_get_any(Path(body_b.id.clone())).await.expect("any session should load");
    assert_eq!(any_session.expect("any session").title, body_b.title);

    ui_store::set_session_scope(None);
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_calls_in_ui_snapshot_preserve_existing_raw_call_records() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();
    let initial = scoped_session("preserve-calls", "Session", "initial", 20);
    let expected_calls = initial.calls.clone();
    let Json(initial_saved) = session_ui_save(
        Path(initial.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(initial.clone()),
    )
    .await
    .expect("initial session save should succeed");
    assert!(initial_saved);

    let mut snapshot = initial;
    snapshot.messages[0].content = "updated".to_string();
    snapshot.calls.clear();
    let Json(snapshot_saved) = session_ui_save(
        Path(snapshot.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(snapshot),
    )
    .await
    .expect("UI snapshot save should succeed");
    assert!(snapshot_saved);

    let Json(loaded) = session_ui_get(
        Path("preserve-calls".to_string()),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("saved session should load");
    assert_eq!(loaded.expect("saved session").calls, expected_calls);
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_snapshot_handlers_cover_archived_scope_and_errors() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();

    let mismatch = session_ui_save(
        Path("route-id".to_string()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(session("body-id", "Mismatch")),
    )
    .await
    .expect_err("body id mismatch should fail");
    assert_eq!(mismatch.status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(mismatch.to_string(), "body.id does not match session_id");

    let Json(saved_archived) = session_archived_put(
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(vec!["archived-a".to_string(), "archived-a".to_string(), "archived-b".to_string()]),
    )
    .await
    .expect("archived ids should save");
    assert!(saved_archived);

    let Json(mut archived) = session_archived_get(Query(query_for(project_path)), HeaderMap::new())
        .await
        .expect("archived ids should load");
    archived.sort();
    assert_eq!(archived, vec!["archived-a".to_string(), "archived-b".to_string()]);

    let Json(initial_scope) = session_scope_get().await.expect("current scope should load");
    assert_eq!(initial_scope, None);

    let Json(scope_saved) = session_scope_put(Json(GatewaySessionScopeBody {
        scope: Some("manual-scope".to_string()),
    }))
    .await
    .expect("scope should save");
    assert!(scope_saved);

    let Json(scope) = session_scope_get().await.expect("scope should load");
    assert_eq!(scope.as_deref(), Some("manual-scope"));

    let Json(cleared) = session_scope_put(Json(GatewaySessionScopeBody { scope: None }))
        .await
        .expect("scope should clear");
    assert!(cleared);
    assert_eq!(session_scope_get().await.expect("scope get").0, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_snapshot_handlers_return_empty_results_for_unknown_session() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();
    let session_id = format!("missing-ui-session-{}", std::process::id());

    let Json(loaded) =
        session_ui_get(Path(session_id.clone()), Query(query_for(project_path)), HeaderMap::new())
            .await
            .expect("unknown scoped session should be handled");
    assert!(loaded.is_none());

    let Json(previews) = session_ui_previews(Query(query_for(project_path)), HeaderMap::new())
        .await
        .expect("empty scoped previews should be handled");
    assert!(previews.is_empty());

    let Json(archived) = session_archived_get(Query(query_for(project_path)), HeaderMap::new())
        .await
        .expect("empty archived ids should be handled");
    assert!(archived.is_empty());

    let scope = ui_store::resolve_session_scope_id(Some(&project_path.to_string_lossy()), None)
        .expect("project directory should resolve to a scope");
    ui_store::set_session_scope(Some(&scope));

    let Json(meta) = session_ui_preview_meta(Path(session_id.clone()))
        .await
        .expect("unknown preview metadata should be handled");
    assert!(meta.is_none());

    let Json(any_session) = session_ui_get_any(Path(session_id))
        .await
        .expect("unknown cross-project session should be handled");
    assert!(any_session.is_none());

    ui_store::set_session_scope(None);
}

#[tokio::test(flavor = "multi_thread")]
async fn session_archived_put_empty_list_clears_scope() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();

    let _ = session_archived_put(
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(vec!["archived-a".to_string(), "archived-b".to_string()]),
    )
    .await
    .expect("archived ids should save");

    let Json(cleared) =
        session_archived_put(Query(query_for(project_path)), HeaderMap::new(), Json(Vec::new()))
            .await
            .expect("empty archived ids should clear the scope");
    assert!(cleared);

    let Json(archived) = session_archived_get(Query(query_for(project_path)), HeaderMap::new())
        .await
        .expect("cleared archived ids should load");
    assert!(archived.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn session_ui_get_maps_invalid_json_llm_raw_to_internal_error() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);

    let project = TempDir::new().expect("project dir");
    let project_path = project.path();
    let scope = ui_store::resolve_session_scope_id(Some(&project_path.to_string_lossy()), None)
        .expect("project directory should resolve to a scope");
    let session_id = format!("invalid-raw-session-{}", std::process::id());
    let _ = ui_store::persist_llm_raw_step(
        &session_id,
        7,
        &serde_json::json!({"valid": true}),
        Some(&scope),
    )
    .expect("raw payload should persist before injection");
    let mut session = session("invalid-json-session", "Invalid JSON");
    session.steps[0].index = 7;
    let Json(saved) = session_ui_save(
        Path(session_id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(session),
    )
    .await
    .expect("session containing invalid raw step should save");
    assert!(saved);
    let tenant = ui_store::current_tenant_scope();
    let connection =
        Connection::open(ui_store::session_file_path(&session_id).expect("session store path"))
            .expect("open session store database");
    connection
        .execute(
            "UPDATE session_step_llm_raw
             SET payload_json = ?1
             WHERE platform_id = ?2 AND company_id = ?3 AND project_id = ?4
               AND session_id = ?5 AND step_index = ?6",
            params![
                "{definitely not json",
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
                &scope,
                &session_id,
                7i64,
            ],
        )
        .expect("raw payload should be corrupted for the error case");

    let error = session_ui_get(Path(session_id), Query(query_for(project_path)), HeaderMap::new())
        .await
        .expect_err("invalid raw payload should fail");
    assert_eq!(error.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test(flavor = "multi_thread")]
async fn ui_snapshot_handlers_isolate_same_session_id_across_tenants() {
    let _guard = UI_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    ui_store::set_session_scope(None);
    let project = ui_ops_test_home().join("tenant-shared-project");
    std::fs::create_dir_all(&project).expect("create shared project");
    let tenant_a = vw_api_types::id::TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = vw_api_types::id::TenantScope::new("platform-b", "company-b").expect("tenant B");
    let session_a = scoped_session("tenant-shared-session", "tenant A", "alpha", 10);
    let session_b = scoped_session("tenant-shared-session", "tenant B", "beta", 20);

    let save_a = ui_store::with_tenant_scope(
        tenant_a.clone(),
        session_ui_save(
            Path(session_a.id.clone()),
            Query(query_for(&project)),
            HeaderMap::new(),
            Json(session_a),
        ),
    )
    .await
    .expect("save tenant A");
    let save_b = ui_store::with_tenant_scope(
        tenant_b.clone(),
        session_ui_save(
            Path(session_b.id.clone()),
            Query(query_for(&project)),
            HeaderMap::new(),
            Json(session_b),
        ),
    )
    .await
    .expect("save tenant B");
    assert!(save_a.0 && save_b.0);

    let loaded_a = ui_store::with_tenant_scope(
        tenant_a,
        session_ui_get(
            Path("tenant-shared-session".to_owned()),
            Query(query_for(&project)),
            HeaderMap::new(),
        ),
    )
    .await
    .expect("load tenant A")
    .0
    .expect("tenant A session");
    let loaded_b = ui_store::with_tenant_scope(
        tenant_b,
        session_ui_get(
            Path("tenant-shared-session".to_owned()),
            Query(query_for(&project)),
            HeaderMap::new(),
        ),
    )
    .await
    .expect("load tenant B")
    .0
    .expect("tenant B session");
    assert_eq!(loaded_a.title, "tenant A");
    assert_eq!(loaded_b.title, "tenant B");
    assert_eq!(loaded_a.messages[0].content, "alpha");
    assert_eq!(loaded_b.messages[0].content, "beta");
}
