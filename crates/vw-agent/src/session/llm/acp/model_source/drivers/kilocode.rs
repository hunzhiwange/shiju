use super::AcpLaunchProfile;
use serde_json::json;
use std::{collections::HashMap, path::Path};

pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    let url = reqwest::Url::parse(endpoint)?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "Kilocode requires an owned loopback relay"
    );
    anyhow::ensure!(!profile.model_alias.is_empty(), "Kilocode requires a selected model");
    for key in
        ["KILO_CONFIG", "KILO_CONFIG_CONTENT", "KILO_CONFIG_DIR", "KILO_TEST_MANAGED_CONFIG_DIR"]
    {
        anyhow::ensure!(
            std::env::var_os(key).is_none_or(|value| value.is_empty()),
            "conflicting inherited Kilocode configuration"
        );
    }
    let config = profile.config();
    anyhow::ensure!(
        matches!(
            Path::new(&config.command).file_name().and_then(|s| s.to_str()),
            Some("kilo" | "kilocode")
        ),
        "Kilocode requires its verified native executable"
    );
    anyhow::ensure!(
        (config.args == ["acp"] || config.args == ["acp", "--pure"]),
        "Kilocode requires the native ACP entry"
    );
    anyhow::ensure!(
        !config.env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("VW_KILO_")
                || key.starts_with("KILO_")
                || key.starts_with("OPENCODE_")
                || key.starts_with("XDG_")
                || matches!(
                    key.as_str(),
                    "NODE_OPTIONS"
                        | "BUN_OPTIONS"
                        | "HTTP_PROXY"
                        | "HTTPS_PROXY"
                        | "ALL_PROXY"
                        | "NO_PROXY"
                )
        }),
        "conflicting Kilocode launch environment"
    );
    // Native managed policy loads after KILO_CONFIG_CONTENT. Preserve its authority
    // by refusing this mode rather than overwriting or bypassing administrator rules.
    #[cfg(target_os = "macos")]
    for path in [
        "/Library/Application Support/kilo",
        "/Library/Managed Preferences/ai.opencode.managed.plist",
    ] {
        reject_managed_configuration(Path::new(path))?;
    }
    #[cfg(not(target_os = "macos"))]
    reject_managed_configuration(Path::new("/etc/kilo"))?;
    #[cfg(target_os = "macos")]
    match std::fs::read_dir("/Library/Managed Preferences") {
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                if entry.file_type()?.is_dir() || entry.file_type()?.is_symlink() {
                    reject_managed_configuration(&entry.path().join("ai.opencode.managed.plist"))?;
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => anyhow::bail!("Kilocode managed preferences cannot be verified"),
    }
    let mut env = HashMap::new();
    for key in ["XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
        let path = profile.directory().join(key);
        std::fs::create_dir(&path)?;
        env.insert(key.to_owned(), path.to_string_lossy().into_owned());
    }
    let root = profile.state_root.as_deref().unwrap_or(profile.directory());
    anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid Kilocode state root");
    use sha2::Digest;
    let identity = &profile.binding.identity;
    let reference = hex::encode(sha2::Sha256::digest(serde_json::to_vec(&json!({
        "tenant":identity.tenant, "project":identity.project, "session":identity.session,
    }))?));
    let mut history = root.to_path_buf();
    for component in ["acp-kilocode-state", &reference] {
        history.push(component);
        private_directory(&history)?;
    }
    for key in ["XDG_DATA_HOME", "XDG_STATE_HOME"] {
        let path = history.join(key);
        private_directory(&path)?;
        env.insert(key.to_owned(), path.to_string_lossy().into_owned());
    }
    // A stable native alias restores history without restoring an old upstream ID.
    let provider = format!("shiju_{reference}");
    let model = format!("{provider}/session");
    let settings = json!({
        "model":model, "small_model":model, "subagent_model":model, "memory_model":model,
        "enabled_providers":[provider.clone()],
        "provider":{(provider):{
            "name":"Session model", "npm":"@ai-sdk/openai-compatible",
            "options":{"baseURL":format!("{endpoint}/v1"),"apiKey":"{env:VW_KILO_RELAY_TOKEN}"},
            "models":{"session":{"id":profile.model_alias,"name":"Session model",
                "tool_call":profile.model_supports_tool_call,"temperature":profile.model_supports_temperature,
                "reasoning":profile.model_supports_reasoning,
                "limit":{"context":profile.model_context_limit,"output":profile.model_output_limit}}}
        }}
    });
    let path = profile.write_config("kilo.json", &serde_json::to_vec(&settings)?)?;
    env.insert("KILO_CONFIG".into(), path.to_string_lossy().into_owned());
    env.insert("KILO_CONFIG_CONTENT".into(), serde_json::to_string(&settings)?);
    env.insert("VW_KILO_RELAY_TOKEN".into(), profile.binding.bearer_token().to_owned());
    env.insert("KILO_DISABLE_AUTOUPDATE".into(), "1".into());
    env.insert("KILO_DISABLE_MODELS_FETCH".into(), "1".into());
    env.insert("NODE_OPTIONS".into(), String::new());
    env.insert("BUN_OPTIONS".into(), String::new());
    for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"]
    {
        env.insert(key.into(), String::new());
    }
    env.insert("NO_PROXY".into(), "127.0.0.1,localhost".into());
    env.insert("no_proxy".into(), "127.0.0.1,localhost".into());
    let pure = profile.config().args.len() == 2;
    profile.overlay_environment(env)?;
    if !pure {
        profile.append_arguments(&["--pure".into()])?;
    }
    Ok(())
}

fn reject_managed_configuration(path: &Path) -> anyhow::Result<()> {
    match path.symlink_metadata() {
        Ok(_) => {
            anyhow::bail!("Kilocode administrator configuration requires explicit routing review")
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => anyhow::bail!("Kilocode administrator configuration cannot be verified"),
    }
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
            "insecure Kilocode state directory"
        );
        Ok(())
    }
    #[cfg(not(unix))]
    anyhow::bail!("Kilocode private state ACL verification unavailable")
}

#[cfg(test)]
#[path = "kilocode_tests.rs"]
mod tests;
