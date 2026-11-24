use std::sync::Arc;

use agent_gateway::api::media::MediaJobState;

use crate::gateway::api::handlers::media::tests::Fixture;
use crate::tools::{
    ToolCallError, ToolRuntimeContext, execute_tool_from_registry, execution_tools_for_context,
};

use super::*;

fn registry(fixture: &Fixture, context: &ToolUseContext) -> Vec<Box<dyn Tool>> {
    let workspace = fixture.workspace();
    let config = crate::config::Config {
        workspace_dir: workspace.to_path_buf(),
        config_path: workspace.join(".vibewindow.json"),
        ..Default::default()
    };
    execution_tools_for_context(
        &ToolRuntimeContext::new(context.session(), Some(workspace.to_string_lossy().into_owned()))
            .with_tool_use_context(context.clone())
            .with_test_config(config),
    )
}

async fn call(
    tools: &[Box<dyn Tool>],
    context: Arc<ToolUseContext>,
    name: &str,
    input: Value,
) -> Result<MediaJob, ToolCallError> {
    let result = execute_tool_from_registry(tools, name, input, context).await?;
    Ok(serde_json::from_value(result.result.model_result).expect("structured media job"))
}

#[tokio::test]
async fn media_agent_tool_discovers_checks_permissions_and_submits_once() {
    let fixture = Fixture::new().await;
    let context = fixture.tool_context(true).await.unwrap();
    let context = Arc::new(
        (*context).clone().with_message_id("assistant-turn").with_tool_call_id("call-image"),
    );
    let tools = registry(&fixture, &context);
    let spec = |name| tools.iter().find(|tool| tool.name() == name).unwrap().spec();
    assert!(!spec(MEDIA_SUBMIT_TOOL_ID).read_only);
    assert!(spec(MEDIA_STATUS_TOOL_ID).read_only);
    assert_eq!(
        spec(MEDIA_SUBMIT_TOOL_ID).input_schema["properties"]["operation"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        7
    );

    assert!(fixture.tool_context(false).await.is_none());
    let unauthenticated = Arc::new(ToolUseContext::new(context.session(), None));
    let input = json!({"operation": "image_generate", "text": "draw"});
    let denied = call(&tools, unauthenticated, MEDIA_SUBMIT_TOOL_ID, input.clone()).await;
    assert!(matches!(denied, Err(ToolCallError::Denied { .. })));

    let readonly =
        Arc::new((*context).clone().with_security(Arc::new(agent_security::SecurityPolicy {
            autonomy: agent_security::AutonomyLevel::ReadOnly,
            ..Default::default()
        })));
    let denied = call(&tools, readonly, MEDIA_SUBMIT_TOOL_ID, input.clone()).await;
    assert!(denied.unwrap_err().message().contains("read-only"));

    let denied = call(
        &tools,
        context.clone(),
        MEDIA_SUBMIT_TOOL_ID,
        json!({
            "operation": "image_generate", "text": "draw",
            "deployment_id": fixture.model.deployment_id, "model_id": "not-entitled"
        }),
    )
    .await;
    assert!(matches!(denied, Err(ToolCallError::Denied { .. })));
    assert_eq!(fixture.image_posts(), 0);

    // A retry of the same runtime tool call has the same journal ID.
    let (first, second) = tokio::join!(
        call(&tools, context.clone(), MEDIA_SUBMIT_TOOL_ID, input.clone()),
        call(&tools, context.clone(), MEDIA_SUBMIT_TOOL_ID, input),
    );
    let first = first.unwrap();
    assert_eq!(first.id, second.unwrap().id);
    let expected = fixture.wait(first.id).await;
    let job =
        call(&tools, context, MEDIA_STATUS_TOOL_ID, json!({"job_id": first.id})).await.unwrap();
    assert_eq!(job.state, expected);
    let MediaJobState::Succeeded { result } = job.state else { panic!("expected an image") };
    let [asset] = result.assets.as_slice() else { panic!("expected one asset") };
    assert_eq!(asset.mime, "image/png");
    assert!(!asset.id.is_empty());
    assert_eq!(fixture.image_posts(), 1);
}

#[tokio::test]
async fn media_agent_tool_reads_desktop_job_without_submitting_again() {
    let fixture = Fixture::new().await;
    let request = fixture.submission(MediaOperation::ImageGenerate, "draw");
    assert_eq!(fixture.submit(&request).await.status(), reqwest::StatusCode::ACCEPTED);
    let expected = fixture.wait(request.request_id).await;
    assert!(matches!(expected, MediaJobState::Succeeded { .. }));

    let context = fixture.tool_context(true).await.unwrap();
    let tools = registry(&fixture, &context);
    let input = json!({"job_id": request.request_id});
    let result = call(&tools, context.clone(), MEDIA_STATUS_TOOL_ID, input.clone()).await.unwrap();
    assert_eq!(result.state, expected);

    let foreign =
        call(&tools, fixture.other_tool_context().await, MEDIA_STATUS_TOOL_ID, input.clone())
            .await
            .unwrap_err();
    assert_eq!(foreign.message(), "Media resource not found.");

    // Delegation to a different session drops the parent's gateway authority.
    let child = Arc::new(
        (*context).clone().for_child("different-session".into(), vec![MEDIA_STATUS_TOOL_ID.into()]),
    );
    let denied = call(&tools, child, MEDIA_STATUS_TOOL_ID, input).await;
    assert!(matches!(denied, Err(ToolCallError::Denied { .. })));
    assert_eq!(fixture.image_posts(), 1);
}
