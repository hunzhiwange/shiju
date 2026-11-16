//! Real application tool invocation after the domain-owned signed fixture install.
use crate::app::agent::tools::{SkillTool, Tool, ToolUseContext, scope_tool_use_context};
use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;
#[tokio::test]
async fn expert_package_install_succeeds_without_connector_credentials() {
    let project = tempfile::tempdir().expect("fixture project");
    let (status, body, calls) = gateway_skill_market::expert_fixture::install_expert_fixture(project.path(), Arc::new(super::RuntimeSkillMarketConfig)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let root = project.path().join(".vibewindow/skills");
    assert!(root.join("fixture--skillset--fixture-suite/SKILL.md").is_file());
    assert!(root.join("fixture--public--first-skill/SKILL.md").is_file());
    let main_id = "fixture--skillset--fixture-suite";
    let config = Arc::new(crate::config::Config::default());
    let loaded = crate::app::agent::skills::load_skills_full_with_config(project.path(), &config);
    assert!(loaded.iter().any(|skill| skill.name == main_id));
    let tool = SkillTool::new_with_runtime_config(
        Arc::new(crate::app::agent::security::SecurityPolicy::default()),
        "expert-install-test".to_string(),
        false,
        project.path().to_path_buf(),
        config,
    );
    let context = Arc::new(
        ToolUseContext::new("expert-install-test", Some(project.path().display().to_string()))
            .with_explicitly_requested_skills([main_id]),
    );
    let invocation = scope_tool_use_context(context, tool.call(json!({"skill": main_id})))
        .await
        .expect("skill call");
    assert_eq!(invocation.data["success"], true, "{invocation:?}");
    assert_eq!(invocation.context_updates[0].key, "skill.name");
    assert!(
        invocation.extra_messages[0]
            .content
            .as_str()
            .is_some_and(|body| body.contains("# Workflow"))
    );
    assert_eq!(
        calls.as_slice(),
        ["skill_set_artifact:fixture:fixture-suite"]
    );
}
