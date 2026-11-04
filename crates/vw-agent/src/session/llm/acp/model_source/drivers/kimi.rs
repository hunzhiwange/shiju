use super::AcpLaunchProfile;
use serde_json::json;
use std::{collections::HashMap, io::Write, path::Path};

/// Kimi Code 2.5.0 reads a single config beneath KIMI_CODE_HOME, including in ACP.
/// Keep history across relay rotations, but persist only an environment key name.
pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["acp"],
        "Kimi requires the native ACP entry without overrides"
    );
    anyhow::ensure!(!profile.model_alias.is_empty(), "Kimi requires a selected model");
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("KIMI_")
                || key.starts_with("VW_KIMI_")
                || matches!(
                    key.as_str(),
                    "NODE_OPTIONS" | "BUN_OPTIONS" | "HTTP_PROXY" | "HTTPS_PROXY" | "ALL_PROXY"
                )
        }),
        "conflicting Kimi model environment"
    );
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("invalid Kimi relay endpoint"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "Kimi requires an owned loopback relay"
    );
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Kimi requires selected model transport"))?;
    let (kind, suffix) = match api.adapter.as_str() {
        "openai-responses" => ("openai_responses", "/v1"),
        "openai" | "openai-compatible" => {
            let upstream = reqwest::Url::parse(&api.url)
                .map_err(|_| anyhow::anyhow!("invalid Kimi upstream transport"))?;
            if upstream.path().trim_end_matches('/').ends_with("/responses") {
                ("openai_responses", "/v1")
            } else {
                ("openai", "/v1")
            }
        }
        "anthropic" => ("anthropic", ""),
        "google" => ("google-genai", ""),
        _ => anyhow::bail!("Kimi selected model protocol is unsupported"),
    };
    let root = profile.state_root.as_deref().unwrap_or(profile.directory());
    anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid Kimi state root");
    use sha2::Digest;
    let identity = &profile.binding.identity;
    let reference = hex::encode(sha2::Sha256::digest(serde_json::to_vec(&json!({
        "tenant": identity.tenant, "project": identity.project, "session": identity.session,
    }))?));
    let mut home = root.to_path_buf();
    for component in ["acp-kimi-state", &reference] {
        home.push(component);
        private_directory(&home)?;
    }
    let mut capabilities = Vec::new();
    if profile.model_supports_tool_call {
        capabilities.push("tool_use");
    }
    if profile.model_supports_reasoning {
        capabilities.push("thinking");
    }
    let settings = json!({
        "default_model": "session-selected", "telemetry": false,
        "providers": {"session-provider": {"type": kind,
            "base_url": format!("{endpoint}{suffix}"), "api_key_env": "VW_KIMI_RELAY_TOKEN"}},
        "models": {"session-selected": {"provider": "session-provider", "model": profile.model_alias,
            "max_context_size": profile.model_context_limit.max(1),
            "max_output_size": profile.model_output_limit.max(1), "capabilities": capabilities}},
    });
    let mut environment = HashMap::from([
        ("KIMI_CODE_HOME".into(), home.to_string_lossy().into_owned()),
        ("VW_KIMI_RELAY_TOKEN".into(), profile.binding.bearer_token().to_owned()),
        ("KIMI_MODEL_NAME".into(), String::new()),
        ("KIMI_MODEL_THINKING_KEEP".into(), String::new()),
        ("KIMI_CODE_CUSTOM_HEADERS".into(), String::new()),
        ("KIMI_DISABLE_TELEMETRY".into(), "1".into()),
        ("KIMI_CODE_NO_AUTO_UPDATE".into(), "1".into()),
        ("NODE_OPTIONS".into(), String::new()),
        ("BUN_OPTIONS".into(), String::new()),
    ]);
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"]
    {
        environment.insert(key.into(), String::new());
    }
    environment.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    environment.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    profile.overlay_environment(environment)?;
    let path = home.join("config.toml");
    if let Ok(metadata) = path.symlink_metadata() {
        anyhow::ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "unsafe Kimi config file"
        );
    }
    let mut file = tempfile::NamedTempFile::new_in(&home)?;
    file.write_all(toml::to_string(&settings)?.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|_| anyhow::anyhow!("cannot publish Kimi configuration"))?;
    Ok(())
}

fn private_directory(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        match std::fs::DirBuilder::new().mode(0o700).create(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let metadata = path.symlink_metadata()?;
        anyhow::ensure!(
            metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.permissions().mode() & 0o777 == 0o700
                && metadata.uid() == unsafe { libc::geteuid() },
            "insecure Kimi state directory"
        );
        Ok(())
    }
    #[cfg(not(unix))]
    anyhow::bail!("Kimi private state ACL verification unavailable")
}

#[cfg(test)]
#[path = "kimi_tests.rs"]
mod tests;
