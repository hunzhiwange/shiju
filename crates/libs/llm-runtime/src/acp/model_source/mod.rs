//! Request-owned ACP CLI launch configuration.
pub mod profile;
pub mod gateway;
pub mod drivers;

/// Process-keyed one-way digest: cache diagnostics cannot disclose or guess credentials.
pub fn fingerprint(bytes: &[u8]) -> String {
    static KEY: std::sync::LazyLock<ring::hmac::Key> = std::sync::LazyLock::new(|| {
        let mut key = [0; 32];
        getrandom::fill(&mut key).expect("ACP cache fingerprint entropy unavailable");
        ring::hmac::Key::new(ring::hmac::HMAC_SHA256, &key)
    });
    hex::encode(ring::hmac::sign(&KEY, bytes).as_ref())
}

pub fn launch_identity(
    tenant: vw_api_transport::TenantScope,
    project: String,
    agent: &str,
    model: &model_types::Model,
    session: &str,
) -> model_transport::RelayIdentity {
    model_transport::RelayIdentity {
        tenant,
        project,
        session: format!("{session}:{agent}"),
        provider: model.provider_id.clone(),
        model: model.id.clone(),
        generation: 0,
    }
}

/// Apply launch policy to the freshly resolved catalog model and host-owned identity.
pub fn prepare_profile(agent: &str, config: &agent_config::schema::AcpAgentConfig, resolved: &model_types::Model, identity: model_transport::RelayIdentity, state_root: impl FnOnce() -> std::path::PathBuf) -> anyhow::Result<profile::AcpLaunchProfile> {
    anyhow::ensure!(
        resolved.status == "active" && !crate::prompt::is_acp_model(resolved),
        "ACP launch requires an active non-ACP model"
    );
    // Claude verifies its native engine when spawning, so reused sessions do
    // not hash the same running executable on every turn.
    if agent != "claude" {
        acp_agent_registry::verify_reviewed_agent_executable(agent, &config.command)
            .map_err(anyhow::Error::msg)?;
    }
    let mut profile = profile::AcpLaunchProfile::new(config, identity)?;
    profile.state_root = Some(state_root());
    profile.apply_model(resolved);
    Ok(profile)
}

/// Finish the reviewed driver configuration after the host binds its actual relay.
pub async fn configure_profile(agent: &str, profile: &mut profile::AcpLaunchProfile, endpoint: &str, directory: &str) -> anyhow::Result<()> {
    drivers::configure(agent, profile, endpoint, directory)?;
    if let Some(gateway) = profile.gateway.as_mut() { gateway.start().await?; }
    Ok(())
}
