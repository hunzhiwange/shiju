use super::*;
use serde_json::json;
use std::sync::{Arc, Mutex};

fn request(root: &std::path::Path, query: &str) -> Request {
    Request {
        stream: 0,
        session: "workflow-binding-test".into(),
        query: query.into(),
        root: Some(root.to_string_lossy().into_owned()),
        model: Some("unused-provider/unused-model".into()),
        reliability: Default::default(),
        options: json!({"agent":"main","allowed_tools":["bash","file_read"]}),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: vec![],
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools: Default::default(),
        session_skills: Default::default(),
    }
}

#[test]
fn native_workflow_bound_command_executes_graph_and_returns_answer() {
    let dir = workflow_engine::test_support::binding_fixture(
        crate::session::ui_store::current_tenant_scope(),
    );
    let request = request(dir.path(), "/shiju 剪辑 sp7");
    let context = ToolRuntimeContext::new(request.session.clone(), request.root.clone());
    let events = Arc::new(Mutex::new(Vec::new()));
    let recorded = events.clone();
    assert!(run_bound_command(&request, &context, &mut move |event| {
        recorded.lock().unwrap().push(event);
        true
    }));
    let events = events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(e, StreamEvent::Delta(text) if text == "verified preview"))
    );
    assert!(matches!(events.last(), Some(StreamEvent::Done(_))));
}

#[tokio::test]
async fn native_workflow_provider_names_preserve_tool_restrictions() {
    let tools = agent::NativeTools {
        context: ToolRuntimeContext::default(),
        allowed_tools: Some(json!([])),
    };
    workflow_engine::test_support::native_workflow_provider_names_preserve_tool_restrictions(&tools).await;
}

#[tokio::test]
async fn native_workflow_provider_names_preserve_agent_restrictions() {
    let (events, _receiver) = tokio::sync::mpsc::unbounded_channel();
    let native = agent::NativeAgent {
        parent: request(std::path::Path::new("."), "unused"),
        events,
        cancellation: tokio_util::sync::CancellationToken::new(),
    };
    let tools = Arc::new(agent::NativeTools {
        context: ToolRuntimeContext::default(),
        allowed_tools: Some(json!([])),
    });
    workflow_engine::test_support::native_workflow_provider_names_preserve_agent_restrictions(&native, tools).await;
}

fn action_runtime(root: &std::path::Path, allowed: Vec<&str>) -> (WorkflowRuntime, tempfile::TempDir) {
    let root = Some(root.to_string_lossy().into_owned());
    // Keep audit storage outside the Git worktree, as in the application config directory.
    let config_dir = tempfile::tempdir().unwrap();
    let config = crate::config::Config {
        workspace_dir: root.as_ref().unwrap().into(),
        config_path: config_dir.path().join(".vibewindow.json"),
        ..Default::default()
    };
    let context = ToolRuntimeContext::new("actions", root.clone())
        .with_test_config(config)
        .with_tool_use_context(
            crate::tools::ToolUseContext::new("actions", root)
                .with_full_access_enabled(true)
                .with_bypass_non_cli_approval_for_turn(true),
        );
    let runtime = crate::workflow::WorkflowRuntime {
        video_edit_budget: Default::default(),
        provider: Arc::new(workflow_engine::test_support::CloudFixtureProvider { invalid_assets: false }),
        model: "unused".into(),
        temperature: 0.0,
        tool_provider: Some(Arc::new(agent::NativeTools {
            context,
            allowed_tools: Some(json!(allowed)),
        })),
        agent_provider: None,
        knowledge_provider: None,
        document_extractor: None,
        pause_store: None,
    };
    (runtime, config_dir)
}

#[tokio::test]
async fn task_pool_actions_absent_steps_have_no_side_effects() {
    workflow_engine::test_support::task_pool_actions_absent_steps_have_no_side_effects(action_runtime).await;
}

#[tokio::test]
async fn task_pool_actions_explicit_git_steps_use_native_tool_permissions() {
    workflow_engine::test_support::task_pool_actions_explicit_git_steps_use_native_tool_permissions(action_runtime).await;
}

#[tokio::test]
async fn task_pool_actions_upstream_failure_skips_downstream_send() {
    workflow_engine::test_support::task_pool_actions_upstream_failure_skips_downstream_send(action_runtime).await;
}
