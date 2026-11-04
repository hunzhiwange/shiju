use super::AcpLaunchProfile;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Snow requires selected model transport metadata"))?;
    let request_method = match api.adapter.as_str() {
        "openai-responses" => "responses",
        "openai" | "openai-compatible" => {
            let url = reqwest::Url::parse(&api.url)
                .map_err(|_| anyhow::anyhow!("Snow selected model endpoint is invalid"))?;
            if url.path().trim_end_matches('/').ends_with("/responses") {
                "responses"
            } else {
                "chat"
            }
        }
        "anthropic" => "anthropic",
        "google" => "gemini",
        _ => anyhow::bail!("Snow selected model protocol is unsupported"),
    };
    if request_method == "chat" {
        anyhow::ensure!(
            profile.model_supports_temperature,
            "Snow Chat requires a model supporting temperature"
        );
    }
    let base_path = if request_method == "gemini" { "/v1beta" } else { "/v1" };
    let config = serde_json::json!({"snowcfg": {
        "baseUrl": format!("{endpoint}{base_path}"),
        "baseUrlMode": "base",
        "apiKey": profile.binding.bearer_token(),
        "requestMethod": request_method,
        "advancedModel": profile.model_alias,
        "basicModel": profile.model_alias,
        "maxTokens": if profile.model_output_limit == 0 {
            vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
        } else {
            profile.model_output_limit
        },
        "supportsVision": true,
        "enablePromptOptimization": false,
        "maxRetries": 0
    }});
    profile.overlay_environment(std::collections::HashMap::from([(
        "SNOW_CONFIG_DIR".into(),
        profile.directory().to_string_lossy().into_owned(),
    )]))?;
    profile.write_config("config.json", &serde_json::to_vec(&config)?)?;
    Ok(())
}

#[cfg(test)]
#[path = "snow_tests.rs"]
mod tests;
