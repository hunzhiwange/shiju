//! Exercise thread commands through the synchronous runtime registry boundary.

use super::CreateThreadTool;
use crate::session::runtime::{
    CreateSessionInput, SessionScope, SessionToolset, create_session, create_visible_task_session,
};
use crate::session::task_session::{TaskSessionWaitCondition, global_task_session_coordinator};
use crate::tools::context::ToolUseContext;
use crate::tools::toolset::{ToolRuntimeContext, execute_tool_call};
use futures_util::FutureExt;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use vw_api_types::id::TenantScope;

async fn parent_context(project: &TempDir) -> (SessionScope, ToolRuntimeContext) {
    let scope = SessionScope::new(TenantScope::default(), project.path()).expect("scope");
    let parent = create_session(CreateSessionInput {
        scope: scope.clone(),
        parent_id: None,
        title: Some("Parent".into()),
        permission: None,
    })
    .await
    .expect("parent session");
    let context = ToolRuntimeContext::new(
        parent.info.id.clone(),
        Some(project.path().to_string_lossy().into_owned()),
    )
    .with_tool_use_context(
        ToolUseContext::new(parent.info.id, None)
            .with_provider("alibaba-token-plan-cn")
            .with_model("deepseek-v4.1-flash-1017"),
    )
    .with_test_config(crate::config::Config {
        workspace_dir: project.path().to_path_buf(),
        config_path: project.path().join("config.json"),
        ..Default::default()
    });
    (scope, context)
}

#[tokio::test]
async fn create_thread_registry_preserves_manual_model_in_background_request() {
    let project = TempDir::new().expect("project directory");
    let (_, context) = parent_context(&project).await;
    let (request_tx, request_rx) = std::sync::mpsc::channel();
    let executor =
        Arc::new(move |_: String, request: vw_api_types::chat::GatewayChatStreamRequest| {
            request_tx.send(request).expect("capture child request");
            async {
                Ok(crate::gateway::api::handlers::session::stream::GatewayChatStreamCollected {
                    output: "done".into(),
                    message_id: None,
                    parent_message_id: None,
                })
            }
            .boxed()
        });
    let tool = CreateThreadTool::new(Some(context.session.clone()), project.path().to_path_buf())
        .with_stream_executor(executor);
    let context = context.with_session_tools(SessionToolset::new(vec![Arc::new(tool)]));

    let result = execute_tool_call("create_thread", r#"{"prompt":"Work"}"#, &context)
        .expect("create through runtime registry");
    let output: Value = serde_json::from_str(&result.model_text()).expect("create JSON");
    let request = request_rx.recv_timeout(Duration::from_secs(2)).expect("background request");

    assert_eq!(
        request.model.as_deref(),
        Some("alibaba-token-plan-cn/deepseek-v4.1-flash-1017")
    );
    assert_eq!(request.session_id.as_ref().map(AsRef::as_ref), output["thread_id"].as_str());
    let completion = global_task_session_coordinator()
        .wait(
            &context.session,
            &[output["thread_id"].as_str().expect("thread id").into()],
            TaskSessionWaitCondition::All,
            Duration::from_secs(2),
        )
        .await
        .expect("background completion");
    assert!(!completion.timed_out);
    assert_eq!(
        completion.tasks[0].state,
        crate::session::task_session::TaskSessionState::Succeeded
    );
}

#[tokio::test]
async fn wait_threads_registry_reports_child_failure_while_other_target_runs() {
    let project = TempDir::new().expect("project directory");
    let (scope, context) = parent_context(&project).await;
    let coordinator = global_task_session_coordinator();
    let mut ids = Vec::new();
    for title in ["Failed task", "Running task"] {
        let task = create_visible_task_session(CreateSessionInput {
            scope: scope.clone(),
            parent_id: Some(context.session.clone()),
            title: Some(title.into()),
            permission: None,
        })
        .await
        .expect("visible task");
        coordinator.register(&context.session, &task.info.id).expect("register");
        coordinator.mark_running(&context.session, &task.info.id).expect("running");
        ids.push(task.info.id);
    }
    let error = json!({"name":"APIError","message":"error decoding response body"}).to_string();
    coordinator.mark_failed(&context.session, &ids[0], &error).expect("failure");

    let result = execute_tool_call(
        "wait_threads",
        &json!({"thread_ids":ids,"timeout_ms":0}).to_string(),
        &context,
    )
    .expect("wait through runtime registry");
    let output: Value = serde_json::from_str(&result.model_text()).expect("wait JSON");

    assert!(result.is_success(), "reading a failed child is a successful query");
    assert_eq!(output["timed_out"], false);
    assert_eq!(output["tasks"][0]["thread_id"], ids[0]);
    assert_eq!(output["tasks"][0]["state"], "failed");
    assert_eq!(output["tasks"][0]["error"], error);
    assert!(output["tasks"][0].get("task_session_id").is_none());
    assert_eq!(output["tasks"][1]["thread_id"], ids[1]);
    assert_eq!(output["tasks"][1]["state"], "running");
}
