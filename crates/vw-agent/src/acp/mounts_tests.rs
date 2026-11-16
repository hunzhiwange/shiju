#[tokio::test]
async fn acp_server_skill_overlay_accepts_native_directory() {
    agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_accepts_native_directory::<super::backend::NativeAcpHost>().await;
}

#[tokio::test]
async fn acp_server_skill_overlay_invokes_reads_attachment_and_rediscovers_without_full_access() {
    agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_invokes_reads_attachment_and_rediscovers_without_full_access::<super::backend::NativeAcpHost, crate::tools::context::AgentContextHost>(
        |id, root, sources| {
            let context = crate::tools::ToolUseContext::new(id, Some(root.to_str().unwrap().into()));
            std::sync::Arc::new(match sources {
                Some(sources) => context.with_session_skills(sources).with_explicitly_requested_skills(["overlay_probe"]),
                None => context,
            })
        },
        |id, root, context, bind| {
            let security = if bind { crate::security::SecurityPolicy { workspace_dir: root.into(), workspace_only: true, ..Default::default() } } else { Default::default() };
            let tool = crate::tools::skill::SkillTool::new_with_runtime_config(std::sync::Arc::new(security), id, false, root.into(), std::sync::Arc::new(crate::config::Config::default()));
            std::sync::Arc::new(if bind { tool.with_skill_directory_context(context) } else { tool })
        },
        |root| std::sync::Arc::new(crate::tools::file_read::FileReadTool::new(std::sync::Arc::new(crate::security::SecurityPolicy { workspace_dir: root.into(), workspace_only: true, ..Default::default() }))),
        |root, sources| crate::skills::load_session_skills(root, &crate::config::Config::default(), sources, false).map_err(|error| error.to_string()),
    ).await;
}

#[tokio::test]
async fn acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing() {
    agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_rejects_conflicts_and_failed_mcp_without_publishing::<super::backend::NativeAcpHost>().await;
}

#[tokio::test]
async fn acp_server_skill_overlay_and_real_mcp_execute_in_one_prepared_session() {
    agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_and_real_mcp_execute_in_one_prepared_session::<super::backend::NativeAcpHost, crate::tools::context::AgentContextHost>(native_overlay_context, native_overlay_skill).await;
}

#[cfg(unix)]
#[tokio::test]
async fn acp_server_skill_overlay_permission_change_revokes_read_and_invocation() {
    agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_permission_change_revokes_read_and_invocation::<super::backend::NativeAcpHost>(|root, sources| crate::skills::load_session_skills(root, &crate::config::Config::default(), sources, true).map_err(|error| error.to_string())).await;
}

#[tokio::test]
async fn acp_server_skill_overlay_retains_native_skill_permission_denial() {
    agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_retains_native_skill_permission_denial::<super::backend::NativeAcpHost, crate::tools::context::AgentContextHost>(native_overlay_context, native_overlay_skill).await;
}

#[tokio::test]
async fn acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears() {
    agent_acp::host_migration_53_overlay_contracts::acp_server_skill_overlay_same_name_sources_are_isolated_and_explicit_empty_clears::<super::backend::NativeAcpHost>(|root, sources| crate::skills::load_session_skills(root, &crate::config::Config::default(), sources, true).map_err(|error| error.to_string())).await;
}

fn native_overlay_context(id: &str, root: &std::path::Path, sources: super::mounts::SessionSkillSources, deny: bool) -> std::sync::Arc<crate::tools::ToolUseContext> {
    let context = crate::tools::ToolUseContext::new(id, Some(root.to_str().unwrap().into())).with_session_skills(sources);
    std::sync::Arc::new(if deny {
        context.with_delegated_skill_rules(vec![permission_types::Rule { permission: "skill".into(), pattern: "overlay_probe".into(), action: permission_types::Action::Deny }])
    } else {
        context.with_explicitly_requested_skills(["overlay_probe"])
    })
}
fn native_overlay_skill(id: String, root: &std::path::Path) -> std::sync::Arc<dyn crate::tools::Tool> {
    std::sync::Arc::new(crate::tools::skill::SkillTool::new_with_runtime_config(
        std::sync::Arc::new(crate::security::SecurityPolicy::default()), id, false, root.into(), std::sync::Arc::new(crate::config::Config::default()),
    ))
}
