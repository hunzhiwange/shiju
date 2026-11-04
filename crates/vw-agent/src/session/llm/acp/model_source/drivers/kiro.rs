use super::AcpLaunchProfile;
use serde_json::json;
use std::{collections::HashMap, io::Write, path::Path};

/// The pinned 3.0.0 native V3 engine uses its existing Responses transport.
/// Keep durable history separate from the request-owned bearer capability.
pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(profile.config().args == ["acp"], "Kiro requires plain native ACP launch");
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| {
            let key = key.to_ascii_uppercase();
            key.starts_with("KIRO_")
                || key.starts_with("KAS_")
                || key.starts_with("ASBX_")
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
        "conflicting Kiro model environment"
    );
    for key in [
        "KIRO_KAS_SERVER_PATH",
        "KIRO_KAS_NODE_PATH",
        "KAS_BUNDLE_PATH",
        "KIRO_AGENT_CONFIG_DIR",
        "ASBX_KIRO_MANDATORY_MCPS",
    ] {
        anyhow::ensure!(std::env::var_os(key).is_none(), "unverified Kiro runtime override");
    }
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("invalid Kiro relay endpoint"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "Kiro requires an owned loopback relay"
    );
    let api = profile
        .model_api
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Kiro requires selected model transport"))?;
    anyhow::ensure!(
        matches!(api.adapter.as_str(), "openai" | "openai-compatible" | "openai-responses"),
        "Kiro selected model transport is unsupported"
    );
    anyhow::ensure!(
        profile.model_supports_tool_call && !profile.model_alias.is_empty(),
        "Kiro requires a tool-capable selected model"
    );
    let root = profile.state_root.as_deref().unwrap_or(profile.directory());
    anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid Kiro state root");
    use sha2::Digest;
    let identity = &profile.binding.identity;
    let reference = hex::encode(sha2::Sha256::digest(serde_json::to_vec(&json!({
        "tenant":identity.tenant,"project":identity.project,"session":identity.session,
    }))?));
    let mut home = root.to_path_buf();
    for component in ["acp-kiro-state", &reference] {
        home.push(component);
        private_directory(&home)?;
    }
    let settings_dir = home.join("settings");
    private_directory(&settings_dir)?;
    let runtime = root.join("acp-kiro-runtime");
    private_directory(&runtime)?;
    let path = settings_dir.join("cli.json");
    if let Ok(metadata) = path.symlink_metadata() {
        anyhow::ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "unsafe Kiro settings file"
        );
    }
    let settings = json!({"chat.defaultModel":profile.model_alias,"acp.agentEngine":"v3",
        "telemetry.enabled":false,"codeWhisperer.shareCodeWhispererContentWithAWS":false,
        "chat.enableAutoAgentUpgrade":false});
    let mut file = tempfile::NamedTempFile::new_in(&settings_dir)?;
    file.write_all(&serde_json::to_vec(&settings)?)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|_| anyhow::anyhow!("cannot publish Kiro settings"))?;
    let mut environment = HashMap::from([
        ("KIRO_HOME".into(), home.to_string_lossy().into_owned()),
        ("KIRO_RUNTIME_DIR".into(), runtime.to_string_lossy().into_owned()),
        ("KIRO_KAS_ENDPOINT".into(), endpoint.into()),
        ("KIRO_KAS_CONTROL_PLANE_ENDPOINT".into(), endpoint.into()),
        ("KIRO_KAS_REGION".into(), "us-east-1".into()),
        ("KIRO_API_KEY".into(), profile.binding.bearer_token().into()),
        ("KIRO_NO_AUTO_UPDATE".into(), "1".into()),
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
    profile.append_arguments(&[
        "--agent-engine".into(),
        "v3".into(),
        "--auth-method".into(),
        "cli".into(),
    ])?;
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
            "insecure Kiro directory"
        );
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        anyhow::bail!("Kiro private directory ACL verification unavailable")
    }
}

#[cfg(all(test, unix))]
#[path = "kiro_tests.rs"]
mod tests;
