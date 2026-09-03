use super::{GatewayChatStreamCollected, SessionTool};
use crate::app::agent::tools::traits::Tool;
use crate::session::task_session::{TaskSessionCoordinator, TaskSessionState};
use futures_util::FutureExt;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::oneshot;
use vw_api_types::id::TenantScope;

#[test]
fn session_tool_read_bounds_output() {
    assert_eq!(super::MAX_SESSION_READ_CHARS, 64 * 1024);
}

async fn create_parent(directory: &std::path::Path) -> crate::session::runtime::SessionSnapshot {
    let scope = crate::session::runtime::SessionScope::new(TenantScope::default(), directory)
        .expect("test scope");
    crate::session::runtime::create_session(crate::session::runtime::CreateSessionInput {
        scope,
        parent_id: None,
        title: Some("Parent".to_string()),
        permission: None,
    })
    .await
    .expect("parent session")
}

async fn create_task(
    directory: &std::path::Path,
    parent_id: &str,
) -> crate::session::runtime::SessionSnapshot {
    let scope = crate::session::runtime::SessionScope::new(TenantScope::default(), directory)
        .expect("test scope");
    crate::session::runtime::create_visible_task_session(
        crate::session::runtime::CreateSessionInput {
            scope,
            parent_id: Some(parent_id.to_string()),
            title: Some("Task".to_string()),
            permission: None,
        },
    )
    .await
    .expect("visible task")
}

#[tokio::test]
async fn session_tool_create_returns_before_background_turn_finishes() {
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let coordinator = TaskSessionCoordinator::new();
    let (started_tx, started_rx) = oneshot::channel();
    let (_finish_tx, finish_rx) = oneshot::channel::<()>();
    let started_tx = std::sync::Mutex::new(Some(started_tx));
    let finish_rx = std::sync::Mutex::new(Some(finish_rx));
    let executor = Arc::new(
        move |_directory: String, request: vw_api_types::chat::GatewayChatStreamRequest| {
            let content = request.messages[0]["content"].as_str().expect("user prompt content");
            let source =
                vw_shared::session::session_utils::parse_task_session_source_marker(content)
                    .expect("source marker");
            assert_eq!(source.content, "Work");
            started_tx.lock().expect("started lock").take().expect("one call").send(()).ok();
            let finish_rx = finish_rx.lock().expect("finish lock").take().expect("one call");
            async move {
                let _ = finish_rx.await;
                Ok(GatewayChatStreamCollected {
                    output: "done".to_string(),
                    message_id: None,
                    parent_message_id: None,
                })
            }
            .boxed()
        },
    );
    let tool = SessionTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
        .with_coordinator(coordinator.clone())
        .with_stream_executor(executor);

    let result = tokio::time::timeout(
        Duration::from_millis(300),
        tool.execute(json!({"action": "create", "title": "Task", "prompt": "Work"})),
    )
    .await
    .expect("create must return before the stream finishes")
    .expect("create should execute");
    assert!(result.success);
    started_rx.await.expect("background stream should start");
    let output: serde_json::Value = serde_json::from_str(&result.output).expect("stable JSON");
    let task_id = output["session_id"].as_str().expect("session id");
    let snapshots = coordinator.snapshots(&parent.info.id, &[task_id.to_string()]).expect("state");
    assert_eq!(snapshots[0].state, TaskSessionState::Running);
}

#[tokio::test]
async fn session_tool_create_rejects_missing_context() {
    let tool = SessionTool::new(None, std::path::PathBuf::new());
    let error = tool
        .execute(json!({"action": "create", "title": "Task", "prompt": "Work"}))
        .await
        .expect_err("missing session context must fail");
    assert!(error.to_string().contains("current session"));

    let project = TempDir::new().expect("project directory");
    let tool = SessionTool::new(Some("parent".to_string()), project.path().to_path_buf());
    let missing_title = tool
        .execute(json!({"action": "create", "title": " ", "prompt": "Work"}))
        .await
        .expect_err("blank title must fail");
    assert!(missing_title.to_string().contains("'title' must be a non-empty string"));
    let missing_prompt = tool
        .execute(json!({"action": "create", "title": "Task", "prompt": ""}))
        .await
        .expect_err("blank prompt must fail");
    assert!(missing_prompt.to_string().contains("'prompt' must be a non-empty string"));
}

#[tokio::test]
async fn session_tool_send_continues_existing_visible_task() {
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let task = create_task(project.path(), &parent.info.id).await;
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register(&parent.info.id, &task.info.id).expect("register");
    coordinator.mark_running(&parent.info.id, &task.info.id).expect("start");
    coordinator.mark_succeeded(&parent.info.id, &task.info.id).expect("finish");
    let (request_tx, request_rx) = oneshot::channel();
    let request_tx = std::sync::Mutex::new(Some(request_tx));
    let executor = Arc::new(
        move |_directory: String, request: vw_api_types::chat::GatewayChatStreamRequest| {
            request_tx
                .lock()
                .expect("request lock")
                .take()
                .expect("one request")
                .send(request)
                .ok();
            async {
                Ok(GatewayChatStreamCollected {
                    output: "done".into(),
                    message_id: None,
                    parent_message_id: None,
                })
            }
            .boxed()
        },
    );
    let tool = SessionTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
        .with_coordinator(coordinator.clone())
        .with_stream_executor(executor);

    let result = tool
        .execute(json!({"action":"send","session_id":task.info.id,"prompt":"Continue"}))
        .await
        .expect("send");
    assert!(result.success);
    let request = request_rx.await.expect("background request");
    assert_eq!(request.session_id.as_ref().map(AsRef::as_ref), Some(task.info.id.as_str()));
    let source = vw_shared::session::session_utils::parse_task_session_source_marker(
        request.messages[0]["content"].as_str().expect("content"),
    )
    .expect("source marker");
    assert_eq!(source.content, "Continue");
    assert_eq!(source.source_session_id, parent.info.id);
}

#[tokio::test]
async fn session_tool_lists_statuses_reads_and_rejects_non_task_targets() {
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let task = create_task(project.path(), &parent.info.id).await;
    let scope = crate::session::runtime::SessionScope::new(TenantScope::default(), project.path())
        .expect("test scope");
    let internal = crate::session::runtime::create_session(
        crate::session::runtime::CreateSessionInput {
            scope,
            parent_id: Some(parent.info.id.clone()),
            title: Some("Internal".to_string()),
            permission: None,
        },
    )
    .await
    .expect("internal child session");
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register(&parent.info.id, &task.info.id).expect("register");
    coordinator.mark_running(&parent.info.id, &task.info.id).expect("start");
    coordinator.mark_succeeded(&parent.info.id, &task.info.id).expect("finish");
    let tool = SessionTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
        .with_coordinator(coordinator);

    let listed = tool.execute(json!({"action":"list"})).await.expect("list");
    let listed: serde_json::Value = serde_json::from_str(&listed.output).expect("list JSON");
    assert_eq!(listed["tasks"][0]["session_id"], task.info.id);
    assert_eq!(listed["tasks"][0]["state"], "succeeded");

    let status = tool
        .execute(json!({"action":"status","session_ids":[task.info.id]}))
        .await
        .expect("status");
    let status: serde_json::Value = serde_json::from_str(&status.output).expect("status JSON");
    assert_eq!(status["tasks"][0]["state"], "succeeded");

    let read = tool
        .execute(json!({"action":"read","session_id":task.info.id}))
        .await
        .expect("read");
    let read: serde_json::Value = serde_json::from_str(&read.output).expect("read JSON");
    assert_eq!(read["session_id"], task.info.id);
    assert_eq!(read["messages"], json!([]));
    assert_eq!(read["truncated"], false);

    let internal_error = tool
        .execute(json!({"action":"read","session_id":internal.info.id}))
        .await
        .expect_err("internal child must stay inaccessible");
    assert!(internal_error.to_string().contains("not a visible task session"));
    assert!(
        tool.execute(json!({"action":"read","session_id":"unknown-session"}))
            .await
            .is_err(),
        "unknown session IDs must be rejected"
    );
}

#[tokio::test]
async fn session_tool_rejects_cross_scope_and_busy_targets() {
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let task = create_task(project.path(), &parent.info.id).await;
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register(&parent.info.id, &task.info.id).expect("register");
    coordinator.mark_running(&parent.info.id, &task.info.id).expect("start");
    let tool = SessionTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
        .with_coordinator(coordinator);

    let busy = tool
        .execute(json!({"action":"send","session_id":task.info.id,"prompt":"Again"}))
        .await
        .expect_err("busy target");
    assert!(busy.to_string().contains("busy"));
    let self_target = tool
        .execute(json!({"action":"read","session_id":parent.info.id}))
        .await
        .expect_err("self target");
    assert!(self_target.to_string().contains("current session"));

    let other_project = TempDir::new().expect("other project");
    let other_parent = create_parent(other_project.path()).await;
    let other_task = create_task(other_project.path(), &other_parent.info.id).await;
    let cross_scope = tool
        .execute(json!({"action":"read","session_id":other_task.info.id}))
        .await
        .expect_err("cross scope");
    assert!(cross_scope.to_string().contains("ownership mismatch"));
}

#[tokio::test]
async fn session_tool_wait_returns_first_changed_target() {
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let first = create_task(project.path(), &parent.info.id).await;
    let second = create_task(project.path(), &parent.info.id).await;
    let coordinator = TaskSessionCoordinator::new();
    for id in [&first.info.id, &second.info.id] {
        coordinator.register(&parent.info.id, id).expect("register");
        coordinator.mark_running(&parent.info.id, id).expect("start");
    }
    let tool = SessionTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
        .with_coordinator(coordinator.clone());
    let first_id = first.info.id.clone();
    let parent_id = parent.info.id.clone();
    let transition = tokio::spawn(async move {
        tokio::task::yield_now().await;
        coordinator.mark_succeeded(&parent_id, &first_id).expect("finish first");
    });
    let result = tool.execute(json!({"action":"wait","session_ids":[first.info.id, second.info.id],"condition":"any","timeout_ms":1000})).await.expect("wait");
    transition.await.expect("transition");
    let output: serde_json::Value = serde_json::from_str(&result.output).expect("JSON");
    assert_eq!(output["timed_out"], false);
    assert_eq!(output["tasks"][0]["state"], "succeeded");
    assert_eq!(output["tasks"][1]["state"], "running");
}

#[test]
fn session_tool_read_bounds_serialized_output() {
    let output = super::bounded_read_output(
        "task",
        vec![json!({"text": "x".repeat(super::MAX_SESSION_READ_CHARS)}), json!({"text":"recent"})],
        512,
    )
    .expect("bounded output");
    assert!(output.chars().count() <= 512);
    let value: serde_json::Value = serde_json::from_str(&output).expect("valid bounded JSON");
    assert_eq!(value["truncated"], true);
    assert_eq!(value["messages"][0]["text"], "recent");
}
