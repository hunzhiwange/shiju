use super::AcpLaunchProfile;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args.is_empty(),
        "Codex managed launch requires the ACP entry"
    );
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("invalid Codex relay endpoint"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str().is_some_and(|host| {
                host.parse::<std::net::IpAddr>().is_ok_and(|ip| ip.is_loopback())
            })
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "Codex requires an owned loopback relay"
    );
    anyhow::ensure!(!profile.model_alias.is_empty(), "Codex requires a selected model");
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Codex requires selected model transport metadata"))?;
    let native_responses = api.adapter == "openai-responses"
        || (matches!(api.adapter.as_str(), "openai" | "openai-compatible")
            && reqwest::Url::parse(&api.url)
                .is_ok_and(|url| url.path().trim_end_matches('/').ends_with("/responses")));
    anyhow::ensure!(
        native_responses,
        "Codex requires a Responses model; its native tools cannot be converted losslessly"
    );
    use sha2::Digest;
    // Project config cannot predefine routing headers for this request-owned provider key.
    let provider = format!(
        "shiju_session_{}",
        hex::encode(sha2::Sha256::digest(profile.binding.bearer_token().as_bytes()))
    );
    let config = serde_json::json!({
        "model": profile.model_alias,
        "review_model": profile.model_alias,
        "model_context_window": profile.model_context_limit,
        "model_provider": provider,
        "model_providers": {provider.clone(): {
            "name": "Session model relay",
            "base_url": format!("{endpoint}/v1"),
            "wire_api": "responses",
            "env_key": "SHIJU_CODEX_RELAY_TOKEN",
            "requires_openai_auth": false,
            "request_max_retries": 0,
            "stream_max_retries": 0
        }}
    });
    let native = toml::to_string(&config)?;
    let reference = profile
        .config()
        .env
        .get(super::super::profile::REFERENCE_ENV)
        .and_then(|value| value.strip_prefix("vw-session-model:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| anyhow::anyhow!("Codex requires a session-owned history reference"))?;
    let history = profile
        .directory()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Codex private runtime has no parent"))?
        .join("shiju-acp-codex-history")
        .join(reference);
    let history_env = history
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Codex history path is not UTF-8"))?
        .to_owned();
    profile.overlay_environment(std::collections::HashMap::from([
        ("CODEX_HOME".into(), profile.directory().to_string_lossy().into_owned()),
        ("SHIJU_CODEX_HISTORY_HOME".into(), history_env),
        ("CODEX_CONFIG".into(), serde_json::to_string(&config)?),
        ("MODEL_PROVIDER".into(), provider),
        ("SHIJU_CODEX_RELAY_TOKEN".into(), profile.binding.bearer_token().into()),
        ("SHIJU_CODEX_MODEL_LOCK".into(), profile.model_alias.clone()),
        // The fixed adapter logs raw configuration/requests when this is enabled.
        ("APP_SERVER_LOGS".into(), String::new()),
        ("DEFAULT_AUTH_REQUEST".into(), String::new()),
        ("CODEX_PATH".into(), String::new()),
        ("NODE_OPTIONS".into(), String::new()),
        ("NODE_PATH".into(), String::new()),
    ]))?;
    ensure_private_history(profile, &history)?;
    // App-server startup and auxiliary threads need the same native defaults as ACP sessions.
    profile.write_config("config.toml", native.as_bytes())?;
    profile.append_arguments(&["cli".into(), "--shiju-model-source".into()])?;
    Ok(())
}

// Keep resumable state separate from the short-lived routing config and bearer lease.
fn ensure_private_history(
    profile: &AcpLaunchProfile,
    history: &std::path::Path,
) -> anyhow::Result<()> {
    #[cfg(not(unix))]
    {
        let _ = (profile, history);
        anyhow::bail!("Codex history ACL verification unavailable on this platform");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        let owner = profile.directory().symlink_metadata()?.uid();
        let root =
            history.parent().ok_or_else(|| anyhow::anyhow!("Codex history root is missing"))?;
        for directory in [root, history] {
            match std::fs::DirBuilder::new().mode(0o700).create(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => anyhow::bail!("cannot create private Codex history directory"),
            }
            let metadata = directory.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && metadata.uid() == owner
                    && metadata.permissions().mode() & 0o777 == 0o700,
                "insecure Codex history directory"
            );
        }
        Ok(())
    }
}

#[cfg(all(test, unix))]
#[path = "codex_tests.rs"]
mod tests;
