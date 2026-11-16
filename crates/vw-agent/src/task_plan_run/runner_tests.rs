//! 真实网关授权与消息ID接线。
use super::SessionRunRequest;

#[tokio::test]
async fn acp_background_approval_plan_inherits_current_workspace_grant() {
    use crate::session::agent_binding::{AgentBinding, resolve_full_access_scoped, save_scoped};
    let _env = crate::test_support::global_async_env_lock().lock().await;
    let _fixture = crate::session::llm::recovery_test_support::RecoveryConfig::new(
        "https://example.test/v1",
        None,
    )
    .await;
    let tenant = crate::session::ui_store::current_tenant_scope();
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().to_str().unwrap();
    let parent = format!("parent-{}", uuid::Uuid::new_v4());
    super::session_bind::ensure_plan_run_session_info(&parent, None, root).await.unwrap();
    let (project, _) = crate::project::from_directory(&tenant, root).await.unwrap();
    let definition = config_agent::AgentDefinitionConfig {
        runtime: config_agent_profile::AgentRuntime::Acp { backend: "codex".into() },
        ..Default::default()
    };
    let binding = AgentBinding::from_definition("acp-codex".into(), &definition);
    save_scoped(&project.id, &parent, &binding).await.unwrap();
    let runner = super::GatewaySessionRunner::new(tenant, Some(parent.clone()));
    let mut request = SessionRunRequest {
        session_id: "child".into(),
        query: "q".into(),
        root: root.into(),
        model: None,
        agent: Some("acp-codex".into()),
        acp_agent: Some("codex".into()),
    };
    assert!(!runner.acp_full_access(&request).await.unwrap());
    resolve_full_access_scoped(&project.id, &parent, Some(true), false).await.unwrap();
    assert!(runner.acp_full_access(&request).await.unwrap());

    request.acp_agent = Some("claude".into());
    assert!(!runner.acp_full_access(&request).await.unwrap());
    request.acp_agent = Some("codex".into());
    let other = tempfile::tempdir().unwrap();
    request.root = other.path().to_string_lossy().into_owned();
    assert!(!runner.acp_full_access(&request).await.unwrap());

    request.root = root.into();
    resolve_full_access_scoped(&project.id, &parent, Some(false), false).await.unwrap();
    assert!(!runner.acp_full_access(&request).await.unwrap());
}



#[test]
fn gateway_plan_run_preallocates_assistant_message_id() {
    let first =
        super::runner::preallocate_assistant_message_id().expect("first assistant message id");
    let second =
        super::runner::preallocate_assistant_message_id().expect("second assistant message id");

    assert!(!first.trim().is_empty());
    assert!(!second.trim().is_empty());
    assert_ne!(first, second);
}
