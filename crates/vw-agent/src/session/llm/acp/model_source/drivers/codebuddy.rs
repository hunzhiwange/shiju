//! CodeBuddy 2.179.3 uses a base URL, while models.json uses a complete Chat URL.
use std::collections::HashMap;

use super::AcpLaunchProfile;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    // Model/settings/resume flags can select a native route or reuse its session anchor.
    anyhow::ensure!(
        profile.config().args == ["--acp"],
        "ACP codebuddy custom launch arguments has not been verified"
    );
    anyhow::ensure!(
        profile.model_supports_temperature,
        "CodeBuddy Chat requires a model supporting temperature"
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
        "CodeBuddy requires a private loopback relay endpoint"
    );
    let directory = profile.directory().to_string_lossy().into_owned();
    let model = profile.model_alias.clone();
    // File URLs take precedence over BASE_URL upstream. Disable their provider,
    // then disable settings layers (including their env and apiKeyHelper values).
    let mut env = HashMap::from([
        ("CODEBUDDY_CONFIG_DIR".into(), directory.clone()),
        ("WORKBUDDY_CONFIG_DIR".into(), directory),
        ("CODEBUDDY_BASE_URL".into(), format!("{}/v1", endpoint.trim_end_matches('/'))),
        ("CODEBUDDY_API_KEY".into(), profile.binding.bearer_token().to_owned()),
        ("CODEBUDDY_AUTH_TOKEN".into(), String::new()),
        ("CODEBUDDY_API_KEY_HELPER_DISABLED".into(), "1".into()),
        ("CODEBUDDY_API_KEY_DISABLED".into(), String::new()),
        ("CODEBUDDY_DISABLE_BUILTIN_MODELS".into(), "1".into()),
        (
            "CODEBUDDY_CODE_MAX_OUTPUT_TOKENS".into(),
            if profile.model_output_limit == 0 {
                vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT
            } else {
                profile.model_output_limit
            }
            .to_string(),
        ),
        ("CODEBUDDY_DISABLE_CUSTOM_MODELS_FILE".into(), "1".into()),
    ]);
    for key in [
        "CODEBUDDY_MODEL",
        "CODEBUDDY_SMALL_FAST_MODEL",
        "CODEBUDDY_BIG_SLOW_MODEL",
        "CODEBUDDY_CODE_SUBAGENT_MODEL",
    ] {
        env.insert(key.into(), model.clone());
    }
    profile.overlay_environment(env)?;
    profile.append_arguments(&["--setting-sources".into(), "none".into()])
}

#[cfg(test)]
#[path = "codebuddy_tests.rs"]
mod tests;
