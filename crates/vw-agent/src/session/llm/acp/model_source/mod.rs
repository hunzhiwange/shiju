//! Runtime-only CLI model launch boundary; local CLI configuration bypasses this module.
pub(super) mod drivers;
mod gateway;
mod mimo_mcp;
pub(crate) mod profile;
use crate::{
    config::schema::AcpAgentConfig,
    model_relay::{ProviderRelayBackend, RelayIdentity, RelayServer},
    project::instance,
    provider::provider,
};
use profile::AcpLaunchProfile;

#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use llm_runtime::acp::model_source::fingerprint;

pub(super) async fn prepare(
    agent: &str,
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
    openclaw_mcp: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<AcpLaunchProfile> {
    prepare_with_mounts(agent, config, model, session, openclaw_mcp, None).await
}

pub(super) async fn prepare_stdio_mcp(
    agent: &str,
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
    cwd: &std::path::Path,
    servers: &[acp_types::AcpMcpServer],
) -> anyhow::Result<AcpLaunchProfile> {
    prepare_with_mounts(agent, config, model, session, &Default::default(), Some((cwd, servers))).await
}

async fn prepare_with_mounts(
    agent: &str,
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
    openclaw_mcp: &serde_json::Map<String, serde_json::Value>,
    mounts: Option<(&std::path::Path, &[acp_types::AcpMcpServer])>,
) -> anyhow::Result<AcpLaunchProfile> {
    let project = instance::project()
        .ok_or_else(|| anyhow::anyhow!("ACP launch requires project context"))?;
    let context = instance::Context {
        directory: instance::directory(),
        worktree: instance::worktree(),
        project,
    };
    // Resolve the current catalog on every issue/reissue; persisted launch records
    // cannot supply a Provider snapshot or resurrect removed models.
    let resolved = provider::get_model(&model.provider_id, &model.id).await?;
    let identity = launch_identity(context.project.id.clone(), agent, model, session);
    let mut copilot_launch = None;
    if agent == "copilot" {
        use llm_runtime::acp::model_source::drivers::copilot::PROJECT_SKILLS_ENV;
        if let Some(grant) = config.env.get(PROJECT_SKILLS_ENV) {
            let tenant = crate::session::ui_store::current_tenant_scope();
            let tenant_config = crate::config::get_global_for_tenant_without_env(&tenant).await?;
            anyhow::ensure!(
                tenant_config.acp.get("copilot")
                    .and_then(|entry| entry.env.get(PROJECT_SKILLS_ENV)) == Some(grant),
                "Copilot native skills require a tenant-owned project grant"
            );
            let project = std::path::Path::new(&context.directory).canonicalize()?;
            if std::path::Path::new(grant) != project {
                let mut launch = config.clone();
                launch.env.remove(PROJECT_SKILLS_ENV);
                copilot_launch = Some(launch);
            }
        }
    }
    let config = copilot_launch.as_ref().unwrap_or(config);
    let state_root = || std::env::var_os("VIBEWINDOW_CONFIG_DIR").map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::global::paths().state.clone());
    let mut profile = llm_runtime::acp::model_source::prepare_profile(agent, config, &resolved, identity, state_root)?;
    let directory = match mounts {
        Some((cwd, _)) => cwd.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 MCP workspace"))?.to_owned(),
        None => context.directory.clone(),
    };
    if agent == "vtcode" && mounts.is_some() {
        acp_agent_registry::verify_vtcode_mcp_executable(&config.command).map_err(anyhow::Error::msg)?;
    }
    let projection = match mounts {
        Some((_, servers)) => Some(mimo_mcp::prepare(servers, &directory, profile.binding.clone()).await?),
        None => None,
    };
    let routes = projection.as_ref().map_or_else(axum::Router::new, |projection| projection.router.clone());
    let catalog = projection.as_ref().filter(|_| agent == "vtcode")
        .map(|projection| projection.tools.clone()).unwrap_or_default();
    let server = RelayServer::start_with_routes_and_vtcode_catalog(
        profile.binding.clone(),
        std::sync::Arc::new(ProviderRelayBackend::new()?),
        context,
        routes,
        catalog,
    )
    .await?;
    let endpoint = format!("http://{}", server.address());
    profile.attach_server(server);
    if let Some(projection) = projection.as_ref().filter(|_| matches!(agent, "vtcode" | "hermes" | "auggie" | "droid" | "autohand")) {
        llm_runtime::acp::model_source::configure_profile(agent, &mut profile, &endpoint, &directory, openclaw_mcp).await?;
        if agent == "auggie" {
            let tools = projection.tools.iter().map(|name| {
                name.strip_prefix("mcp__auggie__").ok_or_else(|| anyhow::anyhow!("Invalid Auggie MCP approval name"))
            }).collect::<anyhow::Result<Vec<_>>>()?;
            let flags = tools.iter().flat_map(|tool| ["--permission".to_owned(), format!("{tool}:ask-user")])
                .collect::<Vec<_>>();
            profile.append_arguments(&flags)?;
            profile.overlay_environment(std::collections::HashMap::from([
                ("SHIJU_AUGGIE_MCP_TOOLS".into(), serde_json::to_string(&tools)?),
            ]))?;
        }
        profile.managed_mcp = Some(projection.approval_config(&endpoint)?);
    } else if let Some(mimo) = projection {
        llm_runtime::acp::model_source::drivers::launch_route(agent, &profile)?;
        llm_runtime::acp::model_source::drivers::mimo_code::configure_with_mcp(
            &mut profile, &endpoint, &directory, mimo.native_config(&endpoint), &mimo.tools,
        )?;
        profile.overlay_environment(std::collections::HashMap::from([
            ("SHIJU_MIMO_MCP_TOKEN".to_owned(), mimo.token().to_owned()),
        ]))?;
    } else {
        let project_directory = instance::directory();
        let configuration_directory = if agent == "copilot" { &project_directory } else { &directory };
        llm_runtime::acp::model_source::configure_profile(agent, &mut profile, &endpoint, configuration_directory, openclaw_mcp).await?;
    }
    Ok(profile)
}

pub(super) fn launch_identity(
    project: String,
    agent: &str,
    model: &provider::Model,
    session: &str,
) -> RelayIdentity {
    llm_runtime::acp::model_source::launch_identity(
        crate::session::ui_store::current_tenant_scope(), project, agent, model, session,
    )
}

#[cfg(all(test, unix))]
#[path = "profile_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) mod acceptance_tests;
