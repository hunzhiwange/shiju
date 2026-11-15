//! Factory Droid 1.9.0 native ACP with isolated Airgap/BYOK settings.
use super::AcpLaunchProfile;
use serde_json::json;
use std::collections::HashMap;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["exec", "--output-format", "acp"],
        "Droid requires the reviewed native ACP entry without overrides"
    );
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("FACTORY_")
                || matches!(
                    key.as_str(),
                    "NODE_OPTIONS" | "BUN_OPTIONS" | "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY"
                )
        }),
        "conflicting Droid model environment"
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
        "Droid requires an owned loopback relay"
    );
    vw_acp::verify_reviewed_agent_executable("droid", &profile.config().command)
        .map_err(anyhow::Error::msg)?;
    // Airgap disables Factory authentication, managed inference, updates and telemetry.
    // Runtime settings take precedence over native user/project model settings.
    let model_id = "custom:Session-Selected-0";
    let settings = json!({
        "customModels": [{
            "model": profile.model_alias, "displayName": "Session Selected",
            "baseUrl": format!("{}/v1", endpoint.trim_end_matches('/')),
            "apiKey": profile.binding.bearer_token(),
            "provider": "generic-chat-completion-api",
            "maxOutputTokens": if profile.model_output_limit == 0 {
                vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
            } else { profile.model_output_limit }
        }],
        "model": model_id, "specModel": model_id,
        "hooksDisabled": true, "cloudSessionSync": false,
        "ideAutoConnect": false
    });
    let path = profile.write_config("settings.json", &serde_json::to_vec(&settings)?)?;
    let mut env = HashMap::from([
        ("FACTORY_HOME_OVERRIDE".into(), profile.directory().to_string_lossy().into_owned()),
        ("FACTORY_AIRGAP_ENABLED".into(), "true".into()),
        ("FACTORY_DISABLE_KEYRING".into(), "true".into()),
        ("FACTORY_DISABLE_DYNAMIC_CONFIG".into(), "true".into()),
    ]);
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"]
    {
        env.insert(key.into(), String::new());
    }
    env.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    env.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    profile.overlay_environment(env)?;
    profile.append_arguments(&[
        "--settings".into(),
        path.to_string_lossy().into_owned(),
        "--model".into(),
        model_id.into(),
        "--spec-model".into(),
        model_id.into(),
    ])?;
    profile.session_model_id = Some(model_id.into());
    Ok(())
}

#[cfg(test)]
#[path = "droid_tests.rs"]
mod tests;
