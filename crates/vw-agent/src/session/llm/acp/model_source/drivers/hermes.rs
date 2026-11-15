use super::AcpLaunchProfile;

// Built-in auxiliary consumers in the pinned hermes-agent 0.19.0 release.
const AUXILIARY_TASKS: &[&str] = &[
    "vision",
    "web_extract",
    "compression",
    "skills_hub",
    "approval",
    "mcp",
    "title_generation",
    "memory_query_rewrite",
    "tts_audio_tags",
    "triage_specifier",
    "kanban_decomposer",
    "profile_describer",
    "goal_judge",
    "curator",
    "monitor",
    "background_review",
    "moa_reference",
    "moa_aggregator",
];

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["acp"],
        "Hermes session model requires plain acp launch"
    );
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Hermes requires selected model transport metadata"))?;
    anyhow::ensure!(
        matches!(
            api.adapter.as_str(),
            "openai" | "openai-compatible" | "openai-responses" | "anthropic" | "google"
        ),
        "Hermes selected model protocol is unsupported"
    );
    anyhow::ensure!(
        profile.model_supports_temperature,
        "Hermes requires a model supporting temperature"
    );
    let relay = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("Hermes relay endpoint is invalid"))?;
    anyhow::ensure!(
        relay.scheme() == "http"
            && relay.host_str() == Some("127.0.0.1")
            && relay.port().is_some()
            && relay.username().is_empty()
            && relay.password().is_none()
            && relay.path() == "/"
            && relay.query().is_none()
            && relay.fragment().is_none(),
        "Hermes requires a loopback relay root"
    );
    // Hermes applies machine-managed settings after its private config. Never
    // bypass that policy or silently route through its account instead.
    anyhow::ensure!(
        std::env::var_os("HERMES_MANAGED_DIR").is_none()
            && !std::path::Path::new("/etc/hermes").exists(),
        "Hermes session model cannot verify machine-managed configuration"
    );
    for key in profile.config().env.keys() {
        anyhow::ensure!(
            !matches!(
                key.to_ascii_uppercase().as_str(),
                "HERMES_HOME"
                    | "HERMES_HOME_MODE"
                    | "HERMES_PROFILE"
                    | "HERMES_CONFIG"
                    | "HERMES_ENV"
                    | "HERMES_MANAGED_DIR"
                    | "CUSTOM_BASE_URL"
            ),
            "conflicting Hermes model-source environment"
        );
    }
    let base_url = format!("{}/v1", endpoint.trim_end_matches('/'));
    let route = serde_json::json!({
        "provider": "custom",
        "base_url": base_url, "api_key": profile.binding.bearer_token(),
        "api_mode": "chat_completions",
    });
    let mut auxiliary_route = route.clone();
    auxiliary_route["model"] = serde_json::json!(profile.model_alias);
    auxiliary_route["fallback_chain"] = serde_json::json!([]);
    let mut auxiliary = serde_json::Map::new();
    for task in AUXILIARY_TASKS {
        auxiliary.insert((*task).into(), auxiliary_route.clone());
    }
    let mut main = route.clone();
    main["default"] = serde_json::json!(profile.model_alias);
    main["max_tokens"] = serde_json::json!(if profile.model_output_limit == 0 {
        vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
    } else {
        profile.model_output_limit
    });
    let mut delegation = route;
    delegation["model"] = serde_json::json!(profile.model_alias);
    let config = serde_json::json!({
        "model": main, "auxiliary": auxiliary, "delegation": delegation,
        "fallback_providers": [], "fallback_model": null,
    });
    profile.overlay_environment(std::collections::HashMap::from([
        ("HERMES_HOME".into(), profile.directory().to_string_lossy().into_owned()),
        ("HERMES_HOME_MODE".into(), "0700".into()),
        ("CUSTOM_BASE_URL".into(), base_url),
    ]))?;
    profile.write_config("config.yaml", &serde_json::to_vec(&config)?)?;
    // Suppress the installed project's .env override semantics without changing
    // HOME; inline credentials above remain owned by this request profile.
    profile.write_config(".env", b"")?;
    // Unpatched 0.19.0 rejects the flag instead of advertising broken restore.
    profile.append_arguments(&["--session-model-binding".into()])?;
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "hermes_tests.rs"]
mod tests;
