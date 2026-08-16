use super::*;
use crate::app::agent::config::Config;
use crate::app::agent::gateway::{GatewayRateLimiter, IdempotencyStore};
use crate::app::agent::memory::NoneMemory;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::pairing::PairingGuard;
use crate::app::agent::{project, session};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use std::path::Path as FsPath;
use std::sync::Arc;
use std::sync::Once;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::broadcast;
use vw_api_types::session::{
    GatewaySessionCreateBody, GatewaySessionPatchBody, GatewaySessionPatchTime,
    GatewaySessionTodoItem, GatewaySessionTodoPutBody,
};
use vw_api_types::todo::{TodoPriority, TodoStatus};

struct StaticProvider;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Provider for StaticProvider {
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        Ok(message.to_string())
    }
}

static TEST_HOME: Once = Once::new();
static SESSION_OPS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn ensure_test_home() {
    TEST_HOME.call_once(|| {
        let dir = std::env::temp_dir().join(format!("vw-session-ops-tests-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("test home should be created");
        unsafe {
            std::env::set_var("VIBEWINDOW_TEST_HOME", dir);
        }
    });
}

fn headers_for(directory: &FsPath) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-vibewindow-directory",
        HeaderValue::from_str(&directory.to_string_lossy()).expect("valid directory header"),
    );
    headers
}

fn query_for(directory: &FsPath) -> InstanceQuery {
    InstanceQuery { directory: Some(directory.to_string_lossy().to_string()) }
}

fn state() -> AppState {
    let (event_tx, _) = broadcast::channel(16);
    AppState {
        config: Arc::new(parking_lot::Mutex::new(Config::default())),
        provider: Arc::new(StaticProvider),
        model: "test-model".to_string(),
        temperature: 0.0,
        mem: Arc::new(NoneMemory::new()),
        auto_save: false,
        webhook_secret_hash: None,
        pairing: Arc::new(PairingGuard::new(false, &[])),
        trust_forwarded_headers: false,
        rate_limiter: Arc::new(GatewayRateLimiter::new(100, 100, 100)),
        idempotency_store: Arc::new(IdempotencyStore::new(Duration::from_secs(300), 1000)),
        whatsapp: None,
        whatsapp_app_secret: None,
        linq: None,
        linq_signing_secret: None,
        nextcloud_talk: None,
        nextcloud_talk_webhook_secret: None,
        wati: None,
        qq: None,
        qq_webhook_enabled: false,
        observer: Arc::new(crate::app::agent::observability::NoopObserver),
        tools_registry: Arc::new(Vec::new()),
        tools_registry_exec: Arc::new(Vec::new()),
        multimodal: crate::app::agent::config::MultimodalConfig::default(),
        max_tool_iterations: 10,
        event_tx,
        session_query_engines: Default::default(),
        hook_runtime: None,
    }
}

async fn create_session_in(directory: &FsPath, title: &str) -> agent_session::session::Info {
    let body = UiSessionCreateBody {
        session: GatewaySessionCreateBody { parent_id: None, title: Some(title.to_string()) },
        permission: None,
    };
    ui_session_create(Query(query_for(directory)), HeaderMap::new(), Json(Some(body)))
        .await
        .expect("session should be created")
        .0
}

fn summary(files: i64, additions: i64, deletions: i64) -> agent_session::session::Summary {
    agent_session::session::Summary { additions, deletions, files, diffs: None }
}

fn info_with_summary(
    id: &str,
    summary: Option<agent_session::session::Summary>,
) -> agent_session::session::Info {
    agent_session::session::Info {
        id: id.to_string(),
        slug: "fixture-slug".to_string(),
        project_id: "fixture-project".to_string(),
        directory: "/tmp/fixture".to_string(),
        parent_id: None,
        summary,
        share: None,
        title: "Fixture".to_string(),
        version: "test".to_string(),
        time: agent_session::session::TimeInfo {
            created: 1,
            updated: 1,
            compacting: None,
            archived: None,
        },
        permission: None,
        revert: None,
    }
}

#[test]
fn session_handlers_are_available() {
    let _ = ui_session_list;
    let _ = ui_session_status;
    let _ = ui_session_get;
    let _ = ui_session_create;
    let _ = ui_session_patch;
    let _ = ui_session_delete;
    let _ = session_children;
    let _ = session_todo_get;
    let _ = session_todo_put;
    let _ = session_fork;
    let _ = session_reset;
    let _ = session_diff;
    let _ = session_summarize;
    let _ = session_title_generate;
}

#[test]
fn task_session_summary_needs_refresh_only_for_empty_task_board_summaries() {
    let non_task = info_with_summary("ses-fixture", None);
    assert!(!task_session_summary_needs_refresh(&non_task));

    let missing = info_with_summary("task-board-missing", None);
    assert!(task_session_summary_needs_refresh(&missing));

    let zero = info_with_summary("task-board-zero", Some(summary(0, 0, 0)));
    assert!(task_session_summary_needs_refresh(&zero));

    let with_files = info_with_summary("task-board-files", Some(summary(1, 0, 0)));
    assert!(!task_session_summary_needs_refresh(&with_files));

    let with_additions = info_with_summary("task-board-additions", Some(summary(0, 1, 0)));
    assert!(!task_session_summary_needs_refresh(&with_additions));

    let with_deletions = info_with_summary("task-board-deletions", Some(summary(0, 0, 1)));
    assert!(!task_session_summary_needs_refresh(&with_deletions));
}

#[tokio::test]
async fn refresh_task_session_summaries_preserves_summary_when_refresh_fails() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let original = summary(0, 0, 0);
    let mut sessions =
        vec![info_with_summary("task-board-missing-refresh-fixture", Some(original.clone()))];

    refresh_task_session_summaries(&mut sessions).await;

    let refreshed = sessions[0].summary.as_ref().expect("summary should be preserved");
    assert_eq!(refreshed.files, original.files);
    assert_eq!(refreshed.additions, original.additions);
    assert_eq!(refreshed.deletions, original.deletions);
    project::instance::dispose_all().await;
}

#[tokio::test]
async fn ui_session_create_accepts_absent_body_and_generates_default_title() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();

    let Json(created) =
        ui_session_create(Query(query_for(project_path)), HeaderMap::new(), Json(None))
            .await
            .expect("session should be created from absent body");

    assert_eq!(created.parent_id, None);
    assert_eq!(created.directory, project_path.to_string_lossy());
    assert!(created.title.starts_with("New session - "));
}

#[tokio::test]
async fn ui_session_list_respects_updated_boundary_and_zero_limit() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();
    let created = create_session_in(project_path, "Boundary Session").await;

    let Json(at_boundary) = ui_session_list(
        Query(UiSessionListQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            roots: None,
            start: Some(created.time.updated),
            search: Some("BOUNDARY".to_string()),
            limit: None,
        }),
        HeaderMap::new(),
    )
    .await
    .expect("inclusive updated boundary should retain the session");
    assert_eq!(
        at_boundary.iter().map(|info| info.id.as_str()).collect::<Vec<_>>(),
        vec![created.id.as_str()]
    );

    let Json(after_boundary) = ui_session_list(
        Query(UiSessionListQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            roots: None,
            start: Some(created.time.updated.saturating_add(1)),
            search: None,
            limit: None,
        }),
        HeaderMap::new(),
    )
    .await
    .expect("updated filter should succeed");
    assert!(after_boundary.is_empty());

    let Json(zero_limited) = ui_session_list(
        Query(UiSessionListQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            roots: None,
            start: None,
            search: None,
            limit: Some(0),
        }),
        HeaderMap::new(),
    )
    .await
    .expect("zero limit should succeed");
    assert!(zero_limited.is_empty());
}

#[tokio::test]
async fn ui_session_status_preserves_existing_non_idle_state() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let created = create_session_in(project_dir.path(), "Busy Session").await;
    agent_session::status::set(&created.id, agent_session::status::Info::Busy);

    let Json(statuses) = ui_session_status(Query(query_for(project_dir.path())), HeaderMap::new())
        .await
        .expect("status lookup should succeed");
    assert!(matches!(statuses.get(&created.id), Some(agent_session::status::Info::Busy)));

    agent_session::status::set(&created.id, agent_session::status::Info::Idle);
}

#[tokio::test]
async fn ui_session_patch_ignores_empty_fields_and_uses_header_scope() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();
    let created = create_session_in(project_path, "Stable Title").await;

    let Json(archived) = ui_session_patch(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionPatchBody {
            title: None,
            time: Some(GatewaySessionPatchTime { archived: Some(7) }),
        }),
    )
    .await
    .expect("archive patch should succeed");
    assert_eq!(archived.time.archived, Some(7));

    let Json(unchanged) = ui_session_patch(
        Path(created.id),
        Query(InstanceQuery { directory: None }),
        headers_for(project_path),
        Json(GatewaySessionPatchBody {
            title: Some(" \n\t ".to_string()),
            time: Some(GatewaySessionPatchTime { archived: None }),
        }),
    )
    .await
    .expect("header scoped no-op patch should succeed");
    assert_eq!(unchanged.title, "Stable Title");
    assert_eq!(unchanged.time.archived, Some(7));
}

#[tokio::test]
async fn session_todo_put_replaces_and_clears_saved_items() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();
    let created = create_session_in(project_path, "Todo Replacement").await;
    let first = GatewaySessionTodoItem {
        id: "todo-first".to_string(),
        content: "first item".to_string(),
        status: TodoStatus::Pending,
        priority: TodoPriority::Low,
    };
    let replacement = GatewaySessionTodoItem {
        id: "todo-replacement".to_string(),
        content: "replacement item".to_string(),
        status: TodoStatus::Completed,
        priority: TodoPriority::High,
    };

    let Json(saved_first) = session_todo_put(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: vec![first] }),
    )
    .await
    .expect("initial todos should save");
    assert!(saved_first);

    let Json(saved_replacement) = session_todo_put(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: vec![replacement.clone()] }),
    )
    .await
    .expect("replacement todos should save");
    assert!(saved_replacement);

    let Json(replaced) = session_todo_get(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("replacement todos should load");
    let mut expected_replacement = replacement;
    expected_replacement.id = "1".to_string();
    assert_eq!(replaced, vec![expected_replacement]);

    let Json(saved_empty) = session_todo_put(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: Vec::new() }),
    )
    .await
    .expect("empty todos should clear storage");
    assert!(saved_empty);

    let Json(cleared) =
        session_todo_get(Path(created.id), Query(query_for(project_path)), HeaderMap::new())
            .await
            .expect("cleared todos should load");
    assert!(cleared.is_empty());
}

#[tokio::test]
async fn scoped_mutations_do_not_modify_sessions_from_another_instance() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let project_a = TempDir::new().expect("project a dir");
    let project_b = TempDir::new().expect("project b dir");
    let foreign = create_session_in(project_b.path(), "Foreign Session").await;

    let patch_result = ui_session_patch(
        Path(foreign.id.clone()),
        Query(query_for(project_a.path())),
        HeaderMap::new(),
        Json(GatewaySessionPatchBody {
            title: Some("Cross-instance mutation".to_string()),
            time: None,
        }),
    )
    .await;
    assert!(patch_result.is_err());

    let delete_result = ui_session_delete(
        State(state()),
        Path(foreign.id.clone()),
        Query(query_for(project_a.path())),
        HeaderMap::new(),
    )
    .await;
    assert!(delete_result.is_err());

    let fork_result = session_fork(
        State(state()),
        Path(foreign.id.clone()),
        Query(query_for(project_a.path())),
        HeaderMap::new(),
        Json(None),
    )
    .await;
    assert!(fork_result.is_err());

    let Json(still_present) = ui_session_get(
        Path(foreign.id),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
    )
    .await
    .expect("rejected scoped mutations must leave the foreign session intact");
    assert_eq!(still_present.title, "Foreign Session");
}

#[tokio::test]
async fn session_ops_cover_lifecycle_filters_and_instance_boundaries() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let project_a = TempDir::new().expect("project a dir");
    let project_b = TempDir::new().expect("project b dir");
    let project_a_path = project_a.path();
    let project_b_path = project_b.path();

    let root = create_session_in(project_a_path, "Alpha Root").await;
    let child_body = UiSessionCreateBody {
        session: GatewaySessionCreateBody {
            parent_id: Some(root.id.clone()),
            title: Some("Alpha Child".to_string()),
        },
        permission: None,
    };
    let child = ui_session_create(
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(Some(child_body)),
    )
    .await
    .expect("child should be created")
    .0;
    let other = create_session_in(project_b_path, "Beta Root").await;

    let Json(listed) = ui_session_list(
        Query(UiSessionListQuery {
            directory: Some(project_a_path.to_string_lossy().to_string()),
            roots: Some(true),
            start: Some(0),
            search: Some("alpha".to_string()),
            limit: Some(10),
        }),
        HeaderMap::new(),
    )
    .await
    .expect("list should succeed");
    assert_eq!(listed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec![root.id.as_str()]);

    let Json(limited) = ui_session_list(
        Query(UiSessionListQuery {
            directory: Some("  ".to_string()),
            roots: None,
            start: None,
            search: None,
            limit: Some(1),
        }),
        HeaderMap::new(),
    )
    .await
    .expect("blank directory list should succeed");
    assert_eq!(limited.len(), 1);

    let Json(statuses) = ui_session_status(Query(query_for(project_a_path)), HeaderMap::new())
        .await
        .expect("status should succeed");
    assert!(matches!(statuses.get(&root.id), Some(agent_session::status::Info::Idle)));
    assert!(statuses.contains_key(&child.id));
    assert!(!statuses.contains_key(&other.id));

    let Json(all_statuses) = ui_session_status(
        Query(InstanceQuery { directory: Some("  ".to_string()) }),
        HeaderMap::new(),
    )
    .await
    .expect("blank directory status should succeed");
    assert!(all_statuses.contains_key(&other.id));

    let Json(global_get) = ui_session_get(
        Path(root.id.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
    )
    .await
    .expect("global get should succeed");
    assert_eq!(global_get.id, root.id);

    let Json(scoped_get) = ui_session_get(
        Path(root.id.clone()),
        Query(InstanceQuery { directory: None }),
        headers_for(project_a_path),
    )
    .await
    .expect("header scoped get should succeed");
    assert_eq!(scoped_get.directory, project_a_path.to_string_lossy());

    let missing_scoped =
        ui_session_get(Path(other.id.clone()), Query(query_for(project_a_path)), HeaderMap::new())
            .await;
    assert!(missing_scoped.is_err());

    let Json(patched) = ui_session_patch(
        Path(root.id.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
        Json(GatewaySessionPatchBody {
            title: Some("  Alpha Renamed  ".to_string()),
            time: Some(GatewaySessionPatchTime { archived: Some(42) }),
        }),
    )
    .await
    .expect("global patch should succeed");
    assert_eq!(patched.title, "Alpha Renamed");
    assert_eq!(patched.time.archived, Some(42));

    let Json(unarchived) = ui_session_patch(
        Path(root.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(GatewaySessionPatchBody {
            title: Some(" ".to_string()),
            time: Some(GatewaySessionPatchTime { archived: Some(0) }),
        }),
    )
    .await
    .expect("scoped patch should succeed");
    assert_eq!(unarchived.title, "Alpha Renamed");
    assert_eq!(unarchived.time.archived, None);

    let Json(children) =
        session_children(Path(root.id.clone()), Query(query_for(project_a_path)), HeaderMap::new())
            .await
            .expect("children should be listed");
    assert_eq!(children.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec![child.id.as_str()]);

    let todo = GatewaySessionTodoItem {
        id: "todo-1".to_string(),
        content: "ship tests".to_string(),
        status: TodoStatus::InProgress,
        priority: TodoPriority::High,
    };
    let Json(saved_todos) = session_todo_put(
        Path(root.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: vec![todo.clone()] }),
    )
    .await
    .expect("todos should be saved");
    assert!(saved_todos);

    let Json(todos) =
        session_todo_get(Path(root.id.clone()), Query(query_for(project_a_path)), HeaderMap::new())
            .await
            .expect("todos should be loaded");
    let mut expected_todo = todo;
    expected_todo.id = "1".to_string();
    assert_eq!(todos, vec![expected_todo]);
    let Json(other_project_todos) =
        session_todo_get(Path(root.id.clone()), Query(query_for(project_b_path)), HeaderMap::new())
            .await
            .expect("other project todo lookup should succeed");
    assert!(other_project_todos.is_empty());

    let Json(forked) = session_fork(
        State(state()),
        Path(root.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(None),
    )
    .await
    .expect("fork should succeed");
    assert_eq!(forked.title, "Alpha Renamed (fork #1)");
    assert_eq!(forked.directory, project_a_path.to_string_lossy());

    let Json(forked_with_message) = session_fork(
        State(state()),
        Path(root.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(Some(GatewaySessionForkBody { message_id: Some("missing-message".to_string()) })),
    )
    .await
    .expect("fork with message id should succeed for empty history");
    assert_eq!(forked_with_message.title, "Alpha Renamed (fork #1)");
    assert_eq!(forked_with_message.directory, project_a_path.to_string_lossy());

    let Json(deleted) = ui_session_delete(
        State(state()),
        Path(forked.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
    )
    .await
    .expect("scoped delete should succeed");
    assert!(deleted);
    assert!(
        ui_session_get(Path(forked.id), Query(query_for(project_a_path)), HeaderMap::new())
            .await
            .is_err()
    );

    let Json(deleted_fork_with_message) = ui_session_delete(
        State(state()),
        Path(forked_with_message.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
    )
    .await
    .expect("scoped delete should remove fork with message");
    assert!(deleted_fork_with_message);

    let Json(deleted_other) = ui_session_delete(
        State(state()),
        Path(other.id.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
    )
    .await
    .expect("global delete should succeed");
    assert!(deleted_other);
}

#[tokio::test]
async fn session_ops_cover_diff_summary_reset_and_title_errors() {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();
    let info = create_session_in(project_path, "Error Paths").await;

    let Json(diffs) = session_diff(
        Path(info.id.clone()),
        Query(GatewaySessionDiffQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            message_id: None,
        }),
        HeaderMap::new(),
    )
    .await
    .expect("diff should return an empty list outside git changes");
    assert!(diffs.is_empty());

    let summarize_error = session_summarize(
        Path(info.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionSummarizeBody { message_id: "missing-message".to_string() }),
    )
    .await
    .expect_err("summarize requires an existing user-message boundary");
    assert!(summarize_error.to_string().contains("compaction boundary"));

    let Json(reset) = session_reset(
        State(state()),
        Path(info.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionResetBody {
            message_id: "missing-message".to_string(),
            revert_code: false,
        }),
    )
    .await
    .expect("reset without revert should safely handle a missing message");
    assert_eq!(reset.id, info.id);

    let Json(reset_with_revert) = session_reset(
        State(state()),
        Path(info.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionResetBody {
            message_id: "missing-message".to_string(),
            revert_code: true,
        }),
    )
    .await
    .expect("reset with revert should safely handle an empty history");
    assert_eq!(reset_with_revert.id, info.id);

    let title_result = session_title_generate(
        Path(info.id),
        Json(GatewaySessionTitleGenerateBody {
            content: "".to_string(),
            preferred_model: None,
            acp_agent: None,
        }),
    )
    .await;
    assert!(title_result.is_err());
}
