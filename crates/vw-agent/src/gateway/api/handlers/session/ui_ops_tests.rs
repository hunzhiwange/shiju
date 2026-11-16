// 覆盖 UI 会话快照、原始模型步骤及归档作用域持久化。

use super::*;
use crate::app::agent::gateway::instance::InstanceQuery;
use crate::session::ui_store;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, HeaderValue};
use rusqlite::params;
use std::path::Path as FsPath;
use std::sync::Once;
use tempfile::TempDir;
use vw_api_session::session::{
    GatewaySessionTodoItem,
    GatewaySessionTodoPutBody,
};
use vw_api_todo::todo::{TodoPriority, TodoStatus};
use session_info::{Info as AgentSessionInfo, TimeInfo as AgentSessionTimeInfo};

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
        "x-directory",
        HeaderValue::from_str(&directory.to_string_lossy()).expect("valid directory header"),
    );
    headers
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
            turn_id: None,
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

fn save_runtime_session(project_path: &FsPath, session: &ui_models::ChatSession) -> String {
    let tenant = ui_store::current_tenant_scope();
    let scope = ui_store::resolve_session_scope_id_for_tenant(
        &tenant,
        Some(&project_path.to_string_lossy()),
        None,
    )
    .expect("project directory should resolve to a scope");
    ui_store::save_session_scoped_for_tenant(&tenant, session, Some(&scope))
        .expect("runtime session should save");
    scope
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
        fork_execution_started: None,
        revert: None,
    }
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
    save_runtime_session(project_a_path, &body_a);
    save_runtime_session(project_b_path, &body_b);

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

    let Json(meta) = session_ui_preview_meta(
        Path(body_b.id.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
    )
    .await
    .expect("preview meta should load");
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
                vw_api_transport::DEFAULT_PLATFORM_ID,
                vw_api_transport::DEFAULT_COMPANY_ID,
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
