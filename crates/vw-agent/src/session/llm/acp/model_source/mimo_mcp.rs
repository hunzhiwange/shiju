//! Request-owned stdio MCP projection for sandboxed ACP backends.
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
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{process::Command, sync::Semaphore};

const TIMEOUT: Duration = Duration::from_secs(15);
const BODY_LIMIT: usize = 4 * 1024 * 1024;
const PREFIX: &str = "/shiju-mimo-mcp";
const VTCODE_PREFIX: &str = "/shiju-vtcode-mcp";
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
    approval_names: BTreeMap<String, String>,
    permits: Semaphore,
    _scratch: tempfile::TempDir,
    graph_root: Option<std::path::PathBuf>,
}

struct PendingApproval {
    tool: String,
    call_id: String,
    arguments: Value,
    issued: Instant,
    valid: acp_client::ApprovalValidity,
}

struct VtcodeApproval {
    require_call_id: bool,
    pending: Mutex<Option<PendingApproval>>,
}

impl VtcodeApproval {
    fn new(require_call_id: bool) -> Self {
        Self { require_call_id, pending: Mutex::new(None) }
    }

    fn issue(
        &self,
        name: &str,
        call_id: &str,
        input: &Value,
        valid: acp_client::ApprovalValidity,
    ) -> bool {
        if call_id.is_empty() {
            return false;
        }
        let Ok(mut current) = self.pending.lock() else { return false };
        *current = Some(PendingApproval {
            tool: name.to_owned(),
            call_id: call_id.to_owned(),
            arguments: input.clone(),
            issued: Instant::now(),
            valid,
        });
        true
    }

    fn consume(&self, name: &str, call_id: Option<&str>, input: &Value) -> bool {
        let Ok(mut current) = self.pending.lock() else { return false };
        let allowed = current.as_ref().is_some_and(|approval| {
            approval.tool == name
                && &approval.arguments == input
                && (!self.require_call_id || call_id == Some(approval.call_id.as_str()))
                && approval.issued.elapsed() <= TIMEOUT
                && (approval.valid)()
        });
        if allowed || !self.require_call_id {
            *current = None;
        }
        allowed
    }
}

struct Bridge {
    approvals: Option<Arc<VtcodeApproval>>,
    launch: Arc<RelayBinding>,
    access: Arc<RelayBinding>,
    servers: BTreeMap<String, Server>,
}

pub(super) struct Projection {
    pub(super) router: Router,
    servers: Vec<String>,
    access: Arc<RelayBinding>,
    pub(super) tools: Vec<String>,
    approvals: Option<Arc<VtcodeApproval>>,
}

impl Projection {
    pub(super) fn token(&self) -> &str {
        self.access.bearer_token()
    }

    pub(super) fn approval_config(&self, endpoint: &str) -> anyhow::Result<(
        Vec<AcpMcpServer>, acp_client::ManagedMcpApproval,
    )> {
        use agent_client_protocol::{HttpHeader, McpServerHttp};
        let approvals = self.approvals.clone().ok_or_else(|| anyhow::anyhow!("Missing managed MCP approval store"))?;
        let catalog = self.tools.clone();
        let approve: acp_client::ManagedMcpApproval = Arc::new(move |name, call_id, input, valid| {
            input.is_object() && catalog.iter().any(|tool| tool == name)
                && approvals.issue(name, call_id, input, valid)
        });
        let servers = self.servers.iter().map(|name| {
            AcpMcpServer::Http(McpServerHttp::new(name.as_str(), format!("{endpoint}{VTCODE_PREFIX}/{name}"))
                .headers(vec![HttpHeader::new("Authorization", format!("Bearer {}", self.token()))]))
        }).collect();
        Ok((servers, approve))
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
    let vtcode = launch.identity().session.ends_with(":vtcode");
    let hermes = launch.identity().session.ends_with(":hermes");
    let auggie = launch.identity().session.ends_with(":auggie");
    let droid = launch.identity().session.ends_with(":droid");
    let autohand = launch.identity().session.ends_with(":autohand");
    let requires_approval = vtcode || hermes || auggie || droid || autohand;
    anyhow::ensure!(
        requires_approval || launch.identity().session.ends_with(":mimo-code"),
        "Managed MCP requires a reviewed relay"
    );
    let approvals = requires_approval
        .then(|| Arc::new(VtcodeApproval::new(vtcode && cfg!(target_os = "linux"))));
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

    let policy_key = match (hermes, vtcode, auggie, droid) {
        _ if autohand => "shiju.autohand.toolPolicy",
        (_, _, _, true) => "shiju.droid.toolPolicy",
        (true, _, _, _) => "shiju.hermes.toolPolicy",
        (_, true, _, _) => "shiju.vtcode.toolPolicy",
        (_, _, true, _) => "shiju.auggie.toolPolicy",
        _ => "shiju.mimo.toolPolicy",
    };
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
            .and_then(|meta| meta.get(policy_key))
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
        let mut args = definition.args.clone();
        let graph_root = if vtcode && executable.file_name().is_some_and(|name| name == "rustcodegraph") {
            let [verb, transport, path_flag, root, watch] = args.as_mut_slice() else {
                anyhow::bail!("VT Code requires read-only RustCodeGraph stdio arguments");
            };
            anyhow::ensure!(verb.as_str() == "serve" && transport.as_str() == "--mcp" && path_flag.as_str() == "--path"
                && watch.as_str() == "--no-watch" && FsPath::new(root.as_str()).canonicalize()? == workspace,
                "Invalid VT Code RustCodeGraph workspace arguments");
            prepare_codegraph_workspace(&workspace, &scratch_path)?;
            *root = scratch_path.to_str().ok_or_else(|| anyhow::anyhow!("Non-UTF-8 graph workspace"))?.to_owned();
            Some(scratch_path.clone())
        } else {
            None
        };
        let mut command = Command::new("/usr/bin/sandbox-exec");
        for (key, value) in [
            ("command", executable.clone()),
            ("workspace", workspace.clone()),
            ("scratch", scratch_path.clone()),
            ("host-home", host_home),
            ("git", workspace.join(".git")),
            ("settings", workspace.join(if droid { ".factory" } else if auggie { ".augment" } else if hermes { ".hermes" } else if vtcode { ".vtcode" } else if autohand { ".autohand" } else { ".mimocode" })),
            ("env", workspace.join(".env")),
            ("mount", workspace.join(".vwacprc.json")),
        ] {
            let value = value.to_str().ok_or_else(|| anyhow::anyhow!("non-UTF-8 MCP path"))?;
            command.arg("-D").arg(format!("{key}={value}"));
        }
        command
            .args(["-p", SANDBOX])
            .arg(executable)
            .args(&args)
            .current_dir(graph_root.as_deref().unwrap_or(&workspace))
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
        let mut approval_names = BTreeMap::new();
        for mut tool in listed {
            let name = tool.name.to_string();
            anyhow::ensure!(valid_name(&name), "invalid MiMo MCP tool name");
            let tool_id = tool_mcp::mcp_tool::mcp_tool_id(&definition.name, &name)?;
            if !mcp_client::mcp_tool_allowed(&management, &name, &tool_id) {
                continue;
            }
            let public = if droid {
                acp_client::droid_mcp_identity(&definition.name, &name)
            } else if auggie {
                // Match Auggie's tool_server identity and reject collisions below.
                let mut alias = format!("{name}_{}", definition.name);
                alias.truncate(64);
                format!("mcp__auggie__{alias}")
            } else if hermes {
                format!("mcp__{}__{name}", definition.name).replace('-', "_")
            } else if vtcode || autohand {
                format!("mcp__{}__{name}", definition.name)
            } else {
                format!("{}_{name}", definition.name)
            };
            if requires_approval {
                approval_names.insert(name.clone(), public.clone());
            }
            anyhow::ensure!(
                (auggie || public.len() <= 64) && native_tools.insert(public),
                "invalid or colliding MiMo MCP tool name"
            );
            if graph_root.is_some() {
                if let Some(properties) = Arc::make_mut(&mut tool.input_schema)
                    .get_mut("properties").and_then(Value::as_object_mut)
                {
                    if let Some(project) = properties.get_mut("projectPath").and_then(Value::as_object_mut) {
                        project.insert("description".into(), json!("Optional. Omit to use this indexed workspace root. Nested packages are not independently indexed."));
                        project.insert("enum".into(), json!(["."]));
                    }
                    if name == "rustcodegraph_node" {
                        properties.entry("offset").or_insert_with(|| json!({"type":"integer","description":"Starting line of an existing file (1-based)."}));
                        properties.entry("limit").or_insert_with(|| json!({"type":"integer","description":"Maximum source lines returned."}));
                    }
                }
            }
            catalog.insert(name, tool);
        }
        if !catalog.is_empty() {
            servers.insert(
                definition.name.clone(),
                Server { peer, catalog, approval_names, permits: Semaphore::new(1), _scratch: scratch, graph_root },
            );
        }
    }
    anyhow::ensure!(!requires_approval || !servers.is_empty(), "No tools in the selected ACP MCP catalog");
    let names = servers.keys().cloned().collect();
    let state = Arc::new(Bridge { launch, access: access.clone(), servers, approvals: approvals.clone() });
    let route = if requires_approval { "/shiju-vtcode-mcp/{service}" } else { "/shiju-mimo-mcp/{service}" };
    let router = Router::new().route(route, post(handle)).with_state(state);
    Ok(Projection { router, servers: names, access, tools: native_tools.into_iter().collect(), approvals })
}

#[cfg(unix)]
fn prepare_codegraph_workspace(workspace: &FsPath, scratch: &FsPath) -> anyhow::Result<()> {
    use std::os::unix::fs::{OpenOptionsExt, symlink};
    let index = workspace.join(".rustcodegraph");
    anyhow::ensure!(index.canonicalize()? == index
        && !index.join("rustcodegraph.db-wal").metadata().is_ok_and(|entry| entry.len() != 0),
        "VT Code requires a checkpointed RustCodeGraph index");
    let mut database = std::fs::OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW)
        .open(index.join("rustcodegraph.db"))?;
    anyhow::ensure!(database.metadata()?.is_file(), "Invalid RustCodeGraph database");
    let private_index = scratch.join(".rustcodegraph");
    std::fs::create_dir(&private_index)?;
    std::io::copy(&mut database, &mut std::fs::File::create(private_index.join("rustcodegraph.db"))?)?;
    for entry in std::fs::read_dir(workspace)? {
        let entry = entry?;
        if entry.file_name() != ".rustcodegraph" {
            symlink(entry.path(), scratch.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn prepare_codegraph_workspace(_: &FsPath, _: &FsPath) -> anyhow::Result<()> {
    anyhow::bail!("VT Code MCP requires Darwin arm64")
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
        response = tokio::time::timeout(TIMEOUT, dispatch(server, state.approvals.as_deref(), request)) => {
            response.map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
        }
    }
}

async fn dispatch(server: &Server, approval: Option<&VtcodeApproval>, request: Request) -> Result<Response, StatusCode> {
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
            let mut params: CallToolRequestParam = serde_json::from_value(message["params"].clone())
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            if !server.catalog.contains_key(params.name.as_ref()) {
                return Ok(Json(json!({"jsonrpc":"2.0","id":id,"error":{"code":-32602,"message":"Tool is outside the session catalog"}})).into_response());
            }
            if let Some(approval) = approval {
                if approval.require_call_id && params.arguments.is_none() {
                    return Err(StatusCode::FORBIDDEN);
                }
                let input = params.arguments.as_ref().map_or_else(|| json!({}), |args| json!(args));
                // The bearer-scoped service and catalog select the source. The
                // host-private call ID is consumed here and never sent to stdio.
                let public = server.approval_names.get(params.name.as_ref()).ok_or(StatusCode::FORBIDDEN)?;
                let call_id = message["params"]["_meta"]["shijuCallId"].as_str();
                if !approval.consume(public, call_id, &input) {
                    return Err(StatusCode::FORBIDDEN);
                }
                if let Some(root) = &server.graph_root
                    && let Some(project) = params.arguments.as_mut().and_then(|args| args.get_mut("projectPath"))
                {
                    if project.as_str() != Some(".") {
                        return Err(StatusCode::BAD_REQUEST);
                    }
                    *project = json!(root);
                }
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

#[cfg(all(test, any(target_os = "linux", all(target_os = "macos", target_arch = "aarch64"))))]
#[path = "mimo_mcp_tests.rs"]
mod tests;
