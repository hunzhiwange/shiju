use super::AcpLaunchProfile;
use serde_json::{Value, json};
use std::collections::HashMap;

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("Gemini relay endpoint is invalid"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "Gemini requires a loopback relay endpoint"
    );
    super::launch_route("gemini", profile)?;
    let config = profile.config();
    let command = std::path::Path::new(config.command.trim()).file_name().and_then(|v| v.to_str());
    anyhow::ensure!(
        matches!(command, Some("gemini" | "gemini.cmd" | "gemini.exe")),
        "Gemini requires the verified CLI executable"
    );
    anyhow::ensure!(
        config
            .args
            .first()
            .is_some_and(|arg| matches!(arg.as_str(), "--experimental-acp" | "--acp"))
            && (config.args.len() == 1
                || matches!(&config.args[1..], [a,b,c] if a == "--approval-mode" && b == "yolo" && c == "--skip-trust")),
        "Gemini managed launch requires verified ACP arguments"
    );
    anyhow::ensure!(
        !config.env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            [
                "GEMINI_",
                "GOOGLE_",
                "NODE_",
                "CLOUD_SHELL",
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "ALL_PROXY",
                "NO_PROXY",
            ]
            .iter()
            .any(|prefix| key.starts_with(prefix))
        }),
        "conflicting Gemini model or authentication environment"
    );
    // Retain administrator policies. Never replace an existing routing policy
    // with a session override or silently borrow its account.
    #[cfg(target_os = "macos")]
    let system = std::path::Path::new("/Library/Application Support/GeminiCli/settings.json");
    #[cfg(not(target_os = "macos"))]
    let system = std::path::Path::new("/etc/gemini-cli/settings.json");
    let mut settings = match std::fs::read(system) {
        Ok(bytes) => {
            let settings: Value = serde_json::from_slice(&bytes)
                .map_err(|_| anyhow::anyhow!("Gemini system settings cannot be verified"))?;
            anyhow::ensure!(settings.is_object(), "Gemini system settings cannot be verified");
            anyhow::ensure!(
                settings.get("model").is_none()
                    && settings.get("modelConfigs").is_none()
                    && settings.pointer("/security/auth").is_none(),
                "Gemini administrator routing policy conflicts with session_model"
            );
            settings
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(_) => anyhow::bail!("Gemini system settings cannot be verified"),
    };
    let security = settings.as_object_mut().unwrap().entry("security").or_insert(json!({}));
    anyhow::ensure!(security.is_object(), "Gemini system security settings cannot be verified");
    security["auth"] = json!({"selectedType":"gateway"});
    settings["model"] = json!({"name":profile.model_alias});
    // Native 0.66.0 resolves this last override for main, helper and compression
    // requests, without changing tool permissions or executing any tools here.
    settings["modelConfigs"] = json!({"aliases":{}, "customAliases":{}, "overrides":[{"match":{},"modelConfig":{"model":profile.model_alias}}], "customOverrides":[]});
    let advanced = settings.as_object_mut().unwrap().entry("advanced").or_insert(json!({}));
    anyhow::ensure!(advanced.is_object(), "Gemini system advanced settings cannot be verified");
    advanced["ignoreLocalEnv"] = true.into();
    let reference = profile
        .config()
        .env
        .get(super::super::profile::REFERENCE_ENV)
        .and_then(|value| value.strip_prefix("vw-session-model:"))
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| anyhow::anyhow!("Gemini requires a session-owned history reference"))?;
    let history = profile
        .directory()
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Gemini private runtime has no parent"))?
        .join("shiju-acp-gemini-history")
        .join(reference);
    let history_env = history
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Gemini history path is not UTF-8"))?
        .to_owned();
    let path = profile.directory().join("managed-settings.json");
    let mut env = HashMap::from([
        ("SHIJU_GEMINI_HISTORY_ROOT".into(), history_env),
        ("GEMINI_CLI_HOME".into(), profile.directory().to_string_lossy().into_owned()),
        ("GEMINI_CLI_SYSTEM_SETTINGS_PATH".into(), path.to_string_lossy().into_owned()),
        ("GOOGLE_GEMINI_BASE_URL".into(), endpoint.trim_end_matches('/').into()),
        ("GEMINI_API_KEY".into(), profile.binding.bearer_token().into()),
        ("GEMINI_MODEL".into(), profile.model_alias.clone()),
        ("GOOGLE_GENAI_USE_GCA".into(), "false".into()),
        ("GOOGLE_GENAI_USE_VERTEXAI".into(), "false".into()),
        ("GEMINI_CLI_USE_COMPUTE_ADC".into(), "false".into()),
        ("CLOUD_SHELL".into(), "false".into()),
    ]);
    for key in [
        "GOOGLE_API_KEY",
        "GOOGLE_APPLICATION_CREDENTIALS",
        "GOOGLE_VERTEX_BASE_URL",
        "GEMINI_CLI_AUTH_OVERRIDE",
        "GEMINI_CLI_CUSTOM_HEADERS",
        "GEMINI_API_KEY_AUTH_MECHANISM",
        "GOOGLE_GENAI_API_VERSION",
        "DEBUG",
        "DEBUG_MODE",
        "NODE_OPTIONS",
        "NODE_PATH",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        env.insert(key.into(), String::new());
    }
    profile.overlay_environment(env)?;
    ensure_private_history(profile, &history)?;
    profile.write_config("managed-settings.json", &serde_json::to_vec(&settings)?)?;
    profile.append_arguments(&["--shiju-model-source".into()])?;
    Ok(())
}

fn ensure_private_history(
    profile: &AcpLaunchProfile,
    history: &std::path::Path,
) -> anyhow::Result<()> {
    #[cfg(not(unix))]
    {
        let _ = (profile, history);
        anyhow::bail!("Gemini history ACL verification unavailable on this platform");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        let owner = profile.directory().symlink_metadata()?.uid();
        let root =
            history.parent().ok_or_else(|| anyhow::anyhow!("Gemini history root is missing"))?;
        for directory in [root, history] {
            match std::fs::DirBuilder::new().mode(0o700).create(directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => anyhow::bail!("cannot create private Gemini history directory"),
            }
            let metadata = directory.symlink_metadata()?;
            anyhow::ensure!(
                metadata.is_dir()
                    && metadata.uid() == owner
                    && metadata.permissions().mode() & 0o777 == 0o700,
                "insecure Gemini history directory"
            );
        }
        Ok(())
    }
}

#[cfg(all(test, unix))]
#[path = "gemini_tests.rs"]
mod tests;
