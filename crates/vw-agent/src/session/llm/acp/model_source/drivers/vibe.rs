use super::AcpLaunchProfile;

/// Requires the reviewed Mistral Vibe 3.0.4 session-binding patch. The explicit
/// flag makes an unpatched binary reject startup instead of ignoring isolation.
pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Vibe requires selected model transport metadata"))?;
    let style = match api.adapter.as_str() {
        "openai-responses" => "openai-responses",
        "openai" | "openai-compatible" => {
            let upstream = reqwest::Url::parse(&api.url)
                .map_err(|_| anyhow::anyhow!("Vibe selected model endpoint is invalid"))?;
            if upstream.path().trim_end_matches('/').ends_with("/responses") {
                "openai-responses"
            } else {
                "openai"
            }
        }
        "anthropic" => "anthropic",
        // Vibe has no native Gemini adapter. The existing Chat relay performs
        // the supported low-level conversion using the selected Provider.
        "google" => "openai",
        _ => anyhow::bail!("Vibe selected model protocol is unsupported"),
    };
    anyhow::ensure!(
        profile.model_supports_temperature,
        "Vibe requires a model supporting temperature"
    );
    anyhow::ensure!(
        profile.config().args.is_empty(),
        "Vibe session model rejects native launch flags"
    );
    let relay = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("Vibe relay endpoint is invalid"))?;
    anyhow::ensure!(
        relay.scheme() == "http"
            && relay.host_str() == Some("127.0.0.1")
            && relay.port().is_some()
            && relay.username().is_empty()
            && relay.password().is_none()
            && relay.path() == "/"
            && relay.query().is_none()
            && relay.fragment().is_none(),
        "Vibe requires a loopback relay root"
    );
    let model = serde_json::json!({
        "name": profile.model_alias,
        "alias": "session-selected",
        "provider": "session-provider",
        "supports_images": false,
        "max_context_length": if profile.model_context_limit == 0 {
            vw_shared::provider::state::DEFAULT_MODEL_CONTEXT_LIMIT
        } else {
            profile.model_context_limit
        },
        "max_output_tokens": if profile.model_output_limit == 0 {
            vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
        } else {
            profile.model_output_limit
        },
    });
    let settings = serde_json::json!({
        "active_model": "session-selected",
        "allowed_models": [profile.model_alias],
        "providers": [{
            "name": "session-provider",
            "api_base": if style == "anthropic" { endpoint.to_owned() } else { format!("{endpoint}/v1") },
            "api_key_env_var": "VW_VIBE_RELAY_TOKEN",
            "api_style": style,
            "backend": "generic",
        }],
        "models": [model.clone()],
        "compaction_model": model,
        "utility_models": {"title": "session-selected", "smart_approve": "session-selected"},
    });
    let bytes = toml::to_string(&settings)?.into_bytes();
    profile.overlay_environment(std::collections::HashMap::from([
        ("VIBE_HOME".into(), profile.directory().to_string_lossy().into_owned()),
        ("VW_VIBE_RELAY_TOKEN".into(), profile.binding.bearer_token().to_owned()),
    ]))?;
    let path = profile.write_config("session-model.toml", &bytes)?;
    profile.append_arguments(&[
        "--session-model-config".into(),
        path.to_string_lossy().into_owned(),
    ])?;
    Ok(())
}

#[cfg(test)]
#[path = "vibe_tests.rs"]
mod tests;
