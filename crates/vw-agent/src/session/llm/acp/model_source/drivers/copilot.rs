use super::AcpLaunchProfile;
use std::collections::HashMap;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    let url = reqwest::Url::parse(endpoint)?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path() == "/",
        "Copilot requires a loopback relay endpoint"
    );
    let config = profile.config();
    let program = std::path::Path::new(config.command.trim()).file_name().and_then(|s| s.to_str());
    anyhow::ensure!(
        matches!(program, Some("copilot" | "copilot.cmd" | "copilot.exe"))
            && config.args == ["--acp", "--stdio"],
        "Copilot managed launch requires verified ACP arguments"
    );
    anyhow::ensure!(
        !config.env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("COPILOT_")
                || matches!(
                    key.as_str(),
                    "GH_TOKEN"
                        | "GITHUB_TOKEN"
                        | "GH_COPILOT_TOKEN"
                        | "NODE_OPTIONS"
                        | "NODE_PATH"
                        | "HTTP_PROXY"
                        | "HTTPS_PROXY"
                        | "ALL_PROXY"
                        | "NO_PROXY"
                )
        }),
        "conflicting Copilot provider or authentication environment"
    );
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Copilot requires selected model transport metadata"))?;
    let (provider_type, base_url) = match api.adapter.as_str() {
        "anthropic" => ("anthropic", endpoint.trim_end_matches('/').to_string()),
        "openai" | "openai-compatible" => {
            let upstream = reqwest::Url::parse(&api.url)?;
            anyhow::ensure!(
                !upstream.path().trim_end_matches('/').ends_with("/responses"),
                "Copilot Responses conversion is not verified"
            );
            ("openai", format!("{}/v1", endpoint.trim_end_matches('/')))
        }
        _ => anyhow::bail!("Copilot selected model protocol is unsupported"),
    };
    anyhow::ensure!(
        profile.model_supports_tool_call && profile.model_supports_temperature,
        "Copilot requires a selected model supporting tools and temperature"
    );
    anyhow::ensure!(
        !profile.model_supports_reasoning,
        "Copilot reasoning model routing has not been verified"
    );
    let model = &profile.model_alias;
    let content = serde_json::to_vec(&serde_json::json!({
        "providers": [{"name": "vibewindow", "type": provider_type, "baseUrl": base_url,
            "apiKey": profile.binding.bearer_token(), "wireApi": "completions"}],
        "models": [{"id": model, "provider": "vibewindow", "wireModel": model,
            "maxOutputTokens": profile.model_output_limit}]
    }))?;
    let directory = profile.directory().to_string_lossy().into_owned();
    profile.overlay_environment(HashMap::from([
        ("COPILOT_HOME".into(), directory.clone()),
        ("COPILOT_PROVIDERS_CONFIG".into(), format!("{directory}/providers.json")),
        ("COPILOT_MODEL".into(), format!("vibewindow/{model}")),
        ("COPILOT_OFFLINE".into(), "true".into()),
        ("COPILOT_AUTO_UPDATE".into(), "false".into()),
    ]))?;
    preserve_history(profile)?;
    profile.append_arguments(&["--no-auto-update".into()])?;
    profile.write_config("providers.json", &content)?;
    Ok(())
}

// Persist only native history. Provider credentials remain in the leased profile.
fn preserve_history(profile: &AcpLaunchProfile) -> anyhow::Result<()> {
    #[cfg(not(unix))]
    anyhow::bail!("Copilot history ACL verification unavailable on this platform");
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        let reference = profile
            .config()
            .env
            .get(super::super::profile::REFERENCE_ENV)
            .and_then(|value| value.strip_prefix("vw-session-model:"))
            .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| anyhow::anyhow!("Copilot requires a session-owned history reference"))?;
        let root = profile
            .directory()
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Copilot runtime parent missing"))?
            .join("shiju-acp-copilot-history");
        let history = root.join(reference);
        let owner = profile.directory().symlink_metadata()?.uid();
        for path in [&root, &history] {
            match std::fs::DirBuilder::new().mode(0o700).create(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => anyhow::bail!("cannot create private Copilot history directory"),
            }
            let metadata = path.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && metadata.uid() == owner
                    && metadata.permissions().mode() & 0o777 == 0o700,
                "insecure Copilot history directory"
            );
        }
        std::os::unix::fs::symlink(history, profile.directory().join("session-state"))?;
        Ok(())
    }
}

#[cfg(all(test, unix))]
#[path = "copilot_tests.rs"]
mod tests;
