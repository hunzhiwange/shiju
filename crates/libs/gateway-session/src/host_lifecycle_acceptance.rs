//! Full lifecycle assertions exercised with the actual host ports by thin host tests.
use crate::session_host::SessionHost;
use crate::session_ops::*;
use crate::shared::{UiSessionCreateBody, UiSessionListQuery};
use agent_project as project;
use gateway_instance::host::InstanceQuery;
use axum::{Json, extract::{Path, Query, State}, http::{HeaderMap, HeaderValue}};
use std::{path::Path as FsPath, sync::Once};
use tempfile::TempDir;
use vw_api_session::session::*;
use vw_api_todo::todo::{TodoPriority, TodoStatus};

static TEST_HOME: Once = Once::new();
pub static SESSION_OPS_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn ensure_test_home() {
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
        "x-directory",
        HeaderValue::from_str(&directory.to_string_lossy()).expect("valid directory header"),
    );
    headers
}

fn query_for(directory: &FsPath) -> InstanceQuery {
    InstanceQuery { directory: Some(directory.to_string_lossy().to_string()) }
}

async fn create_session_in<H: SessionHost>(directory: &FsPath, title: &str) -> session_info::Info {
    let body = UiSessionCreateBody {
        session: GatewaySessionCreateBody { parent_id: None, title: Some(title.to_string()) },
        permission: None,
    };
    ui_session_create::<H>(Query(query_for(directory)), HeaderMap::new(), Json(Some(body)))
        .await
        .expect("session should be created")
        .0
}

// 错误用例固定失败发生在持久化之前，避免跨会话或跨作用域修改已有数据。
pub async fn gateway_first_response_create_without_title_reserves_an_unpublished_session<H: SessionHost>(_state: H::State) {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session_store::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();

    let Json(created) =
        ui_session_create::<H>(Query(query_for(project_path)), HeaderMap::new(), Json(None))
            .await
            .expect("session should be created from absent body");

    assert_eq!(created.parent_id, None);
    assert_eq!(created.directory, project_path.canonicalize().unwrap().to_string_lossy());
    assert!(created.title.is_empty());
    assert!(created.is_first_response_pending());
    let Json(loaded) =
        ui_session_get::<H>(Path(created.id.clone()), Query(query_for(project_path)), HeaderMap::new())
            .await
            .unwrap();
    assert_eq!(loaded.id, created.id);
    let Json(sessions) = ui_session_list::<H>(
        Query(UiSessionListQuery {
            directory: Some(created.directory.clone()),
            project_id: None,
            roots: None,
            start: None,
            search: None,
            limit: None,
        }),
        HeaderMap::new(),
    )
    .await
    .unwrap();
    let listed = sessions
        .iter()
        .find(|session| session.id == created.id)
        .expect("pending root must remain in the session list");
    assert!(listed.is_first_response_pending());
    let error = session_title_generate::<H>(
        Path(created.id),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionTitleGenerateBody {
            content: "must not be sent to a model".into(),
            preferred_model: Some("missing-provider/missing-model".into()),
            acp_agent: None,
        }),
    )
    .await
    .err()
    .unwrap();
    assert_eq!(error.status, axum::http::StatusCode::CONFLICT);
}

pub async fn ui_session_list_respects_updated_boundary_and_zero_limit<H: SessionHost>(_state: H::State) {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session_store::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();
    let created = create_session_in::<H>(project_path, "Boundary Session").await;

    let Json(at_boundary) = ui_session_list::<H>(
        Query(UiSessionListQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            project_id: None,
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

    let Json(after_boundary) = ui_session_list::<H>(
        Query(UiSessionListQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            project_id: None,
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

    let Json(zero_limited) = ui_session_list::<H>(
        Query(UiSessionListQuery {
            directory: Some(project_path.to_string_lossy().to_string()),
            project_id: None,
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

pub async fn ui_session_status_preserves_existing_non_idle_state<H: SessionHost>(_state: H::State) {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session_store::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let created = create_session_in::<H>(project_dir.path(), "Busy Session").await;
    session_engine::status::set(&created.id, session_engine::status::Info::Busy);

    let Json(statuses) = ui_session_status::<H>(Query(query_for(project_dir.path())), HeaderMap::new())
        .await
        .expect("status lookup should succeed");
    assert!(matches!(statuses.get(&created.id), Some(session_engine::status::Info::Busy)));

    session_engine::status::set(&created.id, session_engine::status::Info::Idle);
}

pub async fn ui_session_patch_ignores_empty_fields_and_uses_header_scope<H: SessionHost>(_state: H::State) {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session_store::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();
    let created = create_session_in::<H>(project_path, "Stable Title").await;

    let Json(archived) = ui_session_patch::<H>(
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

    let Json(unchanged) = ui_session_patch::<H>(
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

pub async fn session_todo_put_replaces_and_clears_saved_items<H: SessionHost>(_state: H::State) {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session_store::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();
    let created = create_session_in::<H>(project_path, "Todo Replacement").await;
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

    let Json(saved_first) = session_todo_put::<H>(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: vec![first] }),
    )
    .await
    .expect("initial todos should save");
    assert!(saved_first);

    let Json(saved_replacement) = session_todo_put::<H>(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: vec![replacement.clone()] }),
    )
    .await
    .expect("replacement todos should save");
    assert!(saved_replacement);

    let Json(replaced) = session_todo_get::<H>(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
    )
    .await
    .expect("replacement todos should load");
    let mut expected_replacement = replacement;
    expected_replacement.id = "1".to_string();
    assert_eq!(replaced, vec![expected_replacement]);

    let Json(saved_empty) = session_todo_put::<H>(
        Path(created.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: Vec::new() }),
    )
    .await
    .expect("empty todos should clear storage");
    assert!(saved_empty);

    let Json(cleared) =
        session_todo_get::<H>(Path(created.id), Query(query_for(project_path)), HeaderMap::new())
            .await
            .expect("cleared todos should load");
    assert!(cleared.is_empty());
}

pub async fn scoped_mutations_do_not_modify_sessions_from_another_instance<H: SessionHost>(_state: H::State) {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session_store::ui_store::set_session_scope(None);

    let project_a = TempDir::new().expect("project a dir");
    let project_b = TempDir::new().expect("project b dir");
    let foreign = create_session_in::<H>(project_b.path(), "Foreign Session").await;

    let patch_result = ui_session_patch::<H>(
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

    let delete_result = ui_session_delete::<H>(
        State(_state.clone()),
        Path(foreign.id.clone()),
        Query(query_for(project_a.path())),
        HeaderMap::new(),
    )
    .await;
    assert!(delete_result.is_err());

    let fork_result = session_fork::<H>(
        State(_state.clone()),
        Path(foreign.id.clone()),
        Query(query_for(project_a.path())),
        HeaderMap::new(),
        Json(None),
    )
    .await;
    assert!(fork_result.is_err());

    let Json(still_present) = ui_session_get::<H>(
        Path(foreign.id),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
    )
    .await
    .expect("rejected scoped mutations must leave the foreign session intact");
    assert_eq!(still_present.title, "Foreign Session");
}

pub async fn session_ops_cover_lifecycle_filters_and_instance_boundaries<H: SessionHost>(_state: H::State) {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session_store::ui_store::set_session_scope(None);

    let project_a = TempDir::new().expect("project a dir");
    let project_b = TempDir::new().expect("project b dir");
    let project_a_path = project_a.path();
    let project_b_path = project_b.path();

    let root = create_session_in::<H>(project_a_path, "Alpha Root").await;
    let child_body = UiSessionCreateBody {
        session: GatewaySessionCreateBody {
            parent_id: Some(root.id.clone()),
            title: Some("Alpha Child".to_string()),
        },
        permission: None,
    };
    let child = ui_session_create::<H>(
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(Some(child_body)),
    )
    .await
    .expect("child should be created")
    .0;
    let other = create_session_in::<H>(project_b_path, "Beta Root").await;

    let Json(listed) = ui_session_list::<H>(
        Query(UiSessionListQuery {
            directory: Some(project_a_path.to_string_lossy().to_string()),
            project_id: None,
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

    let Json(limited) = ui_session_list::<H>(
        Query(UiSessionListQuery {
            directory: Some("  ".to_string()),
            project_id: None,
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

    let Json(statuses) = ui_session_status::<H>(Query(query_for(project_a_path)), HeaderMap::new())
        .await
        .expect("status should succeed");
    assert!(matches!(statuses.get(&root.id), Some(session_engine::status::Info::Idle)));
    assert!(statuses.contains_key(&child.id));
    assert!(!statuses.contains_key(&other.id));

    let Json(all_statuses) = ui_session_status::<H>(
        Query(InstanceQuery { directory: Some("  ".to_string()) }),
        HeaderMap::new(),
    )
    .await
    .expect("blank directory status should succeed");
    assert!(all_statuses.contains_key(&other.id));

    let Json(global_get) = ui_session_get::<H>(
        Path(root.id.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
    )
    .await
    .expect("global get should succeed");
    assert_eq!(global_get.id, root.id);

    let Json(scoped_get) = ui_session_get::<H>(
        Path(root.id.clone()),
        Query(InstanceQuery { directory: None }),
        headers_for(project_a_path),
    )
    .await
    .expect("header scoped get should succeed");
    assert_eq!(scoped_get.directory, project_a_path.to_string_lossy());

    let missing_scoped =
        ui_session_get::<H>(Path(other.id.clone()), Query(query_for(project_a_path)), HeaderMap::new())
            .await;
    assert!(missing_scoped.is_err());

    let Json(patched) = ui_session_patch::<H>(
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

    let Json(unarchived) = ui_session_patch::<H>(
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
        session_children::<H>(Path(root.id.clone()), Query(query_for(project_a_path)), HeaderMap::new())
            .await
            .expect("children should be listed");
    assert_eq!(children.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec![child.id.as_str()]);

    let todo = GatewaySessionTodoItem {
        id: "todo-1".to_string(),
        content: "ship tests".to_string(),
        status: TodoStatus::InProgress,
        priority: TodoPriority::High,
    };
    let Json(saved_todos) = session_todo_put::<H>(
        Path(root.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(GatewaySessionTodoPutBody { todos: vec![todo.clone()] }),
    )
    .await
    .expect("todos should be saved");
    assert!(saved_todos);

    let Json(todos) =
        session_todo_get::<H>(Path(root.id.clone()), Query(query_for(project_a_path)), HeaderMap::new())
            .await
            .expect("todos should be loaded");
    let mut expected_todo = todo;
    expected_todo.id = "1".to_string();
    assert_eq!(todos, vec![expected_todo]);
    let Json(other_project_todos) =
        session_todo_get::<H>(Path(root.id.clone()), Query(query_for(project_b_path)), HeaderMap::new())
            .await
            .expect("other project todo lookup should succeed");
    assert!(other_project_todos.is_empty());

    let Json(forked) = session_fork::<H>(
        State(_state.clone()),
        Path(root.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(None),
    )
    .await
    .expect("fork should succeed");
    assert_eq!(forked.title, "Alpha Renamed (fork #1)");
    assert_eq!(forked.directory, project_a_path.to_string_lossy());

    let fork_error = session_fork::<H>(
        State(_state.clone()),
        Path(root.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
        Json(Some(GatewaySessionForkBody {
            message_id: Some("missing-message".to_string()),
            ..Default::default()
        })),
    )
    .await
    .expect_err("unknown legacy message boundary should be rejected");
    assert_eq!(fork_error.status, axum::http::StatusCode::BAD_REQUEST);

    let Json(deleted) = ui_session_delete::<H>(
        State(_state.clone()),
        Path(forked.id.clone()),
        Query(query_for(project_a_path)),
        HeaderMap::new(),
    )
    .await
    .expect("scoped delete should succeed");
    assert!(deleted);
    assert!(
        ui_session_get::<H>(Path(forked.id), Query(query_for(project_a_path)), HeaderMap::new())
            .await
            .is_err()
    );

    let Json(deleted_other) = ui_session_delete::<H>(
        State(_state.clone()),
        Path(other.id.clone()),
        Query(InstanceQuery { directory: None }),
        HeaderMap::new(),
    )
    .await
    .expect("global delete should succeed");
    assert!(deleted_other);
}

pub async fn session_ops_cover_diff_summary_reset_and_title_errors<H: SessionHost>(_state: H::State) {
    let _guard = SESSION_OPS_TEST_LOCK.lock().await;
    ensure_test_home();
    project::instance::dispose_all().await;
    session_store::ui_store::set_session_scope(None);

    let project_dir = TempDir::new().expect("project dir");
    let project_path = project_dir.path();
    let info = create_session_in::<H>(project_path, "Error Paths").await;

    let Json(diffs) = session_diff::<H>(
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

    let summarize_error = session_summarize::<H>(
        Path(info.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionSummarizeBody { message_id: "missing-message".to_string() }),
    )
    .await
    .expect_err("summarize requires an existing user-message boundary");
    assert!(summarize_error.to_string().contains("compaction boundary"));

    let Json(reset) = session_reset::<H>(
        State(_state.clone()),
        Path(info.id.clone()),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionResetBody { message_id: "missing-message".to_string() }),
    )
    .await
    .expect("reset should safely handle a missing message");
    assert_eq!(reset.id, info.id);

    let title_result = session_title_generate::<H>(
        Path(info.id),
        Query(query_for(project_path)),
        HeaderMap::new(),
        Json(GatewaySessionTitleGenerateBody {
            content: "".to_string(),
            preferred_model: None,
            acp_agent: None,
        }),
    )
    .await;
    assert!(title_result.is_err());
}
