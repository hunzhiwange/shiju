use super::AcpLaunchProfile;
use std::collections::HashMap;

pub fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !cfg!(target_os = "linux"),
        "Poolside managed model source requires Linux isolation for native skill discovery and stdio MCP"
    );
    let url = reqwest::Url::parse(endpoint)?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.path() == "/"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Poolside requires a private loopback relay"
    );
    let config = profile.config();
    anyhow::ensure!(config.args == ["acp"], "Poolside requires reviewed ACP arguments");
    anyhow::ensure!(
        config.env.keys().all(|key| key == super::super::profile::REFERENCE_ENV),
        "Poolside custom launch environment is not verified"
    );
    acp_agent_registry::verify_reviewed_agent_executable("poolside", &config.command)
        .map_err(anyhow::Error::msg)?;
    let model = profile.model_alias.clone();
    anyhow::ensure!(
        !model.is_empty() && profile.model_supports_tool_call,
        "Poolside requires a selected tool-capable model"
    );
    profile.overlay_environment(HashMap::from([
        ("SHIJU_POOLSIDE_RELAY_URL".into(), endpoint.trim_end_matches('/').into()),
        ("SHIJU_POOLSIDE_RELAY_MODEL".into(), model.clone()),
        ("SHIJU_POOLSIDE_RELAY_TOKEN".into(), profile.binding.bearer_token().into()),
    ]))?;
    profile.session_model_id = Some(model);
    Ok(())
}
