//! Request-owned stdio MCP projection for the sandboxed MiMo CLI.
use acp_types::AcpMcpServer;
use axum::{
    Json, Router,
    body::to_bytes,
    extract::{Path, Request, State},
    http::{StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
    routing::post,
};
use model_transport::RelayBinding;
use rmcp::model::{CallToolRequestParam, Tool};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path as FsPath,
    sync::Arc,
    time::Duration,
};
use tokio::{process::Command, sync::Semaphore};

const TIMEOUT: Duration = Duration::from_secs(15);
const BODY_LIMIT: usize = 4 * 1024 * 1024;
const PREFIX: &str = "/shiju-mimo-mcp";
const SANDBOX: &str = r#"
(version 1)
(allow default)
(deny file-read-data (subpath "/Users") (subpath "/private/tmp") (subpath "/Volumes")
    (subpath "/private/var/folders") (subpath (param "host-home")))
(allow file-read-data (literal (param "command")) (subpath (param "workspace"))
    (subpath (param "scratch")))
(deny file-read-data (subpath (param "git")) (subpath (param "settings"))
    (literal (param "env")) (literal (param "mount")))
(deny file-write*)
(allow file-write* (subpath (param "scratch")) (literal "/dev/null"))
(deny process-fork process-exec network*)
(allow process-exec (literal (param "command")))
"#;

struct Server {
    peer: mcp_client::RunningMcpPeer,
    catalog: BTreeMap<String, Tool>,
    permits: Semaphore,
    _scratch: tempfile::TempDir,
}

struct Bridge {
    launch: Arc<RelayBinding>,
    access: Arc<RelayBinding>,
    servers: BTreeMap<String, Server>,
}

pub(super) struct Projection {
    pub(super) router: Router,
    servers: Vec<String>,
    access: Arc<RelayBinding>,
    pub(super) tools: Vec<String>,
}

impl Projection {
    pub(super) fn token(&self) -> &str {
        self.access.bearer_token()
    }

    pub(super) fn native_config(&self, endpoint: &str) -> Map<String, Value> {
        self.servers
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    json!({
                        "type": "remote",
                        "url": format!("{endpoint}{PREFIX}/{name}"),
                        "headers": {"Authorization": "Bearer {env:SHIJU_MIMO_MCP_TOKEN}"},
                        "oauth": false
                    }),
                )
            })
            .collect()
    }
}

pub(super) async fn prepare(
    definitions: &[AcpMcpServer],
    directory: &str,
    launch: Arc<RelayBinding>,
) -> anyhow::Result<Projection> {
    anyhow::ensure!(
        cfg!(all(target_os = "macos", target_arch = "aarch64")),
        "MiMo MCP requires the reviewed Darwin arm64 runtime"
    );
    anyhow::ensure!(
        launch.identity().session.ends_with(":mimo-code"),
        "MiMo MCP requires a MiMo relay"
    );
    anyhow::ensure!(
        !definitions.is_empty() && definitions.len() <= 16,
        "invalid MiMo MCP service count"
    );
    let workspace = FsPath::new(directory).canonicalize()?;
    anyhow::ensure!(
        workspace.is_dir() && workspace.parent().is_some(),
        "invalid MiMo MCP workspace"
    );
    let access = RelayBinding::new(launch.identity().clone(), Duration::from_secs(900))?;
    let mut servers = BTreeMap::new();
    let mut services = BTreeSet::new();
    let mut native_tools = BTreeSet::new();

    for definition in definitions {
        let AcpMcpServer::Stdio(definition) = definition else {
            anyhow::bail!("MiMo managed MCP accepts isolated stdio services");
        };
        anyhow::ensure!(
            valid_name(&definition.name) && services.insert(definition.name.as_str()),
            "invalid or duplicate MiMo MCP service name"
        );
        anyhow::ensure!(
            definition.env.is_empty(),
            "MiMo managed MCP does not accept stdio environment overrides"
        );
        let policy = definition
            .meta
            .as_ref()
            .and_then(|meta| meta.get("shiju.mimo.toolPolicy"))
            .ok_or_else(|| {
                anyhow::anyhow!("MiMo MCP requires a host-selected service and tool policy")
            })?;
        let management = acp_config::McpServerManagement {
            allowed_tools: serde_json::from_value(policy["allow"].clone())?,
            denied_tools: serde_json::from_value(policy["deny"].clone())?,
            ..acp_config::McpServerManagement::default_for_server(&definition.name)
        };
        let executable = FsPath::new(&definition.command);
        anyhow::ensure!(executable.is_absolute(), "MiMo MCP requires an absolute stdio command");
        let executable = executable.canonicalize()?;
        anyhow::ensure!(executable.is_file(), "invalid MiMo MCP executable");
        let scratch = tempfile::Builder::new().prefix("shiju-mimo-mcp-").tempdir()?;
        let scratch_path = scratch.path().canonicalize()?;
        let host_home = std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("missing host home"))?
            .canonicalize()?;
        anyhow::ensure!(
            !host_home.starts_with(&workspace),
            "MiMo MCP workspace overlaps host home"
        );
        let mut command = Command::new("/usr/bin/sandbox-exec");
        for (key, value) in [
            ("command", executable.clone()),
            ("workspace", workspace.clone()),
            ("scratch", scratch_path.clone()),
            ("host-home", host_home),
            ("git", workspace.join(".git")),
            ("settings", workspace.join(".mimocode")),
            ("env", workspace.join(".env")),
            ("mount", workspace.join(".vwacprc.json")),
        ] {
            let value = value.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 MCP path"))?;
            command.arg("-D").arg(format!("{key}={value}"));
        }
        command
            .args(["-p", SANDBOX])
            .arg(executable)
            .args(&definition.args)
            .current_dir(&workspace)
            .env_clear()
            .env("HOME", &scratch_path)
            .env("TMPDIR", &scratch_path)
            .env("LANG", "en_US.UTF-8");

        let peer = mcp_client::client::open_isolated_stdio_peer(
            command,
            TIMEOUT,
            launch.cancelled().clone(),
        )
        .await?;
        let listed =
            tokio::time::timeout(TIMEOUT, mcp_client::catalog::list_tools(&peer)).await??;
        let mut catalog = BTreeMap::new();
        for tool in listed {
            let name = tool.name.to_string();
            anyhow::ensure!(valid_name(&name), "invalid MiMo MCP tool name");
            let tool_id = tool_mcp::mcp_tool::mcp_tool_id(&definition.name, &name)?;
            if !mcp_client::mcp_tool_allowed(&management, &name, &tool_id) {
                continue;
            }
            let public = format!("{}_{name}", definition.name);
            anyhow::ensure!(
                public.len() <= 64 && native_tools.insert(public),
                "invalid or colliding MiMo MCP tool name"
            );
            catalog.insert(name, tool);
        }
        if !catalog.is_empty() {
            servers.insert(
                definition.name.clone(),
                Server { peer, catalog, permits: Semaphore::new(1), _scratch: scratch },
            );
        }
    }
    let names = servers.keys().cloned().collect();
    let state = Arc::new(Bridge { launch, access: access.clone(), servers });
    let router = Router::new().route("/shiju-mimo-mcp/{service}", post(handle)).with_state(state);
    Ok(Projection { router, servers: names, access, tools: native_tools.into_iter().collect() })
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        && name.bytes().any(|b| b.is_ascii_alphanumeric())
}

async fn handle(
    State(state): State<Arc<Bridge>>,
    Path(service): Path<String>,
    request: Request,
) -> Result<Response, StatusCode> {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !state.access.accepts(token, state.launch.identity())
        || state.launch.cancelled().is_cancelled()
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    if request.uri().query().is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let server = state.servers.get(&service).ok_or(StatusCode::NOT_FOUND)?;
    let _permit = server.permits.try_acquire().map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    tokio::select! {
        biased;
        () = state.launch.cancelled().cancelled() => Err(StatusCode::UNAUTHORIZED),
        response = tokio::time::timeout(TIMEOUT, dispatch(server, request)) => {
            response.map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
        }
    }
}

async fn dispatch(server: &Server, request: Request) -> Result<Response, StatusCode> {
    let bytes = to_bytes(request.into_body(), BODY_LIMIT)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
    let message: Value = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    if message["jsonrpc"] != "2.0" {
        return Err(StatusCode::BAD_REQUEST);
    }
    let method = message["method"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    if method == "notifications/initialized" && message.get("id").is_none() {
        return Ok(StatusCode::ACCEPTED.into_response());
    }
    let id = message
        .get("id")
        .filter(|id| id.is_string() || id.is_i64())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let result = match method {
        "initialize" => json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {"tools":{}},
            "serverInfo": {"name":"shiju-mimo-mcp", "version":"1"}
        }),
        "tools/list" => json!({"tools": server.catalog.values().collect::<Vec<_>>()}),
        "tools/call" => {
            let params: CallToolRequestParam = serde_json::from_value(message["params"].clone())
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            if !server.catalog.contains_key(params.name.as_ref()) {
                return Ok(Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":"Tool is outside the session catalog"}})).into_response());
            }
            match server.peer.call_tool(params).await {
                Ok(result) => serde_json::to_value(result).map_err(|_| StatusCode::BAD_GATEWAY)?,
                Err(_) => return Ok(Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":"MCP tool execution failed"}})).into_response()),
            }
        }
        _ => return Ok(Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Unsupported MCP method"}})).into_response()),
    };
    let body = serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"result":result}))
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if body.len() > BODY_LIMIT {
        return Err(StatusCode::BAD_GATEWAY);
    }
    Ok(([(axum::http::header::CONTENT_TYPE, "application/json")], body).into_response())
}

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
#[path = "mimo_mcp_tests.rs"]
mod tests;
