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
    launch: Arc<RelayBinding>,
    linux_owner: Option<LinuxMcpOwner>,
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
        let iflow_source = match self.linux_owner {
            Some(LinuxMcpOwner::IFlow(source)) => Some(source),
            _ => None,
        };
        let glm = matches!(self.linux_owner, Some(LinuxMcpOwner::Glm(_)));
        if iflow_source.is_some() || glm {
            let address = endpoint.strip_prefix("http://")
                .and_then(|value| value.parse::<std::net::SocketAddr>().ok())
                .filter(|address| match address.ip() {
                    std::net::IpAddr::V4(ip) => ip == std::net::Ipv4Addr::LOCALHOST,
                    std::net::IpAddr::V6(ip) => !glm && ip == std::net::Ipv6Addr::LOCALHOST,
                })
                .filter(|address| !matches!(address.port(), 0 | 80));
            anyhow::ensure!(
                address.is_some_and(|address| endpoint == format!("http://{address}")),
                "Managed MCP requires the exact private loopback origin"
            );
        }
        let approvals = self.approvals.clone().ok_or_else(|| anyhow::anyhow!("Missing managed MCP approval store"))?;
        let catalog = self.tools.clone();
        let launch = self.launch.clone();
        let access = self.access.clone();
        let approve: acp_client::ManagedMcpApproval = Arc::new(move |name, call_id, input, valid| {
            !launch.cancelled().is_cancelled()
                && access.expires().is_none_or(|expires| Instant::now() < expires)
                && input.is_object() && catalog.iter().any(|tool| tool == name)
                && approvals.issue(name, call_id, input, valid)
        });
        let servers = self.servers.iter().map(|name| {
            let mut server = McpServerHttp::new(name.as_str(), format!("{endpoint}{VTCODE_PREFIX}/{name}"))
                .headers(vec![HttpHeader::new("Authorization", format!("Bearer {}", self.token()))]);
            if let Some(source) = iflow_source {
                let source = match source {
                    LinuxMcpSource::Project => "project",
                    LinuxMcpSource::Mounted => "mounted",
                };
                server.meta = Some(Map::from_iter([("shiju.iflow.mcp-v4".into(), json!(source))]));
            }
            // GLM v5 has no source field on its wire. The launch-scoped catalog
            // and bearer carry the host-reviewed source through execution.
            AcpMcpServer::Http(server)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LinuxMcpOwner {
    VtCode,
    IFlow(LinuxMcpSource),
    Glm(LinuxMcpSource),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LinuxMcpSource {
    Project,
    Mounted,
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) type IFlowMcpSource = LinuxMcpSource;

/// A pin from the host's reviewed service catalog. The ACP mount is only a
/// selection and cannot supply this pin or choose the isolated launcher.
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) struct ReviewedLinuxStdio<'a> {
    pub(super) owner: LinuxMcpOwner,
    pub(super) executable: &'a FsPath,
    pub(super) sha256: [u8; 32],
    pub(super) args: &'a [String],
    pub(super) policy: &'a acp_config::McpServerManagement,
}

/// Candidate entry point for a reviewed GNU ACP launch. The registry and
/// adapter still have to supply a reviewed selection and enable their route.
#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
pub(super) async fn prepare_linux_stdio(
    selected: &[AcpMcpServer],
    reviewed: ReviewedLinuxStdio<'_>,
    launch: Arc<RelayBinding>,
) -> anyhow::Result<Projection> {
    prepare_linux_stdio_with(selected, reviewed, launch, |executable, args| {
        let args = args.iter().map(|arg| std::ffi::OsString::from(arg.as_str())).collect::<Vec<_>>();
        let command = process_sandbox::bubblewrap::isolated_stdio_command(executable, &args)?;
        Ok(Command::from(command))
    }).await
}

#[cfg(all(target_os = "linux", target_env = "gnu", any(target_arch = "x86_64", target_arch = "aarch64")))]
async fn prepare_linux_stdio_with(
    selected: &[AcpMcpServer],
    reviewed: ReviewedLinuxStdio<'_>,
    launch: Arc<RelayBinding>,
    isolate: impl FnOnce(&FsPath, &[String]) -> anyhow::Result<Command>,
) -> anyhow::Result<Projection> {
    use sha2::{Digest, Sha256};
    use std::{io::{Read, Write}, os::unix::fs::OpenOptionsExt};

    let backend = match reviewed.owner {
        LinuxMcpOwner::VtCode => "vtcode",
        LinuxMcpOwner::IFlow(_) => "iflow",
        LinuxMcpOwner::Glm(_) => "glm-acp-agent",
    };
    anyhow::ensure!(
        launch.identity().session.rsplit_once(':').is_some_and(|(_, owner)| owner == backend),
        "Linux MCP projection belongs to another adapter"
    );
    let [AcpMcpServer::Stdio(service)] = selected else {
        anyhow::bail!("Linux ACP requires one explicitly selected stdio service");
    };
    let policy = reviewed.policy;
    anyhow::ensure!(
        policy.enabled && !policy.allowed_tools.is_empty()
            && valid_name(&service.name) && service.name == policy.name
            && service.env.is_empty()
            && service.command.as_path() == reviewed.executable
            && service.args.as_slice() == reviewed.args,
        "Linux ACP selection differs from the reviewed service"
    );
    anyhow::ensure!(
        reviewed.executable.is_absolute() && reviewed.executable.canonicalize()? == reviewed.executable,
        "Linux ACP service requires a canonical executable"
    );
    anyhow::ensure!(!launch.cancelled().is_cancelled(), "Linux ACP projection was revoked");
    let scratch = tempfile::Builder::new().prefix("shiju-vtcode-mcp-").tempdir()?;
    let entry = scratch.path().canonicalize()?.join("entry");
    // Hash the same bytes written to the private entry that Bubblewrap opens.
    let mut copy = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o500).open(&entry)?;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(reviewed.executable)?;
    anyhow::ensure!(file.metadata()?.is_file(), "Linux ACP service must be a regular file");
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let size = file.read(&mut buffer)?;
        if size == 0 { break; }
        hash.update(&buffer[..size]);
        copy.write_all(&buffer[..size])?;
    }
    let actual: [u8; 32] = hash.finalize().into();
    anyhow::ensure!(
        actual == reviewed.sha256,
        "Linux ACP service checksum mismatch"
    );

    drop(copy);
    drop(file);
    anyhow::ensure!(!launch.cancelled().is_cancelled(), "Linux ACP projection was revoked");
    let command = isolate(&entry, reviewed.args)?;
    let (peer, listed) = discover_stdio(command, &launch).await?;
    let mut catalog = BTreeMap::new();
    let mut approval_names = BTreeMap::new();
    for tool in listed {
        let name = tool.name.to_string();
        anyhow::ensure!(valid_name(&name), "Invalid Linux ACP tool name");
        let tool_id = tool_mcp::mcp_tool::mcp_tool_id(&service.name, &name)?;
        if mcp_client::mcp_tool_allowed(policy, &name, &tool_id) {
            let public = if matches!(reviewed.owner, LinuxMcpOwner::Glm(_)) {
                llm_runtime::acp::model_source::drivers::glm_acp_agent::projected_mcp_tool_name(
                    &service.name, &name,
                )?
            } else {
                format!("mcp__{}__{name}", service.name)
            };
            anyhow::ensure!(
                public.len() <= 64 && !catalog.contains_key(&name),
                "Invalid or duplicate Linux ACP tool"
            );
            approval_names.insert(name.clone(), public);
            catalog.insert(name, tool);
        }
    }
    let tools = approval_names.values().cloned().collect::<BTreeSet<_>>();
    let servers = BTreeMap::from([(
        service.name.clone(),
        Server { peer, catalog, approval_names, permits: Semaphore::new(1), _scratch: scratch, graph_root: None },
    )]);
    finish_projection(launch, servers, tools, Some(Arc::new(VtcodeApproval::new(true))), Some(reviewed.owner))
}

async fn discover_stdio(
    command: Command,
    launch: &RelayBinding,
) -> anyhow::Result<(mcp_client::RunningMcpPeer, Vec<Tool>)> {
    tokio::select! {
        biased;
        () = launch.cancelled().cancelled() => anyhow::bail!("MCP projection was revoked"),
        result = async {
            let peer = mcp_client::client::open_isolated_stdio_peer(
                command, TIMEOUT, launch.cancelled().clone(),
            ).await?;
            let listed = tokio::time::timeout(TIMEOUT, mcp_client::catalog::list_tools(&peer)).await??;
            Ok((peer, listed))
        } => result,
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
        .then(|| Arc::new(VtcodeApproval::new(false)));
    anyhow::ensure!(
        !definitions.is_empty() && definitions.len() <= 16,
        "invalid MiMo MCP service count"
    );
    let workspace = FsPath::new(directory).canonicalize()?;
    anyhow::ensure!(
        workspace.is_dir() && workspace.parent().is_some(),
        "invalid MiMo MCP workspace"
    );
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

        let (peer, listed) = discover_stdio(command, &launch).await?;
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
    finish_projection(launch, servers, native_tools, approvals, None)
}

fn finish_projection(
    launch: Arc<RelayBinding>,
    servers: BTreeMap<String, Server>,
    tools: BTreeSet<String>,
    approvals: Option<Arc<VtcodeApproval>>,
    linux_owner: Option<LinuxMcpOwner>,
) -> anyhow::Result<Projection> {
    let requires_approval = approvals.is_some();
    anyhow::ensure!(!requires_approval || !tools.is_empty(), "No tools in the selected ACP MCP catalog");
    let access = RelayBinding::new(launch.identity().clone(), Duration::from_secs(900))?;
    let names = servers.keys().cloned().collect();
    let state = Arc::new(Bridge { launch: launch.clone(), access: access.clone(), servers, approvals: approvals.clone() });
    let route = if requires_approval { "/shiju-vtcode-mcp/{service}" } else { "/shiju-mimo-mcp/{service}" };
    let router = Router::new().route(route, post(handle)).with_state(state);
    Ok(Projection { router, servers: names, access, launch, linux_owner, tools: tools.into_iter().collect(), approvals })
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
