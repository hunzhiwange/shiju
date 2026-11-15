use super::AcpLaunchProfile;
use serde_json::json;
use std::collections::HashMap;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
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
        "Qoder requires a loopback relay endpoint"
    );
    let config = profile.config();
    anyhow::ensure!(config.args == ["--acp"], "Qoder requires verified ACP arguments");
    anyhow::ensure!(
        !config.env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("QODER_")
                || key.starts_with("QODERCN_")
                || key.starts_with("SHIJU_QODER_")
                || matches!(
                    key.as_str(),
                    "NODE_OPTIONS" | "NODE_PATH" | "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY"
                )
        }),
        "conflicting Qoder model or authentication environment"
    );
    vw_acp::verify_reviewed_agent_executable("qoder", &config.command)
        .map_err(anyhow::Error::msg)?;
    anyhow::ensure!(profile.model_supports_tool_call, "Qoder requires model tool support");
    let model = profile.model_alias.clone();
    let provider = json!({"protocol": "openai", "baseUrl": format!("{endpoint}/v1"),
        "apiKey": profile.binding.bearer_token(), "model": model,
        "models": [{"model": model, "maxOutputTokens": profile.model_output_limit,
            "contextWindow": profile.model_context_limit, "capabilities": {"tools": true, "vision": false}}]});
    profile.write_config(
        "settings.json",
        &serde_json::to_vec(&json!({
            "model": {"name": format!("shiju/{model}")},
            "general": {"enableAutoUpdate": false, "maxAttempts": 1},
            "providers": {"shiju": provider}
        }))?,
    )?;
    profile.write_config(
        "model.json",
        &serde_json::to_vec(&json!({
            "model": model, "endpoint": endpoint, "token": profile.binding.bearer_token(),
            "outputLimit": profile.model_output_limit, "contextLimit": profile.model_context_limit,
        }))?,
    )?;
    let directory = profile.directory().to_string_lossy().into_owned();
    profile.overlay_environment(HashMap::from([(
        "SHIJU_QODER_MODEL_CONFIG".into(),
        profile.directory().join("model.json").to_string_lossy().into_owned(),
    )]))?;
    profile.append_arguments(&[
        "--config-dir".into(),
        directory,
        "--setting-sources".into(),
        "user".into(),
        "--model".into(),
        format!("shiju/{model}"),
    ])?;
    Ok(())
}

#[cfg(test)]
#[path = "qoder_tests.rs"]
mod tests;
