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

pub(super) async fn prepare_mimo_mcp(
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
    cwd: &std::path::Path,
    servers: &[acp_types::AcpMcpServer],
) -> anyhow::Result<AcpLaunchProfile> {
    prepare_with_mounts("mimo-code", config, model, session, &Default::default(), Some((cwd, servers))).await
}

async fn prepare_with_mounts(
    agent: &str,
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
    openclaw_mcp: &serde_json::Map<String, serde_json::Value>,
    mimo_mounts: Option<(&std::path::Path, &[acp_types::AcpMcpServer])>,
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
    let state_root = || std::env::var_os("VIBEWINDOW_CONFIG_DIR").map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::global::paths().state.clone());
    let mut profile = llm_runtime::acp::model_source::prepare_profile(agent, config, &resolved, identity, state_root)?;
    let directory = match mimo_mounts {
        Some((cwd, _)) => cwd.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 MiMo workspace"))?.to_owned(),
        None => context.directory.clone(),
    };
    let mimo = match mimo_mounts {
        Some((_, servers)) => Some(mimo_mcp::prepare(servers, &directory, profile.binding.clone()).await?),
        None => None,
    };
    let routes = mimo.as_ref().map_or_else(axum::Router::new, |projection| projection.router.clone());
    let server = RelayServer::start_with_routes(
        profile.binding.clone(),
        std::sync::Arc::new(ProviderRelayBackend::new()?),
        context,
        routes,
    )
    .await?;
    let endpoint = format!("http://{}", server.address());
    profile.attach_server(server);
    if let Some(mimo) = mimo {
        llm_runtime::acp::model_source::drivers::launch_route(agent, &profile)?;
        llm_runtime::acp::model_source::drivers::mimo_code::configure_with_mcp(
            &mut profile, &endpoint, &directory, mimo.native_config(&endpoint), &mimo.tools,
        )?;
        profile.overlay_environment(std::collections::HashMap::from([
            ("SHIJU_MIMO_MCP_TOKEN".to_owned(), mimo.token().to_owned()),
        ]))?;
    } else {
        llm_runtime::acp::model_source::configure_profile(agent, &mut profile, &endpoint, &directory, openclaw_mcp).await?;
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
