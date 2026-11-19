//! Request-owned ACP CLI launch configuration.
pub mod profile;
pub mod gateway;
pub mod drivers;
#[cfg(windows)]
mod windows_private_dir;

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

fn verify_glm_launch_host() -> anyhow::Result<()> {
    anyhow::ensure!(
        cfg!(any(target_os = "linux", target_os = "macos", target_os = "windows")),
        "GLM managed launch is unavailable on this platform"
    );
    #[cfg(windows)]
    anyhow::ensure!(
        std::env::var_os("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .is_some_and(|path| path.is_absolute() && path.is_dir()),
        "GLM managed launch requires a Windows local application data directory"
    );
    // Match the ACP client's Node resolution for the reviewed JavaScript entry.
    let node_name = if cfg!(windows) { "node.exe" } else { "node" };
    anyhow::ensure!(
        shell_command::resolve_executable(node_name).is_some_and(|path| path.is_absolute()),
        "Managed GLM requires Node.js"
    );
    Ok(())
}

/// Apply launch policy to the freshly resolved catalog model and host-owned identity.
pub fn prepare_profile(agent: &str, config: &agent_config::schema::AcpAgentConfig, resolved: &model_types::Model, identity: model_transport::RelayIdentity, state_root: impl FnOnce() -> std::path::PathBuf) -> anyhow::Result<profile::AcpLaunchProfile> {
    anyhow::ensure!(
        resolved.status == "active" && !crate::prompt::is_acp_model(resolved),
        "ACP launch requires an active non-ACP model"
    );
    // Claude verifies its native engine when spawning, so reused sessions do
    // not hash the same running executable on every turn.
    // DimCode and Dirac keep their original launchers for sessions with no MCP mount.
    // The ACP client requires their reviewed executables for a mount.
    if !matches!(agent, "claude" | "dimcode" | "dirac") {
        acp_agent_registry::verify_reviewed_agent_executable(agent, &config.command)
            .map_err(anyhow::Error::msg)?;
    }
    if agent == "glm-acp-agent" {
        verify_glm_launch_host()?;
        anyhow::ensure!(config.args.is_empty(), "Unsupported managed GLM launch arguments");
    }
    let mut profile = profile::AcpLaunchProfile::new_for_agent(agent, config, identity)?;
    profile.state_root = Some(state_root());
    profile.apply_model(resolved);
    Ok(profile)
}

/// Finish the reviewed driver configuration after the host binds its actual relay.
pub async fn configure_profile(
    agent: &str,
    profile: &mut profile::AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
    openclaw_mcp: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<()> {
    if agent == "openclaw" && !openclaw_mcp.is_empty() {
        drivers::launch_route(agent, profile)?;
        drivers::openclaw::configure_with_mcp(profile, endpoint, directory, openclaw_mcp)?;
    } else {
        drivers::configure(agent, profile, endpoint, directory)?;
    }
    if let Some(gateway) = profile.gateway.as_mut() { gateway.start().await?; }
    Ok(())
}

#[cfg(test)]
mod glm_capability_tests;
