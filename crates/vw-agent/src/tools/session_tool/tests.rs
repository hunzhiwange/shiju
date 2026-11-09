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
async fn session_tool_recovers_visible_tasks_after_coordinator_restart() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let task = create_task(project.path(), &parent.info.id).await;
    let old = TaskSessionCoordinator::new();
    old.register(&parent.info.id, &task.info.id, None).expect("register");
    old.mark_running(&parent.info.id, &task.info.id).expect("start");
    super::persist_task_state(
        &old,
        &TenantScope::default(),
        &parent.info.id,
        &task.info.id,
        &task.info.project_id,
    )
    .expect("persist running turn");
    let restarted = SessionTool::new(Some(parent.info.id.clone()), project.path().to_path_buf())
        .with_coordinator(TaskSessionCoordinator::new());

    let listed = restarted.execute(json!({"action":"list"})).await.expect("list");
    let listed: serde_json::Value = serde_json::from_str(&listed.output).expect("JSON");
    assert_eq!(listed["tasks"][0]["session_id"], task.info.id);
    assert_eq!(listed["tasks"][0]["state"], "failed");
    let status = restarted
        .execute(json!({"action":"status","session_ids":[task.info.id]}))
        .await
        .expect("status");
    let status: serde_json::Value = serde_json::from_str(&status.output).expect("JSON");
    assert_eq!(status["tasks"][0]["state"], "failed");
    let executor =
        Arc::new(|_directory: String, _request: vw_api_types::chat::GatewayChatStreamRequest| {
            async {
                Ok(GatewayChatStreamCollected {
                    output: "continued".to_string(),
                    message_id: None,
                    parent_message_id: None,
                })
            }
            .boxed()
        });
    let restarted = restarted.with_stream_executor(executor);
    let sent = restarted
        .execute(json!({"action":"send","session_id":task.info.id,"prompt":"Continue"}))
        .await
        .expect("send after recovery");
    assert!(sent.success);
    assert_eq!(wait_for_task(&restarted, &task.info.id).await["state"], "succeeded");
}

#[tokio::test]
async fn session_tool_first_response_keeps_running_task_out_of_history() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let coordinator = TaskSessionCoordinator::new();
    let (started_tx, started_rx) = oneshot::channel();
    let (finish_tx, finish_rx) = oneshot::channel::<()>();
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
        Duration::from_secs(5),
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
    assert_eq!(output["title"], "Task");
    let task = tool.validated_target(task_id).await.expect("running task remains addressable");
    assert!(task.info.is_visible_task());
    assert!(task.info.title.is_empty(), "title before first response: {:?}", task.info.title);
    assert!(task.info.is_first_response_pending());
    let listed = tool.execute(json!({"action":"list"})).await.expect("list running task");
    let listed: serde_json::Value = serde_json::from_str(&listed.output).expect("list JSON");
    assert_eq!(listed["tasks"][0]["session_id"], task_id);
    assert_eq!(listed["tasks"][0]["title"], "Task");
    assert!(
        tool.execute(json!({"action":"send", "session_id":task_id, "prompt":"Too soon"}))
            .await
            .expect_err("running task rejects concurrent send")
            .to_string()
            .contains("busy")
    );
    let tenant = TenantScope::default();
    let project_id = Some(parent.info.project_id.as_str());
    assert!(
        crate::session::ui_store::load_agent_sessions_scoped_for_tenant(&tenant, project_id)
            .iter()
            .all(|info| info.id != task_id)
    );
    finish_tx.send(()).expect("release executor");
    let failed = wait_for_task(&tool, task_id).await;
    assert_eq!(failed["state"], "failed");
    assert!(failed["error"].as_str().unwrap().contains("did not commit"));
    assert!(tool.validated_target(task_id).await.unwrap().info.is_first_response_pending());
}

async fn wait_for_task(tool: &SessionTool, task_id: &str) -> serde_json::Value {
    let result = tool
        .execute(json!({"action":"wait","session_ids":[task_id],"timeout_ms":15000}))
        .await
        .expect("wait");
    let outcome: serde_json::Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(outcome["timed_out"], false, "{outcome}");
    outcome["tasks"][0].clone()
}

#[tokio::test(flavor = "multi_thread")]
async fn session_tool_first_response_retries_failed_task_on_same_id_and_publishes_once() {
    use crate::session::{llm::StreamEvent, message::AssistantError, processor, runtime, ui_store};
    use crate::tools::context::{ToolUseContext, scope_tool_use_context};

    let _env = crate::test_support::global_async_env_lock().lock().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let provider = axum::Router::new().route(
        "/v1/models",
        axum::routing::get(|| async { axum::Json(json!({"data": [{"id":"original"}]})) }),
    );
    let provider = tokio::spawn(async move { axum::serve(listener, provider).await });
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        &endpoint,
        Some("fixture-key"),
    )
    .await;
    let _streams = processor::test_llm_stream_guard();
    let workspace = TempDir::new().unwrap();
    let parent = create_parent(workspace.path()).await;
    let tenant = TenantScope::default();
    let scope = runtime::SessionScope::new(tenant.clone(), workspace.path()).unwrap();
    let project = parent.info.project_id.as_str();
    let new_tool = || {
        SessionTool::new(Some(parent.info.id.clone()), workspace.path().to_path_buf())
            .with_coordinator(TaskSessionCoordinator::new())
    };
    let context = Arc::new(
        ToolUseContext::new(&parent.info.id, Some(parent.info.directory.clone()))
            .with_provider("model-recovery")
            .with_model("original"),
    );

    processor::enqueue_test_llm_stream(vec![StreamEvent::Error(AssistantError::Unknown {
        message: "fixture model failure".into(),
    })]);
    let tool = new_tool();
    let created = scope_tool_use_context(
        context.clone(),
        tool.execute(json!({
            "action":"create", "title":"Research task", "prompt":"failed prompt",
        })),
    )
    .await
    .unwrap();
    let created: serde_json::Value = serde_json::from_str(&created.output).unwrap();
    let task_id = created["session_id"].as_str().unwrap();
    let failure = wait_for_task(&tool, task_id).await;
    assert_eq!(failure["state"], "failed", "{failure}");
    assert!(failure["error"].as_str().unwrap().contains("fixture model failure"), "{failure}");
    let pending = tool.validated_target(task_id).await.unwrap();
    assert!(pending.info.is_visible_task());
    assert!(pending.info.is_first_response_pending());
    assert!(pending.info.title.is_empty());
    assert!(!pending.history.is_empty(), "failed turn diagnostic history is retained");
    assert!(
        ui_store::load_session_meta_scoped_for_tenant(&tenant, task_id, Some(project))
            .unwrap()
            .is_none()
    );
    let other_tenant = TenantScope::new("other-platform", "other-company").unwrap();
    assert!(
        ui_store::with_tenant_scope(
            other_tenant,
            tool.execute(json!({
                "action":"read", "session_id":task_id,
            }))
        )
        .await
        .is_err()
    );

    let restarted = new_tool();
    let recovered = wait_for_task(&restarted, task_id).await;
    assert_eq!(recovered, failure);
    assert_eq!(recovered["title"], "Research task");

    for (prompt, output) in [("successful retry", "first answer"), ("follow up", "second answer")] {
        processor::enqueue_test_llm_stream(vec![
            StreamEvent::Delta(output.into()),
            StreamEvent::Done { usage: Default::default(), finish_reason: Some("stop".into()) },
        ]);
        let sent = scope_tool_use_context(
            context.clone(),
            restarted.execute(json!({
                "action":"send", "session_id":task_id, "prompt":prompt,
            })),
        )
        .await
        .unwrap();
        let sent: serde_json::Value = serde_json::from_str(&sent.output).unwrap();
        assert_eq!(sent["session_id"], task_id);
        let succeeded = wait_for_task(&restarted, task_id).await;
        assert_eq!(succeeded["state"], "succeeded", "{succeeded}");
        let meta = ui_store::load_session_meta_scoped_for_tenant(&tenant, task_id, Some(project))
            .unwrap()
            .expect("published metadata");
        assert_eq!(meta.title, "Research task");
        assert_eq!(meta.last_content.as_deref(), Some("successful retry"));
    }

    let published = new_tool();
    let recovered = wait_for_task(&published, task_id).await;
    assert_eq!(recovered["state"], "succeeded");
    let info = published.validated_target(task_id).await.unwrap().info;
    assert!(info.is_visible_task());
    assert!(!info.is_agent_tool() && !info.is_plan_run() && !info.is_first_response_pending());
    assert_eq!(info.slug, pending.info.source_slug());
    assert_eq!(
        ui_store::load_sessions_scoped_for_tenant(&tenant, Some(project))
            .iter()
            .filter(|meta| meta.id == task_id)
            .count(),
        1
    );
    let fork =
        runtime::fork_visible_task_session(scope, &parent.info.id, task_id, None).await.unwrap();
    assert!(fork.is_visible_task());
    assert!(!fork.is_first_response_pending());
    assert_ne!(fork.id, task_id);

    provider.abort();
    let _ = provider.await;
}

#[tokio::test]
async fn session_tool_create_rejects_missing_context() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
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
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let task = create_task(project.path(), &parent.info.id).await;
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register(&parent.info.id, &task.info.id, None).expect("register");
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
    assert_eq!(wait_for_task(&tool, &task.info.id).await["state"], "succeeded");
}

#[tokio::test]
async fn background_turn_reports_startup_failure_instead_of_succeeding_silently() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let task = create_task(project.path(), &parent.info.id).await;
    let executor_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let executor_called_for_call = Arc::clone(&executor_called);
    let executor = Arc::new(
        move |_directory: String, _request: vw_api_types::chat::GatewayChatStreamRequest| {
            executor_called_for_call.store(true, std::sync::atomic::Ordering::SeqCst);
            async {
                Ok(GatewayChatStreamCollected {
                    output: "unexpected".into(),
                    message_id: None,
                    parent_message_id: None,
                })
            }
            .boxed()
        },
    );
    let tool = SessionTool::new(Some("parent".into()), project.path().to_path_buf())
        .with_coordinator(TaskSessionCoordinator::new())
        .with_stream_executor(executor);

    let error = tool
        .spawn_background_turn(parent.info.id, task.info, "Continue".into())
        .expect_err("unregistered task startup must fail");

    assert!(error.to_string().contains("unknown task session"));
    assert!(!executor_called.load(std::sync::atomic::Ordering::SeqCst));
}

#[tokio::test]
async fn session_tool_lists_statuses_reads_and_rejects_non_task_targets() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let task = create_task(project.path(), &parent.info.id).await;
    let scope = crate::session::runtime::SessionScope::new(TenantScope::default(), project.path())
        .expect("test scope");
    let internal =
        crate::session::runtime::create_session(crate::session::runtime::CreateSessionInput {
            scope,
            parent_id: Some(parent.info.id.clone()),
            title: Some("Internal".to_string()),
            permission: None,
        })
        .await
        .expect("internal child session");
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register(&parent.info.id, &task.info.id, None).expect("register");
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

    let read =
        tool.execute(json!({"action":"read","session_id":task.info.id})).await.expect("read");
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
        tool.execute(json!({"action":"read","session_id":"unknown-session"})).await.is_err(),
        "unknown session IDs must be rejected"
    );
}

#[tokio::test]
async fn session_tool_rejects_cross_scope_and_busy_targets() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let task = create_task(project.path(), &parent.info.id).await;
    let coordinator = TaskSessionCoordinator::new();
    coordinator.register(&parent.info.id, &task.info.id, None).expect("register");
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
    assert!(cross_scope.to_string().contains("does not belong to directory"));
}

#[tokio::test]
async fn session_tool_rejects_other_parent_visible_task() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let other_parent = create_parent(project.path()).await;
    let other_task = create_task(project.path(), &other_parent.info.id).await;
    let tool = SessionTool::new(Some(parent.info.id), project.path().to_path_buf())
        .with_coordinator(TaskSessionCoordinator::new());

    for args in [
        json!({"action":"read","session_id":other_task.info.id}),
        json!({"action":"status","session_ids":[other_task.info.id]}),
        json!({"action":"wait","session_ids":[other_task.info.id],"timeout_ms":1}),
        json!({"action":"send","session_id":other_task.info.id,"prompt":"Continue"}),
    ] {
        let error = tool.execute(args.clone()).await.expect_err("other parent's task must fail");
        assert!(error.to_string().contains("parent session mismatch"), "{args}: {error}");
    }
}

#[tokio::test]
async fn session_tool_wait_returns_first_changed_target() {
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let project = TempDir::new().expect("project directory");
    let parent = create_parent(project.path()).await;
    let first = create_task(project.path(), &parent.info.id).await;
    let second = create_task(project.path(), &parent.info.id).await;
    let coordinator = TaskSessionCoordinator::new();
    for id in [&first.info.id, &second.info.id] {
        coordinator.register(&parent.info.id, id, None).expect("register");
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
