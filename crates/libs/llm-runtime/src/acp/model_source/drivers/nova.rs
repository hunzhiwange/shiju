use super::AcpLaunchProfile;
use std::collections::HashMap;

pub fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
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
        "Nova requires a private loopback relay"
    );
    anyhow::ensure!(profile.config().args == ["acp"], "Nova requires reviewed ACP arguments");
    acp_agent_registry::verify_reviewed_agent_executable("nova", &profile.config().command)
        .map_err(anyhow::Error::msg)?;
    anyhow::ensure!(profile.model_supports_tool_call, "Nova requires model tool support");
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("COMPASS_")
                || key.starts_with("NOVA_")
                || key.starts_with("ANTHROPIC_")
                || key.starts_with("SHIJU_NOVA_")
                || matches!(key.as_str(), "NODE_OPTIONS" | "NODE_PATH")
        }),
        "conflicting Nova model environment"
    );
    let config = serde_json::to_vec(&serde_json::json!({
        "model": profile.model_alias, "endpoint": endpoint,
        "outputLimit": profile.model_output_limit,
    }))?;
    let path = profile.directory().join("nova-model.json");
    profile.overlay_environment(HashMap::from([
        ("SHIJU_NOVA_MODEL_CONFIG".into(), path.to_string_lossy().into_owned()),
        ("SHIJU_NOVA_RELAY_TOKEN".into(), profile.binding.bearer_token().into()),
        ("ANTHROPIC_AUTH_TOKEN".into(), profile.binding.bearer_token().into()),
        ("ANTHROPIC_API_KEY".into(), profile.binding.bearer_token().into()),
        ("COMPASS_MODEL".into(), profile.model_alias.clone()),
        ("NODE_OPTIONS".into(), "--no-addons".into()),
        ("COMPASS_TELEMETRY".into(), "false".into()),
        ("COMPASS_ERROR_REPORTING".into(), "false".into()),
        ("DO_NOT_TRACK".into(), "1".into()),
    ]))?;
    profile.write_config("nova-model.json", &config)?;
    profile.session_model_id = Some(format!("anthropic/{}", profile.model_alias));
    Ok(())
}
