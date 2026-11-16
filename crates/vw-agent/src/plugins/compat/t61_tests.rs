#[tokio::test]
async fn codex_plugin_compat_t61_public_skill_lifecycle_and_upgrade() {
    agent_plugins::package_acceptance::skill::lifecycle(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/plugin_compat/t61"),
        |config, name| async move {
            use crate::tools::{Tool, ToolUseContext, scope_tool_use_context, skill::SkillTool};
            use std::sync::Arc;
            let config = Arc::new(config);
            let tool = SkillTool::new_with_runtime_config(
                Arc::new(crate::security::SecurityPolicy::default()), "t61-public".into(),
                false, config.workspace_dir.clone(), config,
            );
            let context = Arc::new(ToolUseContext::new("t61-public", None).with_full_access_enabled(true));
            scope_tool_use_context(context, tool.call(serde_json::json!({"skill":name}))).await
        },
        &crate::skills::load_skills_full_with_config,
        &super::mcp_connection_sessions::close_plugin,
    ).await;
}
