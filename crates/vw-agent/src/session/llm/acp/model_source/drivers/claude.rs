use super::AcpLaunchProfile;
use std::collections::HashMap;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("Claude relay endpoint is invalid"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path() == "/",
        "Claude requires a loopback relay endpoint"
    );
    let config = profile.config();
    let program =
        std::path::Path::new(config.command.trim()).file_name().and_then(|name| name.to_str());
    let valid_arguments = match program {
        Some("claude-agent-acp" | "claude-agent-acp.cmd" | "claude-agent-acp.exe") => {
            config.args.is_empty()
        }
        Some("npx" | "npx.cmd" | "npx.exe") => matches!(config.args.as_slice(), [flag, package]
            if matches!(flag.as_str(), "-y" | "--yes") && matches!(package.as_str(),
                "@agentclientprotocol/claude-agent-acp@^0.26.0" | "@agentclientprotocol/claude-agent-acp@0.26.0")),
        _ => false,
    };
    anyhow::ensure!(
        valid_arguments,
        "Claude managed launch requires verified ACP adapter arguments"
    );
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Claude requires selected model transport metadata"))?;
    let native_messages = match api.adapter.as_str() {
        "anthropic" => true,
        "openai" | "openai-compatible" => {
            let url = reqwest::Url::parse(&api.url)
                .map_err(|_| anyhow::anyhow!("Claude selected model endpoint is invalid"))?;
            anyhow::ensure!(
                !url.path().trim_end_matches('/').ends_with("/responses")
                    && !url.path().trim_end_matches('/').ends_with("/messages"),
                "Claude selected model requires an unsupported protocol conversion"
            );
            false
        }
        _ => anyhow::bail!("Claude selected model protocol is unsupported"),
    };
    anyhow::ensure!(
        profile.model_supports_temperature && profile.model_supports_tool_call,
        "Claude requires a selected model supporting temperature and tools"
    );
    anyhow::ensure!(
        !profile.model_supports_reasoning || native_messages,
        "Claude reasoning requires a native Messages model; thinking signatures cannot be converted"
    );
    let model = profile.model_alias.clone();
    let mut env = HashMap::from([
        // SDK 0.2.96 filters provider/model settings overrides while preserving
        // project permissions and instructions when the host owns the provider.
        ("CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST".into(), "1".into()),
        ("ANTHROPIC_BASE_URL".into(), endpoint.trim_end_matches('/').into()),
        ("ANTHROPIC_AUTH_TOKEN".into(), profile.binding.bearer_token().into()),
        ("ANTHROPIC_API_KEY".into(), String::new()),
        ("ANTHROPIC_CUSTOM_HEADERS".into(), String::new()),
        ("CLAUDE_CODE_OAUTH_TOKEN".into(), String::new()),
        ("CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR".into(), String::new()),
        ("CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR".into(), String::new()),
        ("CLAUDE_CODE_EXECUTABLE".into(), String::new()),
        ("CLAUDE_CODE_USE_BEDROCK".into(), "0".into()),
        ("CLAUDE_CODE_USE_VERTEX".into(), "0".into()),
        ("CLAUDE_CODE_USE_FOUNDRY".into(), "0".into()),
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".into(), "1".into()),
        ("DISABLE_AUTOUPDATER".into(), "1".into()),
    ]);
    env.insert(
        "CLAUDE_CODE_MAX_OUTPUT_TOKENS".into(),
        if profile.model_output_limit == 0 {
            vw_shared::provider::state::DEFAULT_MODEL_OUTPUT_LIMIT.to_string()
        } else {
            profile.model_output_limit.to_string()
        },
    );
    if !profile.model_supports_reasoning {
        // SDK 0.2.96 honors zero by omitting adaptive thinking/context edits and
        // choosing its non-thinking request path. Do not fabricate thought state.
        env.insert("MAX_THINKING_TOKENS".into(), "0".into());
    }
    for key in [
        "ANTHROPIC_MODEL",
        "ANTHROPIC_SMALL_FAST_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "CLAUDE_CODE_SUBAGENT_MODEL",
    ] {
        env.insert(key.into(), model.clone());
    }
    // Enterprise policy remains authoritative. Refuse routing overrides instead of
    // silently sending the session to an enterprise account/backend.
    #[cfg(target_os = "macos")]
    let managed =
        std::path::Path::new("/Library/Application Support/ClaudeCode/managed-settings.json");
    #[cfg(not(target_os = "macos"))]
    let managed = std::path::Path::new("/etc/claude-code/managed-settings.json");
    match std::fs::read(managed) {
        Ok(bytes) => validate_managed_settings(&bytes)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => anyhow::bail!("Claude managed settings cannot be verified"),
    }
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            [
                "ANTHROPIC_",
                "CLAUDE_",
                "AWS_",
                "GOOGLE_",
                "AZURE_",
                "NODE_",
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "ALL_PROXY",
                "NO_PROXY",
            ]
            .into_iter()
            .any(|prefix| key.starts_with(prefix))
        }),
        "conflicting Claude model or authentication environment"
    );
    let settings =
        serde_json::to_vec(&serde_json::json!({"model": model, "env": env, "apiKeyHelper": ""}))?;
    env.insert(
        "CLAUDE_CONFIG_DIR".into(),
        profile.claude_state_directory()?.to_string_lossy().into_owned(),
    );
    env.insert(
        "VW_ACP_CLAUDE_SETTINGS".into(),
        profile.directory().join("settings.json").to_string_lossy().into_owned(),
    );
    // Validate all explicit overrides before publishing private launch material.
    profile.overlay_environment(env)?;
    profile.write_config("settings.json", &settings)?;
    Ok(())
}

fn validate_managed_settings(bytes: &[u8]) -> anyhow::Result<()> {
    let settings: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| anyhow::anyhow!("Claude managed settings cannot be verified"))?;
    anyhow::ensure!(settings.is_object(), "Claude managed settings cannot be verified");
    anyhow::ensure!(
        settings.get("env").is_none_or(serde_json::Value::is_object),
        "Claude managed settings cannot be verified"
    );
    let routing = settings.get("model").is_some()
        || settings.get("apiKeyHelper").is_some()
        || settings.get("env").and_then(serde_json::Value::as_object).is_some_and(|env| {
            env.keys().any(|key| {
                let key = key.to_ascii_uppercase();
                [
                    "ANTHROPIC_",
                    "CLAUDE_",
                    "AWS_",
                    "GOOGLE_",
                    "AZURE_",
                    "NODE_",
                    "HTTP_PROXY",
                    "HTTPS_PROXY",
                    "ALL_PROXY",
                    "NO_PROXY",
                ]
                .into_iter()
                .any(|prefix| key.starts_with(prefix))
            })
        });
    anyhow::ensure!(
        !routing,
        "Claude enterprise model or authentication policy conflicts with session_model"
    );
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "claude_tests.rs"]
mod tests;
