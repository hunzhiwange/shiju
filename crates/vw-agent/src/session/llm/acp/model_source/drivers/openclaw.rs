use super::AcpLaunchProfile;
use serde_json::json;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// OpenClaw 2026.10.10 ACP is only a bridge. Put the provider on an owned
/// foreground Gateway, and bind the bridge explicitly to its fresh loopback port.
pub(super) fn configure(profile: &mut AcpLaunchProfile, endpoint: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        profile.config().args == ["acp"],
        "OpenClaw requires native ACP without Gateway overrides"
    );
    anyhow::ensure!(
        Path::new(&profile.config().command).file_name().is_some_and(|name| name == "openclaw"),
        "OpenClaw requires the installed native executable"
    );
    anyhow::ensure!(
        !profile.config().env.keys().any(|key| key.to_ascii_uppercase().starts_with("OPENCLAW_")
            || key == "NODE_OPTIONS"
            || key.starts_with("VW_OPENCLAW_")),
        "conflicting OpenClaw configuration"
    );
    let url = reqwest::Url::parse(endpoint)
        .map_err(|_| anyhow::anyhow!("invalid OpenClaw model relay"))?;
    anyhow::ensure!(
        url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none(),
        "OpenClaw requires an owned loopback relay"
    );
    anyhow::ensure!(
        !profile.model_alias.is_empty() && !profile.model_alias.contains(['\0', '$', '{', '}']),
        "OpenClaw requires a literal selected model"
    );
    let state = state_directory(profile)?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| anyhow::anyhow!("OpenClaw Gateway token entropy unavailable"))?;
    let gateway_token = hex::encode(bytes);
    let token_file = profile.write_config("gateway-token", gateway_token.as_bytes())?;
    let config = json!({
        "gateway": {"mode":"local", "bind":"loopback", "port":address.port(),
            "auth":{"mode":"token","token":"${VW_OPENCLAW_GATEWAY_TOKEN}"},
            "controlUi":{"enabled":false}},
        "session":{"store":state.join("{agentId}/sessions.json")},
        "discovery":{"mdns":{"mode":"off"}}, "browser":{"enabled":false},
        "plugins":{"enabled":false},
        "agents":{"defaults":{"workspace": crate::project::instance::directory(),
            "model":{"primary":format!("session-provider/{}",profile.model_alias),"fallbacks":[]},
            "imageModel":{"primary":format!("session-provider/{}",profile.model_alias),"fallbacks":[]},
            "subagents":{"model":format!("session-provider/{}",profile.model_alias)},
            "heartbeat":{"every":"0m"}, "skipBootstrap":true}},
        "models":{"mode":"replace","providers":{"session-provider":{
            "baseUrl":format!("{endpoint}/v1"), "apiKey":"${VW_OPENCLAW_RELAY_TOKEN}",
            "api":"openai-completions", "models":[{"id":profile.model_alias,"name":"Session model",
                "contextWindow":profile.model_context_limit.max(16000), "maxTokens":profile.model_output_limit.max(1),
                "reasoning":false,"input":["text"]}]
        }}}
    });
    let config_file = profile.write_config("openclaw.json", &serde_json::to_vec(&config)?)?;
    profile.overlay_environment(HashMap::from([
        ("OPENCLAW_STATE_DIR".into(), profile.directory().to_string_lossy().into_owned()),
        ("OPENCLAW_CONFIG_PATH".into(), config_file.to_string_lossy().into_owned()),
        ("OPENCLAW_SKIP_CHANNELS".into(), "1".into()),
        ("OPENCLAW_SKIP_CANVAS_HOST".into(), "1".into()),
        ("NODE_OPTIONS".into(), String::new()),
        ("VW_OPENCLAW_GATEWAY_TOKEN".into(), gateway_token),
        ("VW_OPENCLAW_RELAY_TOKEN".into(), profile.binding.bearer_token().to_owned()),
    ]))?;
    profile.append_arguments(&[
        "--url".into(),
        format!("ws://{address}"),
        "--token-file".into(),
        token_file.to_string_lossy().into_owned(),
        "--no-prefix-cwd".into(),
        "--session".into(),
        format!("agent:main:acp-{}", state.file_name().unwrap().to_string_lossy()),
    ])?;
    let mut command = vw_shared::shell::std_command(&profile.config().command);
    command.args(["gateway", "run", "--port", &address.port().to_string(), "--bind", "loopback"]);
    command.env_clear();
    for key in ["PATH", "HOME", "TMPDIR", "SystemRoot"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    // The Gateway uses the same runtime PATH as the ACP bridge, including
    // interpreters required by native CLI launch scripts in desktop sessions.
    vw_shared::shell::apply_augmented_path_std_command(&mut command);
    command.envs(profile.config().env.iter().filter(|(key, _)| {
        key.starts_with("OPENCLAW_")
            || key.starts_with("VW_OPENCLAW_")
            || key.as_str() == "SHIJU_OPENCLAW_SKILL_ROOT"
            || key.as_str() == "NODE_OPTIONS"
    }));
    drop(listener); // OpenClaw does not accept an inherited listening socket.
    profile.gateway = Some(super::super::gateway::OwnedGateway::new(command, address));
    Ok(())
}

fn state_directory(profile: &AcpLaunchProfile) -> anyhow::Result<PathBuf> {
    let root = profile.state_root.as_deref().unwrap_or(profile.directory());
    anyhow::ensure!(root.is_absolute() && root.is_dir(), "invalid OpenClaw state root");
    use sha2::Digest;
    let identity = &profile.binding.identity;
    let reference = hex::encode(sha2::Sha256::digest(serde_json::to_vec(&json!({
        "tenant":identity.tenant,"project":identity.project,"session":identity.session,"source":"session_model"
    }))?));
    let mut path = root.to_path_buf();
    for part in ["acp-openclaw-state", &reference] {
        path.push(part);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
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
                "insecure OpenClaw state directory"
            );
        }
        #[cfg(not(unix))]
        anyhow::bail!("OpenClaw private state ACL verification unavailable");
    }
    Ok(path)
}

#[cfg(test)]
#[path = "openclaw_tests.rs"]
mod tests;
