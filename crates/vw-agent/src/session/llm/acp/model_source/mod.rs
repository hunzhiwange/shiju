//! Runtime-only CLI model launch boundary; local CLI configuration bypasses this module.
pub(super) mod drivers;
mod gateway;
pub(crate) mod profile;
use crate::{
    config::schema::AcpAgentConfig,
    model_relay::{ProviderRelayBackend, RelayIdentity, RelayServer},
    project::instance,
    provider::provider,
};
use profile::AcpLaunchProfile;

/// Process-keyed one-way digest: cache diagnostics cannot disclose or guess credentials.
pub(super) fn fingerprint(bytes: &[u8]) -> String {
    static KEY: std::sync::LazyLock<ring::hmac::Key> = std::sync::LazyLock::new(|| {
        let mut key = [0; 32];
        getrandom::fill(&mut key).expect("ACP cache fingerprint entropy unavailable");
        ring::hmac::Key::new(ring::hmac::HMAC_SHA256, &key)
    });
    hex::encode(ring::hmac::sign(&KEY, bytes).as_ref())
}

pub(super) async fn prepare(
    agent: &str,
    config: &AcpAgentConfig,
    model: &provider::Model,
    session: &str,
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
    anyhow::ensure!(
        resolved.status == "active" && !super::super::prompt::is_acp_model(&resolved),
        "ACP launch requires an active non-ACP model"
    );
    // Catalog rejection must remain authoritative even when the runtime is absent.
    vw_acp::verify_reviewed_agent_executable(agent, &config.command).map_err(anyhow::Error::msg)?;
    let identity = launch_identity(context.project.id.clone(), agent, model, session);
    let mut profile = AcpLaunchProfile::new(config, identity)?;
    profile.state_root = Some(
        std::env::var_os("VIBEWINDOW_CONFIG_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| crate::global::paths().state.clone()),
    );
    profile.model_output_limit = resolved.limit.output;
    profile.model_context_limit = resolved.limit.context;
    profile.model_supports_temperature = resolved.capabilities.temperature;
    profile.model_supports_reasoning = resolved.capabilities.reasoning;
    profile.model_reasoning_options = resolved.capabilities.reasoning_options.clone();
    profile.model_supports_tool_call = resolved.capabilities.toolcall;
    profile.model_api = Some(resolved.api.clone());
    let server = RelayServer::start(
        profile.binding.clone(),
        std::sync::Arc::new(ProviderRelayBackend::new()?),
        context,
    )
    .await?;
    let endpoint = format!("http://{}", server.address());
    profile.attach_server(server);
    drivers::configure(agent, &mut profile, &endpoint)?;
    if let Some(gateway) = profile.gateway.as_mut() {
        gateway.start().await?;
    }
    Ok(profile)
}

pub(super) fn launch_identity(
    project: String,
    agent: &str,
    model: &provider::Model,
    session: &str,
) -> RelayIdentity {
    RelayIdentity {
        tenant: crate::session::ui_store::current_tenant_scope(),
        project,
        session: format!("{session}:{agent}"),
        provider: model.provider_id.clone(),
        model: model.id.clone(),
        generation: 0,
    }
}

#[cfg(all(test, unix))]
#[path = "profile_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) mod acceptance_tests;
