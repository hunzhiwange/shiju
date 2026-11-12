use super::AcpLaunchProfile;
use std::collections::HashMap;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["acp", "serve"],
        "TRAE requires verified ACP arguments"
    );
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("TRAE_")
                || key.starts_with("VW_TRAE_")
                || key.starts_with("SHIJU_TRAE_")
                || key.starts_with("COCO_")
                || key.starts_with("OTEL_")
                || matches!(key.as_str(), "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY")
        }),
        "conflicting TRAE model or authentication environment"
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
        "TRAE requires an owned loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && profile.model_supports_temperature,
        "TRAE requires a tool-capable Chat model supporting temperature"
    );
    vw_acp::verify_reviewed_agent_executable("trae", &profile.config().command)
        .map_err(anyhow::Error::msg)?;
    let path = profile.write_config(
        "binding.json",
        &serde_json::to_vec(&serde_json::json!({
            "reference": profile.config().env[super::super::profile::REFERENCE_ENV],
            "model": profile.model_alias, "endpoint": endpoint,
            "output_limit": profile.model_output_limit.max(1),
            "context_limit": profile.model_context_limit.max(1),
        }))?,
    )?;
    let mut environment = HashMap::from([
        ("SHIJU_TRAE_MODEL_CONFIG".into(), path.to_string_lossy().into_owned()),
        ("VW_TRAE_RELAY_TOKEN".into(), profile.binding.bearer_token().to_owned()),
    ]);
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"]
    {
        environment.insert(key.into(), String::new());
    }
    profile.overlay_environment(environment)?;
    profile.session_model_id = Some("session-selected".into());
    Ok(())
}

#[cfg(test)]
#[path = "trae_tests.rs"]
mod tests;
