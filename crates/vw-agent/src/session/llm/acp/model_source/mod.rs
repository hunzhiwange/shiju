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

pub(super) use llm_runtime::acp::model_source::fingerprint;

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
    let identity = launch_identity(context.project.id.clone(), agent, model, session);
    let state_root = || std::env::var_os("VIBEWINDOW_CONFIG_DIR").map(std::path::PathBuf::from)
        .unwrap_or_else(|| crate::global::paths().state.clone());
    let mut profile = llm_runtime::acp::model_source::prepare_profile(agent, config, &resolved, identity, state_root)?;
    let directory = context.directory.clone();
    let server = RelayServer::start(
        profile.binding.clone(),
        std::sync::Arc::new(ProviderRelayBackend::new()?),
        context,
    )
    .await?;
    let endpoint = format!("http://{}", server.address());
    profile.attach_server(server);
    llm_runtime::acp::model_source::configure_profile(agent, &mut profile, &endpoint, &directory).await?;
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
