use super::AcpLaunchProfile;
use acp_types::PermissionMode;
use agent_client_protocol::McpServer;
use serde_json::{Map, Value, json};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use vw_api_acp_model::acp_model_source::AcpModelSource;

/// OpenClaw 2026.10.10 ACP is only a bridge. Put the provider on an owned
/// foreground Gateway, and bind the bridge explicitly to its fresh loopback port.
pub fn configure(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    workspace: &str,
) -> anyhow::Result<()> {
    configure_with_mcp(profile, endpoint, workspace, &Map::new())
}

pub(crate) fn configure_with_mcp(
    profile: &mut AcpLaunchProfile,
    endpoint: &str,
    workspace: &str,
    mcp: &Map<String, Value>,
) -> anyhow::Result<()> {
    if !mcp.is_empty() {
        acp_agent_registry::verify_openclaw_mcp_executable(&profile.config().command)
            .map_err(anyhow::Error::msg)?;
    }
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
    let mut config = json!({
        "gateway": {"mode":"local", "bind":"loopback", "port":address.port(),
            "auth":{"mode":"token","token":"${VW_OPENCLAW_GATEWAY_TOKEN}"},
            "controlUi":{"enabled":false}},
        "session":{"store":state.join("{agentId}/sessions.json")},
        "discovery":{"mdns":{"mode":"off"}}, "browser":{"enabled":false},
        "plugins":{"enabled":false},
        "agents":{"defaults":{"workspace": workspace,
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
    if !mcp.is_empty() {
        config["mcp"] = json!({"servers": mcp});
        config["tools"] = json!({"toolSearch": false});
    }
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
    let mut command = shell_command::std_command(&profile.config().command);
    command.args(["gateway", "run", "--port", &address.port().to_string(), "--bind", "loopback"]);
    command.env_clear();
    for key in ["PATH", "HOME", "TMPDIR", "SystemRoot"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    // The Gateway uses the same runtime PATH as the ACP bridge, including
    // interpreters required by native CLI launch scripts in desktop sessions.
    shell_command::apply_augmented_path_std_command(&mut command);
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
    let identity = profile.binding.identity();
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

/// The private Gateway executes native MCP tools without ACP permission callbacks.
/// Only an already unrestricted host request can move its selected servers there.
/// ACP's session tool inventory is sent as Claude metadata. OpenClaw's service
/// selection comes from the host-resolved MCP defaults.
pub(crate) fn project_mcp<S>(
    options: &crate::acp::options::ParsedAcpOptions<S>,
    source: AcpModelSource,
) -> anyhow::Result<Map<String, Value>> {
    let mut servers = Map::new();
    if options.mcp_servers.is_empty() {
        return Ok(servers);
    }
    anyhow::ensure!(
        source == AcpModelSource::SessionModel,
        "OpenClaw MCP requires an owned session-model Gateway"
    );
    anyhow::ensure!(
        options.full_access
            && matches!(options.permission_mode, None | Some(PermissionMode::ApproveAll))
            && options.non_interactive_permissions.is_none(),
        "OpenClaw native MCP requires full access without non-interactive approval restrictions"
    );
    for server in &options.mcp_servers {
        let McpServer::Stdio(server) = server else {
            anyhow::bail!("OpenClaw managed MCP supports only stdio");
        };
        let command = server.command.to_str().filter(|command| literal_mcp_value(command));
        anyhow::ensure!(
            command.is_some()
                && server.command.is_absolute()
                && server.args.iter().all(|arg| literal_mcp_value(arg))
                && server.env.is_empty(),
            "OpenClaw managed stdio MCP requires a literal absolute command and no environment overrides"
        );
        anyhow::ensure!(
            !server.name.is_empty()
                && server
                    .name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')),
            "OpenClaw managed MCP server name is invalid"
        );
        let entry = json!({"transport":"stdio", "command":command, "args":server.args});
        anyhow::ensure!(
            servers.insert(server.name.clone(), entry).is_none(),
            "OpenClaw managed MCP server names must be unique"
        );
    }
    Ok(servers)
}

fn literal_mcp_value(value: &str) -> bool {
    !value.contains('\0') && !value.contains("${")
}
