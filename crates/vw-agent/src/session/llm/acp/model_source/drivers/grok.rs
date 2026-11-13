use super::AcpLaunchProfile;
#[cfg(unix)]
use std::collections::HashMap;

/// Grok Build's explicit environment overrides are read without changing CLI defaults.
#[cfg(unix)]
pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["agent", "stdio"]
            || profile.config().args == ["-y", "@xai-official/grok@1.0.77", "agent", "stdio"],
        "Grok Build custom launch arguments are not verified"
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
        "Grok Build requires a private loopback relay"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && !profile.model_alias.is_empty(),
        "Grok Build requires a selected tool-capable model"
    );
    let home = {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        let root = profile.state_root.as_deref().unwrap_or(profile.directory());
        anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid Grok Build state root");
        let reference = super::super::profile::launch_reference(&profile.binding.identity)?;
        let reference = reference.strip_prefix("vw-session-model:").unwrap();
        let mut home = root.to_path_buf();
        for component in ["acp-grok-state", reference] {
            home.push(component);
            match std::fs::DirBuilder::new().mode(0o700).create(&home) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
            let metadata = home.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && !metadata.file_type().is_symlink()
                    && metadata.permissions().mode() & 0o777 == 0o700
                    && metadata.uid() == unsafe { libc::geteuid() },
                "insecure Grok Build state directory"
            );
        }
        home
    };
    let model = serde_json::json!({
        "model": profile.model_alias,
        "base_url": format!("{}/v1", endpoint.trim_end_matches('/')),
        "env_key": "SHIJU_GROK_RELAY_KEY",
        "api_backend": "responses"
    });
    let document = serde_json::json!({
        "model": {"shiju-session":model, "grok-4.6":model, "grok-4.5":model},
        "models": {"default":"shiju-session", "session_summary":"shiju-session",
            "prompt_suggestion":"shiju-session", "image_description":"shiju-session",
            "web_search":"shiju-session", "allowed_models":["shiju-session"], "max_retries":0},
        "subagents":{"enabled":false},
        "cli":{"use_leader":false, "auto_update":false}
    });
    let contents = toml::to_string(&document)?;
    let config = home.join("config.toml");
    if config.exists() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let metadata = config.symlink_metadata()?;
        anyhow::ensure!(
            metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.permissions().mode() & 0o777 == 0o600
                && metadata.uid() == unsafe { libc::geteuid() },
            "unsafe Grok config"
        );
        let saved: serde_json::Value = toml::from_str(&std::fs::read_to_string(&config)?)?;
        for key in ["model", "base_url", "env_key", "api_backend"] {
            anyhow::ensure!(
                saved["model"]["shiju-session"][key] == model[key],
                "Grok session binding changed"
            );
        }
    }
    // Grok writes UI defaults on startup. Republish only our generated settings
    // on restore, retaining the session store and never accepting CLI-written permissions.
    {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new_in(&home)?;
        file.write_all(contents.as_bytes())?;
        file.persist(&config)?;
    }
    let mut env = HashMap::from([
        ("GROK_DEFAULT_MODEL".into(), "shiju-session".into()),
        ("GROK_HOME".into(), home.to_string_lossy().into_owned()),
        ("SHIJU_GROK_RELAY_KEY".into(), profile.binding.bearer_token().into()),
    ]);
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"]
    {
        env.insert(key.into(), String::new());
    }
    env.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    env.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    profile.overlay_environment(env)?;
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn configure(_profile: &mut AcpLaunchProfile, _endpoint: &str) -> anyhow::Result<()> {
    anyhow::bail!("Grok Build state ACL is not verified on this platform")
}

#[cfg(all(test, unix))]
#[path = "grok_tests.rs"]
mod tests;
