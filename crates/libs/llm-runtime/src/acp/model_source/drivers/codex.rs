use super::AcpLaunchProfile;

pub fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    configure_launch(profile, endpoint, None)
}

/// The caller owns the reviewed mounted service projection and its execution
/// relay. This candidate is deliberately independent of public capability discovery.
pub fn configure_host_mcp_candidate(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    server: &str,
    service_url: &str,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        cfg!(all(
            target_os = "linux",
            target_env = "gnu",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )),
        "Codex host MCP candidate requires Linux GNU"
    );
    anyhow::ensure!(
        valid_identifier(server, 48)
            && service_url == format!("{endpoint}/shiju-vtcode-mcp/{server}"),
        "Codex host MCP requires the reviewed private service route"
    );
    configure_launch(profile, endpoint, Some((server, service_url)))
}

/// Only use this decoder on the Codex-owned projection. The engine writes
/// callId on tools/call; ordinary MCP JSON-RPC IDs do not identify approvals.
pub fn host_mcp_call_id(params: &serde_json::Value) -> Option<&str> {
    let metadata = params.get("_meta")?.as_object()?;
    if metadata.contains_key("shijuCallId") {
        return None;
    }
    metadata.get("callId")?.as_str().filter(|id| valid_identifier(id, 128))
}

fn valid_identifier(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn configure_launch(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    host_mcp: Option<(&str, &str)>,
) -> anyhow::Result<()> {
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
    let native_responses = !super::launch_route("codex", profile)?.conversion_required;
    anyhow::ensure!(
        host_mcp.is_none() || native_responses,
        "Codex host MCP candidate requires native Responses"
    );
    use sha2::Digest;
    // Project config cannot predefine routing headers for this request-owned provider key.
    let provider = format!(
        "shiju_session_{}",
        hex::encode(sha2::Sha256::digest(profile.binding.bearer_token().as_bytes()))
    );
    let mut config = serde_json::json!({
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
    if !native_responses || host_mcp.is_some() {
        // The Chat transport and the host MCP candidate cannot use hosted search.
        config["web_search"] = "disabled".into();
    }
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
    let (host_mcp_server, host_mcp_url) = host_mcp.unwrap_or_default();
    let mut environment = std::collections::HashMap::from([
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
    ]);
    if cfg!(target_os = "linux") {
        environment.insert("SHIJU_CODEX_HOST_MCP_SERVER".into(), host_mcp_server.into());
        environment.insert("SHIJU_CODEX_HOST_MCP_URL".into(), host_mcp_url.into());
    }
    profile.overlay_environment(environment)?;
    ensure_private_history(profile, &history)?;
    // App-server startup and auxiliary threads need the same native defaults as ACP sessions.
    profile.write_config("config.toml", native.as_bytes())?;
    profile.append_arguments(&["cli".into(), "--shiju-model-source".into()])?;
    Ok(())
}

// Keep resumable state separate from the short-lived routing config and bearer lease.
pub fn ensure_private_history(
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
