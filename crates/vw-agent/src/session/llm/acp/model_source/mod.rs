//! Runtime-only CLI model launch boundary; local CLI configuration bypasses this module.
mod drivers;
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
    let identity = RelayIdentity {
        tenant: crate::session::ui_store::current_tenant_scope(),
        project: context.project.id.clone(),
        session: format!("{session}:{agent}"),
        provider: model.provider_id.clone(),
        model: model.id.clone(),
        generation: 0,
    };
    let mut profile = AcpLaunchProfile::new(config, identity)?;
    let server = RelayServer::start(
        profile.binding.clone(),
        std::sync::Arc::new(ProviderRelayBackend::new()?),
        context,
    )
    .await?;
    let endpoint = format!("http://{}", server.address());
    profile.attach_server(server);
    drivers::configure(agent, &mut profile, &endpoint)?;
    Ok(profile)
}

#[cfg(all(test, unix))]
#[path = "profile_tests.rs"]
mod tests;

#[cfg(test)]
mod acceptance_tests;
