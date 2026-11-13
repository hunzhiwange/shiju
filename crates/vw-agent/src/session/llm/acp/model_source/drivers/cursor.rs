use super::AcpLaunchProfile;
use serde_json::json;
use std::collections::HashMap;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
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
        "Cursor requires a loopback relay endpoint"
    );
    let config = profile.config();
    anyhow::ensure!(config.args == ["acp"], "Cursor requires verified ACP arguments");
    anyhow::ensure!(
        !config.env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("CURSOR_")
                || key.starts_with("SHIJU_CURSOR_")
                || key.starts_with("ANTHROPIC_")
                || matches!(
                    key.as_str(),
                    "AGENT_CLI_CREDENTIAL_STORE"
                        | "NODE_OPTIONS"
                        | "NODE_PATH"
                        | "HTTP_PROXY"
                        | "HTTPS_PROXY"
                        | "ALL_PROXY"
                )
        }),
        "conflicting Cursor model or authentication environment"
    );
    vw_acp::verify_reviewed_agent_executable("cursor", &config.command)
        .map_err(anyhow::Error::msg)?;
    anyhow::ensure!(profile.model_supports_tool_call, "Cursor requires model tool support");
    let model = profile.model_alias.clone();
    let content = serde_json::to_vec(&json!({
        "model": model, "endpoint": endpoint, "token": profile.binding.bearer_token(),
        "outputLimit": profile.model_output_limit,
    }))?;
    let history = history_directory(profile)?;
    let path = profile.directory().join("model.json");
    profile.overlay_environment(HashMap::from([
        ("SHIJU_CURSOR_MODEL_CONFIG".into(), path.to_string_lossy().into_owned()),
        ("CURSOR_CONFIG_DIR".into(), history.join("config").to_string_lossy().into_owned()),
        ("CURSOR_DATA_DIR".into(), history.join("data").to_string_lossy().into_owned()),
        ("AGENT_CLI_CREDENTIAL_STORE".into(), "memory".into()),
        ("CURSOR_AGENT_CLI_AUTHLESS_MODE".into(), "true".into()),
    ]))?;
    profile.append_arguments(&["--model".into(), model])?;
    profile.write_config("model.json", &content)?;
    Ok(())
}

fn history_directory(profile: &AcpLaunchProfile) -> anyhow::Result<std::path::PathBuf> {
    #[cfg(not(unix))]
    anyhow::bail!("Cursor history ACL verification unavailable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        let reference = profile.config().env[super::super::profile::REFERENCE_ENV]
            .strip_prefix("vw-session-model:")
            .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| anyhow::anyhow!("Cursor requires a session-owned history reference"))?;
        let root = profile
            .directory()
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cursor runtime parent missing"))?
            .join("shiju-acp-cursor-history");
        let history = root.join(reference);
        for path in [&root, &history] {
            match std::fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => anyhow::bail!("cannot create private Cursor history directory"),
            }
            let metadata = path.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && metadata.uid() == unsafe { libc::geteuid() }
                    && metadata.permissions().mode() & 0o777 == 0o700,
                "insecure Cursor history directory"
            );
        }
        Ok(history)
    }
}

#[cfg(all(test, unix))]
#[path = "cursor_tests.rs"]
mod tests;
