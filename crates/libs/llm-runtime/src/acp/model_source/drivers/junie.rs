//! Bind the stock Junie agent to the request-owned OpenAI Chat relay.
use super::AcpLaunchProfile;
use std::collections::HashMap;

pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    directory: &str,
) -> anyhow::Result<()> {
    let config = profile.config();
    anyhow::ensure!(config.args == ["--acp=true"], "Junie requires its reviewed ACP arguments");
    anyhow::ensure!(
        config.env.keys().all(|key| key == super::super::profile::REFERENCE_ENV),
        "Junie does not accept custom launch environment"
    );
    acp_agent_registry::verify_reviewed_agent_executable("junie", &config.command)
        .map_err(anyhow::Error::msg)?;
    anyhow::ensure!(profile.model_supports_tool_call, "Junie requires model tool support");

    let content = serde_json::to_vec(&serde_json::json!({
        "reference": config.env[super::super::profile::REFERENCE_ENV],
        "model": profile.model_alias,
        "endpoint": endpoint,
        "token": profile.binding.bearer_token(),
        "context": profile.model_context_limit,
        "workspace": std::fs::canonicalize(directory)?,
    }))?;
    let path = profile.write_config("junie.json", &content)?;
    profile.overlay_environment(HashMap::from([(
        "SHIJU_JUNIE_CONFIG".into(),
        path.to_string_lossy().into_owned(),
    )]))?;
    profile.session_model_id = Some("v1:6:custom:custom:shiju".into());
    Ok(())
}
