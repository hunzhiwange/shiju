use super::*;
use crate::app::agent::config::DelegateAgentConfig;
use crate::app::agent::tools::traits::Tool;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::sync::oneshot;

async fn create_parent(directory: String) -> crate::session::session::Info {
    crate::app::agent::project::instance::provide(directory.clone(), None, move || {
        Box::pin(async move {
            crate::session::session::create_next(crate::session::session::CreateInput {
                parent_id: None,
                title: Some("AgentTool parent".to_string()),
                directory,
                permission: None,
            })
            .await
        })
    })
    .await
    .expect("project context should initialize")
    .expect("parent session should persist")
}

async fn load_session(directory: String, session_id: String) -> crate::session::session::Info {
    crate::app::agent::project::instance::provide(directory, None, move || {
        Box::pin(async move { crate::session::session::get(&session_id).await })
    })
    .await
    .expect("project context should initialize")
    .expect("session should persist")
}

fn build_tool_with_agents(agents: HashMap<String, DelegateAgentConfig>) -> AgentTool {
    AgentTool::new(agents, None, std::path::PathBuf::new())
}

fn build_tool() -> AgentTool {
    build_tool_with_agents(HashMap::from([("writer".to_string(), DelegateAgentConfig::default())]))
}

#[test]
fn agent_tool_spec_allows_isolated_child_sessions_to_run_concurrently() {
    let spec = build_tool().spec();

    assert!(!spec.read_only);
    assert!(spec.concurrency_safe);
}

fn build_persistent_tool<F>(
    parent_session_id: String,
    directory: String,
    execute_stream: F,
) -> AgentTool
where
    F: Fn(
            vw_api_types::chat::GatewayChatStreamRequest,
        ) -> futures_util::future::BoxFuture<
            'static,
            Result<
                crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected,
                crate::gateway::ApiError,
            >,
        > + Send
        + Sync
        + 'static,
{
    let agents = HashMap::from([("writer".to_string(), DelegateAgentConfig::default())]);
    let execute_stream = Arc::new(execute_stream);
    AgentTool::new(agents, Some(parent_session_id), directory.into())
        .with_child_agent_stream_executor(execute_stream)
}

#[test]
fn action_validation_accepts_launch_aliases_and_rejects_legacy_actions() {
    let tool = build_tool();

    for (args, expected) in [
        (json!({}), AgentToolAction::Launch),
        (json!({"action": "launch"}), AgentToolAction::Launch),
        (json!({"action": "run"}), AgentToolAction::Run),
        (json!({"action": " SPAWN "}), AgentToolAction::Launch),
        (json!({"action": "status"}), AgentToolAction::Status),
        (json!({"action": "wait"}), AgentToolAction::Wait),
        (json!({"action": "stop"}), AgentToolAction::Stop),
    ] {
        assert_eq!(
            tool.action_from_args(&args).expect("supported action should be accepted"),
            expected
        );
    }

    for action in ["list", "get", "kill", "message", "dance"] {
        let err = tool
            .action_from_args(&json!({"action": action}))
            .expect_err("legacy or unknown action should fail");
        assert!(err.to_string().contains("Unknown AgentTool action"));
    }
}

#[test]
fn argument_helpers_trim_and_validate_agent_alias() {
    let tool = build_tool();
    let args = json!({
        "subagent_type": " writer ",
        "prompt": " draft "
    });

    assert_eq!(tool.agent_name_from_args(&args), Some("writer"));
    assert_eq!(tool.prompt_from_args(&args), Some("draft"));
}

#[test]
fn argument_helpers_cover_primary_fields_and_types() {
    let tool = build_tool();
    let primary = json!({
        "agent": " writer ",
        "subagent_type": "ignored",
        "prompt": " draft "
    });

    assert_eq!(tool.agent_name_from_args(&primary), Some("writer"));
    assert_eq!(tool.prompt_from_args(&primary), Some("draft"));
    assert_eq!(tool.agent_name_from_args(&json!({"agent": 7})), None);
    assert_eq!(tool.prompt_from_args(&json!({"prompt": false})), None);
}

#[test]
fn blank_primary_fields_do_not_fall_back_to_aliases() {
    let tool = build_tool();
    let args = json!({
        "agent": "   ",
        "subagent_type": "writer",
        "prompt": "\t"
    });

    assert_eq!(tool.agent_name_from_args(&args), None);
    assert_eq!(tool.prompt_from_args(&args), None);

    let typed_aliases = json!({
        "agent": 7,
        "subagent_type": " writer ",
        "prompt": false
    });
    assert_eq!(tool.agent_name_from_args(&typed_aliases), Some("writer"));
    assert_eq!(tool.prompt_from_args(&typed_aliases), None);
}

#[test]
fn spec_and_schema_describe_configured_agents() {
    let tool = build_tool_with_agents(HashMap::from([(
        "writer".to_string(),
        DelegateAgentConfig {
            label: Some("Writer".to_string()),
            description: Some("Draft and revise documents".to_string()),
            ..DelegateAgentConfig::default()
        },
    )]));

    assert_eq!(tool.name(), "AgentTool");
    let description = tool.description();
    for expected in ["complex", "simple", "parallel", "synthesize"] {
        assert!(
            description.contains(expected),
            "AgentTool description should explain {expected}: {description}"
        );
    }
    for unsupported in ["action=list", "action=get"] {
        assert!(
            !description.contains(unsupported),
            "AgentTool description must not advertise unsupported '{unsupported}': {description}"
        );
    }

    let schema = tool.parameters_schema();
    let agent_description = schema["properties"]["agent"]["description"]
        .as_str()
        .expect("agent description");
    for expected in ["writer", "Writer", "Draft and revise documents"] {
        assert!(
            agent_description.contains(expected),
            "agent description should contain {expected}: {agent_description}"
        );
    }
    assert_eq!(schema["properties"]["agent"]["enum"], json!(["writer"]));
    assert_eq!(schema["properties"]["subagent_type"]["enum"], json!(["writer"]));
    assert!(
        schema["properties"]["prompt"]["description"]
            .as_str()
            .expect("prompt description")
            .contains("validation")
    );

    let spec = tool.spec();
    assert_eq!(spec.id, "AgentTool");
    assert!(spec.aliases.contains(&"agent".to_string()));
    assert!(!spec.read_only);
    assert!(!spec.destructive);
    assert!(spec.concurrency_safe);
    assert!(!spec.requires_user_interaction);
    assert!(spec.strict);
}

#[test]
fn schema_is_closed_and_agent_description_is_stable() {
    let agents = HashMap::from([
        ("zeta".to_string(), DelegateAgentConfig::default()),
        ("alpha".to_string(), DelegateAgentConfig::default()),
    ]);
    let tool = build_tool_with_agents(agents);

    assert_eq!(tool.available_agents_description(), "alpha, zeta");
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["action"]["enum"],
        json!(["launch", "run", "spawn", "status", "wait", "stop"])
    );
    for removed_property in ["task", "run_in_background", "session_id", "status"] {
        assert!(
            schema["properties"].get(removed_property).is_none(),
            "legacy property '{removed_property}' must not be exposed"
        );
    }

    let empty = build_tool_with_agents(HashMap::new());
    assert_eq!(empty.available_agents_description(), "（未配置）");
    assert!(
        empty.parameters_schema()["properties"]["agent"]["description"]
            .as_str()
            .unwrap()
            .contains("（未配置）")
    );
}

#[tokio::test]
async fn execute_rejects_missing_required_parameters_before_delegation() {
    let tool = build_tool();

    let missing_agent =
        tool.execute(json!({"action": "launch", "prompt": "draft"})).await.unwrap_err();
    assert!(missing_agent.to_string().contains("Missing 'agent' parameter"));

    let missing_prompt =
        tool.execute(json!({"action": "launch", "agent": "writer"})).await.unwrap_err();
    assert!(missing_prompt.to_string().contains("Missing 'prompt' parameter"));
}

#[tokio::test]
async fn launch_reports_unknown_agent() {
    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_in_tool = callback_called.clone();
    let tool = build_tool().with_child_session_created_callback(Arc::new(move |_created| {
        callback_called_in_tool.store(true, Ordering::SeqCst);
    }));

    let result = tool
        .execute(json!({"action": "launch", "agent": "missing", "prompt": "draft"}))
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("Unknown agent 'missing'"));
    assert!(!callback_called.load(Ordering::SeqCst));
}

#[tokio::test]
async fn launch_without_parent_does_not_report_running_child() {
    let callback_called = Arc::new(AtomicBool::new(false));
    let callback_called_in_tool = callback_called.clone();
    let tool = build_tool().with_child_session_created_callback(Arc::new(move |_created| {
        callback_called_in_tool.store(true, Ordering::SeqCst);
    }));

    let error = tool
        .execute(json!({"action": "launch", "agent": "writer", "prompt": "draft"}))
        .await
        .expect_err("missing parent should fail");

    assert!(error.to_string().contains("current parent session ID"));
    assert!(!callback_called.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "current_thread")]
async fn run_waits_for_persistent_child_session() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let completed = Arc::new(AtomicBool::new(false));
    let completed_in_executor = completed.clone();
    let release_rx = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));
    let started_tx = Arc::new(std::sync::Mutex::new(Some(started_tx)));
    let tool = build_persistent_tool(parent.id.clone(), directory, move |_request| {
        let release_rx = release_rx.clone();
        let started_tx = started_tx.clone();
        let completed = completed_in_executor.clone();
        Box::pin(async move {
            started_tx
                .lock()
                .expect("started lock")
                .take()
                .expect("one child")
                .send(())
                .expect("executor should start");
            release_rx
                .lock()
                .await
                .take()
                .expect("one child")
                .await
                .expect("release child executor");
            completed.store(true, Ordering::SeqCst);
            Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                output: "child completed".to_string(),
                message_id: None,
                parent_message_id: None,
            })
        })
    });

    let task = tokio::spawn(async move {
        tool.execute(json!({"action": "run", "agent": "writer", "prompt": "inspect"})).await
    });
    started_rx.await.expect("AgentTool should start child stream");
    assert!(!task.is_finished(), "AgentTool must wait for child completion");
    release_tx.send(()).expect("release child executor");
    let result = task.await.expect("launch task should join").expect("launch should return");
    let body: serde_json::Value = serde_json::from_str(&result.output).expect("structured result");

    assert!(completed.load(Ordering::SeqCst));
    assert!(result.success);
    assert_eq!(body["parent_session_id"], parent.id);
    assert_ne!(body["child_session_id"], parent.id);
    assert_eq!(body["terminal_state"], "succeeded");
    assert_eq!(body["output"], "child completed");
}

#[tokio::test(flavor = "current_thread")]
async fn run_leaves_model_selection_to_child_agent_config() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let tool = build_persistent_tool(parent.id, directory, |request| {
        Box::pin(async move {
            assert_eq!(request.model, None);
            Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                output: "child completed".to_string(),
                message_id: None,
                parent_message_id: None,
            })
        })
    });

    let result = tool
        .execute(json!({"action": "run", "agent": "writer", "prompt": "inspect"}))
        .await
        .expect("child run should complete");

    assert!(result.success);
}

#[tokio::test(flavor = "current_thread")]
async fn launch_returns_running_before_child_completes_and_wait_observes_completion() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let started_in_executor = started.clone();
    let release_in_executor = release.clone();
    let coordinator =
        crate::session::child_agent::coordinator::ChildAgentCoordinator::new(8);
    let launcher = build_persistent_tool(parent.id.clone(), directory.clone(), move |_request| {
        let started = started_in_executor.clone();
        let release = release_in_executor.clone();
        Box::pin(async move {
            started.notify_one();
            release.notified().await;
            Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                output: "background complete".to_string(),
                message_id: None,
                parent_message_id: None,
            })
        })
    })
    .with_coordinator(coordinator.clone());
    let controller = AgentTool::new(
        HashMap::from([("writer".to_string(), DelegateAgentConfig::default())]),
        Some(parent.id.clone()),
        directory.into(),
    )
    .with_coordinator(coordinator);

    let launch = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        launcher.execute(json!({
            "action": "launch",
            "agent": "writer",
            "prompt": "inspect in background"
        })),
    )
    .await
    .expect("background launch should return before the child completes")
    .expect("launch should succeed");
    let launch_body: serde_json::Value =
        serde_json::from_str(&launch.output).expect("structured launch result");
    assert_eq!(launch_body["state"], "running");
    let child_session_id = launch_body["child_session_id"]
        .as_str()
        .expect("child session ID")
        .to_string();
    tokio::time::timeout(std::time::Duration::from_millis(200), started.notified())
        .await
        .expect("background child should start");

    release.notify_waiters();
    let waited = controller
        .execute(json!({
            "action": "wait",
            "child_session_ids": [child_session_id],
            "condition": "all",
            "timeout_ms": 1_000
        }))
        .await
        .expect("wait should succeed");
    let wait_body: serde_json::Value =
        serde_json::from_str(&waited.output).expect("structured wait result");

    assert_eq!(wait_body["timed_out"], false);
    assert_eq!(wait_body["tasks"][0]["state"], "succeeded");
    assert_eq!(wait_body["tasks"][0]["output"], "background complete");
}

#[tokio::test]
async fn status_wait_timeout_and_stop_are_scoped_to_the_parent_session() {
    let coordinator =
        crate::session::child_agent::coordinator::ChildAgentCoordinator::new(8);
    let owned_cancellation = coordinator
        .register("parent-1", "child-owned", "writer", "owned task")
        .expect("owned task should register");
    coordinator
        .mark_running("parent-1", "child-owned")
        .expect("owned task should start");
    coordinator
        .register("parent-2", "child-foreign", "writer", "foreign task")
        .expect("foreign task should register");
    let tool = AgentTool::new(
        HashMap::from([("writer".to_string(), DelegateAgentConfig::default())]),
        Some("parent-1".to_string()),
        std::path::PathBuf::new(),
    )
    .with_coordinator(coordinator);

    let status = tool
        .execute(json!({"action": "status"}))
        .await
        .expect("status should succeed");
    let status_body: Value = serde_json::from_str(&status.output).expect("status result");
    assert_eq!(status_body["tasks"].as_array().expect("tasks").len(), 1);
    assert_eq!(status_body["tasks"][0]["child_session_id"], "child-owned");

    let waited = tool
        .execute(json!({
            "action": "wait",
            "child_session_id": "child-owned",
            "timeout_ms": 1
        }))
        .await
        .expect("wait should return a timeout snapshot");
    let wait_body: Value = serde_json::from_str(&waited.output).expect("wait result");
    assert_eq!(wait_body["timed_out"], true);
    assert_eq!(wait_body["tasks"][0]["state"], "running");

    let stopped = tool
        .execute(json!({"action": "stop", "child_session_ids": ["child-owned"]}))
        .await
        .expect("stop should succeed");
    let stop_body: Value = serde_json::from_str(&stopped.output).expect("stop result");
    assert_eq!(stop_body["tasks"][0]["state"], "cancelled");
    assert!(owned_cancellation.is_cancelled());

    let foreign_error = tool
        .execute(json!({"action": "status", "child_session_id": "child-foreign"}))
        .await
        .expect_err("another parent's child must not be visible");
    assert!(foreign_error.to_string().contains("does not belong to parent"));
}

#[tokio::test(flavor = "current_thread")]
async fn concurrent_runs_respect_configured_limit() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let active_in_executor = active.clone();
    let max_active_in_executor = max_active.clone();
    let tool = Arc::new(
        build_persistent_tool(parent.id, directory, move |_request| {
            let active = active_in_executor.clone();
            let max_active = max_active_in_executor.clone();
            Box::pin(async move {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(now, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                    output: "done".to_string(),
                    message_id: None,
                    parent_message_id: None,
                })
            })
        })
        .with_max_parallel_subagents(2),
    );

    let tasks = (0..5)
        .map(|index| {
            let tool = tool.clone();
            tokio::spawn(async move {
                tool.execute(json!({
                    "action": "run",
                    "agent": "writer",
                    "prompt": format!("task {index}")
                }))
                .await
            })
        })
        .collect::<Vec<_>>();
    for task in tasks {
        task.await.expect("launch task should join").expect("launch should return");
    }

    assert_eq!(max_active.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn launch_rejects_session_depth_at_agent_limit() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let root = create_parent(directory.clone()).await;
    let directory_for_chain = directory.clone();
    let root_id = root.id.clone();
    let grandchild =
        crate::app::agent::project::instance::provide(directory.clone(), None, move || {
            Box::pin(async move {
                let child =
                    crate::session::session::create_next(crate::session::session::CreateInput {
                        parent_id: Some(root_id),
                        title: Some("child".to_string()),
                        directory: directory_for_chain.clone(),
                        permission: None,
                    })
                    .await?;
                crate::session::session::create_next(crate::session::session::CreateInput {
                    parent_id: Some(child.id),
                    title: Some("grandchild".to_string()),
                    directory: directory_for_chain,
                    permission: None,
                })
                .await
            })
        })
        .await
        .expect("project context should initialize")
        .expect("session chain should persist");
    let executor_called = Arc::new(AtomicBool::new(false));
    let executor_called_in_tool = executor_called.clone();
    let agents = HashMap::from([(
        "main".to_string(),
        DelegateAgentConfig { max_depth: 2, ..DelegateAgentConfig::default() },
    )]);
    let tool = AgentTool::new(agents, Some(grandchild.id), directory.into())
        .with_child_agent_stream_executor(Arc::new(move |_request| {
            executor_called_in_tool.store(true, Ordering::SeqCst);
            Box::pin(async move {
                Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                    output: "should not run".to_string(),
                    message_id: None,
                    parent_message_id: None,
                })
            })
        }));

    let result = tool
        .execute(json!({"action": "launch", "agent": "main", "prompt": "recurse"}))
        .await
        .expect("depth rejection should return a tool result");

    assert!(!result.success);
    assert!(result.error.expect("depth error").contains("depth limit reached"));
    assert!(!executor_called.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "current_thread")]
async fn completed_launch_render_metadata_retains_child_session_id() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let tool = build_persistent_tool(parent.id.clone(), directory, |_request| {
        Box::pin(async move {
            Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                output: "child completed".to_string(),
                message_id: None,
                parent_message_id: None,
            })
        })
    });

    let result = tool
        .execute(json!({"action": "run", "agent": "writer", "prompt": "inspect"}))
        .await
        .expect("launch should return");
    let lifted = ToolCallResult::from_legacy_result(result);
    let hint = tool.render_hint(&lifted).expect("AgentTool render metadata");
    let output: Value =
        serde_json::from_str(lifted.data["output"].as_str().expect("structured output"))
            .expect("parse output");

    assert_eq!(hint.metadata["parent_session_id"], parent.id);
    assert_eq!(hint.metadata["child_session_id"], output["child_session_id"]);
    assert_eq!(hint.metadata["agent"], "writer");
    assert_eq!(hint.metadata["task"], "inspect");
}

#[test]
fn single_task_status_render_metadata_retains_child_session_id() {
    let tool = build_tool();
    let lifted = ToolCallResult::from_legacy_result(ToolResult {
        success: true,
        output: json!({
            "action": "status",
            "tasks": [{
                "parent_session_id": "parent",
                "child_session_id": "child",
                "agent": "writer",
                "task": "inspect",
                "state": "running",
            }],
        })
        .to_string(),
        error: None,
    });
    let hint = tool.render_hint(&lifted).expect("single task render metadata");

    assert_eq!(hint.metadata["parent_session_id"], "parent");
    assert_eq!(hint.metadata["child_session_id"], "child");
    assert_eq!(hint.metadata["agent"], "writer");
    assert_eq!(hint.summary.as_deref(), Some("inspect"));
}

#[tokio::test(flavor = "current_thread")]
async fn agent_tool_running_event_exposes_persistent_child_before_completion() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = oneshot::channel();
    let release_rx = Arc::new(tokio::sync::Mutex::new(Some(release_rx)));
    let started_tx = Arc::new(std::sync::Mutex::new(Some(started_tx)));
    let (created_tx, created_rx) = oneshot::channel();
    let created_tx = Arc::new(std::sync::Mutex::new(Some(created_tx)));
    let tool = build_persistent_tool(parent.id.clone(), directory.clone(), move |_request| {
        let release_rx = release_rx.clone();
        let started_tx = started_tx.clone();
        Box::pin(async move {
            started_tx
                .lock()
                .expect("started lock")
                .take()
                .expect("one child")
                .send(())
                .expect("executor should start");
            release_rx
                .lock()
                .await
                .take()
                .expect("one child")
                .await
                .expect("release child executor");
            Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                output: "child completed".to_string(),
                message_id: None,
                parent_message_id: None,
            })
        })
    })
    .with_child_session_created_callback(Arc::new(move |created| {
        created_tx
            .lock()
            .expect("created lock")
            .take()
            .expect("one child")
            .send(created)
            .expect("report child creation");
    }));

    let task = tokio::spawn(async move {
        tool.execute(json!({
            "action": "run",
            "agent": "writer",
            "prompt": "inspect the persistent child"
        }))
        .await
    });
    let created = created_rx.await.expect("running event should arrive");
    assert!(!task.is_finished(), "running event must arrive before completion");
    assert_eq!(created.parent_session_id, parent.id);
    assert_eq!(created.agent, "writer");
    assert_eq!(created.task, "inspect the persistent child");
    let child = load_session(directory, created.child_session_id.clone()).await;
    assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));

    started_rx.await.expect("child executor should start");
    release_tx.send(()).expect("release child executor");
    let result = task.await.expect("launch task should join").expect("launch should return");
    let body: Value = serde_json::from_str(&result.output).expect("structured result");
    assert_eq!(body["child_session_id"], created.child_session_id);
}

#[tokio::test(flavor = "current_thread")]
async fn failed_launch_allows_new_child_session_attempt() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let directory = workspace.path().to_string_lossy().to_string();
    let parent = create_parent(directory.clone()).await;
    let created_ids = Arc::new(std::sync::Mutex::new(Vec::new()));
    let created_ids_in_tool = created_ids.clone();
    let tool = build_persistent_tool(parent.id.clone(), directory, |_request| {
        Box::pin(async move { Err(crate::gateway::ApiError::internal("provider unavailable")) })
    })
    .with_child_session_created_callback(Arc::new(move |created| {
        created_ids_in_tool.lock().expect("created ids lock").push(created.child_session_id);
    }));

    let first = tool
        .execute(json!({"action": "run", "agent": "writer", "prompt": "inspect"}))
        .await
        .expect("failed child should produce a tool result");
    let second = tool
        .execute(json!({"action": "run", "agent": "writer", "prompt": "inspect again"}))
        .await
        .expect("new attempt should produce a tool result");
    let first_body: serde_json::Value = serde_json::from_str(&first.output).expect("first result");
    let second_body: serde_json::Value =
        serde_json::from_str(&second.output).expect("second result");

    assert!(!first.success);
    assert!(!second.success);
    assert_eq!(first_body["parent_session_id"], parent.id);
    assert_eq!(first_body["terminal_state"], "failed");
    assert!(
        first_body["error"].as_str().is_some_and(|error| error.contains("provider unavailable"))
    );
    assert_ne!(first_body["child_session_id"], second_body["child_session_id"]);
    let created_ids = created_ids.lock().expect("created ids lock");
    assert_eq!(first_body["child_session_id"], created_ids[0]);
    assert_eq!(second_body["child_session_id"], created_ids[1]);
}
