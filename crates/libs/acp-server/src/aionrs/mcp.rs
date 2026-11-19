//! A private per-process broker for sandboxed aionrs MCP transports.
use super::process::failure;
use crate::server::BackendError;
use agent_client_protocol::McpServer;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    process::{Child, ChildStdin, ChildStdout, Command},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

const LIMIT: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(15);

type Catalog = Arc<Mutex<HashSet<String>>>;
type Approvals = Arc<Mutex<HashMap<String, Value>>>;

pub struct Broker {
    socket: PathBuf,
    task: JoinHandle<()>,
    cancel: CancellationToken,
    catalog: Catalog,
    approvals: Approvals,
}

impl Broker {
    pub fn start(
        socket: PathBuf,
        token: String,
        workspace: &Path,
        servers: Vec<McpServer>,
    ) -> Result<Self, BackendError> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) || servers.len() > 16 {
            return Err(invalid_mount());
        }
        let state = socket.parent().ok_or_else(invalid_mount)?;
        let mounts = servers
            .into_iter()
            .map(|server| Mount::new(server, workspace, state))
            .collect::<Result<Vec<_>, _>>()?;
        let listener = UnixListener::bind(&socket).map_err(failure)?;
        let catalog = Catalog::default();
        let approvals = Approvals::default();
        let cancel = CancellationToken::new();
        let task = tokio::spawn(serve(
            listener,
            token,
            mounts,
            catalog.clone(),
            approvals.clone(),
            cancel.clone(),
        ));
        Ok(Self { socket, task, cancel, catalog, approvals })
    }

    pub fn contains(&self, name: &str) -> bool {
        self.catalog.lock().unwrap().contains(name)
    }

    pub fn approve(&self, name: String, arguments: Value) -> Result<(), BackendError> {
        let mut approvals = self.approvals.lock().unwrap();
        if approvals.contains_key(&name) {
            return Err(failure("concurrent approval for the same MCP tool"));
        }
        approvals.insert(name, arguments);
        Ok(())
    }

    pub fn clear_approvals(&self) {
        self.approvals.lock().unwrap().clear();
    }
}

impl Drop for Broker {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.task.abort();
        let _ = std::fs::remove_file(&self.socket);
    }
}

fn invalid_mount() -> BackendError {
    BackendError::InvalidRequest("unsupported managed aionrs MCP launch configuration".into())
}

#[derive(Default, serde::Deserialize)]
struct ToolPolicy {
    allow: HashSet<String>,
    deny: HashSet<String>,
}

impl ToolPolicy {
    fn allows(&self, server: &str, name: &str) -> bool {
        let id = format!("{server}__{name}");
        !self.deny.contains(name)
            && !self.deny.contains(&id)
            && (self.allow.is_empty() || self.allow.contains(name) || self.allow.contains(&id))
    }
}

struct Mount {
    name: String,
    program: PathBuf,
    args: Vec<String>,
    workspace: PathBuf,
    peer: Option<Peer>,
    scratch: tempfile::TempDir,
    codegraph: bool,
    tool_policy: ToolPolicy,
    tools: HashMap<String, String>,
}

impl Mount {
    fn new(server: McpServer, workspace: &Path, state: &Path) -> Result<Self, BackendError> {
        let McpServer::Stdio(server) = server else { return Err(invalid_mount()) };
        if !literal_name(&server.name) || !server.command.is_absolute() {
            return Err(invalid_mount());
        }
        let program = server.command.canonicalize().map_err(|_| invalid_mount())?;
        if !program.is_file() || program.starts_with(workspace) {
            return Err(invalid_mount());
        }
        let scratch = tempfile::tempdir_in(state).map_err(failure)?;
        let codegraph = program.file_name().is_some_and(|name| name == "rustcodegraph");
        if !server.env.is_empty() {
            return Err(invalid_mount());
        }
        let tool_policy = server
            .meta
            .as_ref()
            .and_then(|meta| meta.get("shiju.aionrs.toolPolicy"))
            .map(|policy| serde_json::from_value(policy.clone()))
            .transpose()
            .map_err(|_| invalid_mount())?
            .unwrap_or_default();
        let mut args = server.args;
        if codegraph {
            let [serve, mcp, path, root, no_watch] = args.as_mut_slice() else {
                return Err(invalid_mount());
            };
            if serve != "serve"
                || mcp != "--mcp"
                || path != "--path"
                || no_watch != "--no-watch"
                || Path::new(root.as_str()).canonicalize().map_err(failure)? != workspace
            {
                return Err(invalid_mount());
            }
            prepare_codegraph_workspace(workspace, scratch.path())?;
            *root = scratch.path().to_str().ok_or_else(invalid_mount)?.into();
        }
        Ok(Self {
            name: server.name,
            program,
            args,
            workspace: workspace.into(),
            scratch,
            codegraph,
            tool_policy,
            peer: None,
            tools: HashMap::new(),
        })
    }

    async fn request(
        &mut self,
        mut body: Value,
        catalog: &Catalog,
        approvals: &Approvals,
    ) -> Result<Value, BackendError> {
        let method = body.get("method").and_then(Value::as_str);
        let id = body.get("id").cloned().unwrap_or(Value::Null);
        match method {
            Some("initialize")
                if self.peer.is_none() && !id.is_null() && body["params"].is_object() =>
            {
                // The host decides the capabilities advertised to the local process.
                body["params"]["capabilities"] = json!({});
                self.peer = Some(Peer::start(
                    &self.program,
                    &self.args,
                    &self.workspace,
                    self.scratch.path(),
                    self.codegraph,
                )?);
            }
            Some("notifications/initialized") if id.is_null() => {}
            Some("tools/list") if !id.is_null() => {}
            Some("tools/call") if !id.is_null() => {
                let Some(name) = body.pointer("/params/name").and_then(Value::as_str) else {
                    return Ok(rpc_error(id));
                };
                let allowed = self
                    .tools
                    .get(name)
                    .zip(body.pointer("/params/arguments"))
                    .filter(|(_, args)| take_approval(approvals, name, args))
                    .map(|(original, _)| original.clone());
                let Some(original) = allowed else { return Ok(rpc_error(id)) };
                body["params"]["name"] = Value::String(original);
                if self.codegraph
                    && let Some(path) = body.pointer_mut("/params/arguments/projectPath")
                {
                    let Some(project) = path.as_str() else { return Ok(rpc_error(id)) };
                    if !project.is_empty()
                        && self.workspace.join(project).canonicalize().ok().as_deref()
                            != Some(self.workspace.as_path())
                    {
                        return Ok(rpc_error(id));
                    }
                    *path = json!(self.scratch.path());
                }
            }
            _ => return Ok(rpc_error(id)),
        }
        let list = body["method"] == "tools/list";
        let Some(peer) = self.peer.as_mut() else { return Ok(rpc_error(id)) };
        let mut response = peer.request(body, &id).await?;
        if list {
            let Some(tools) = response.pointer_mut("/result/tools").and_then(Value::as_array_mut)
            else {
                return Ok(rpc_error(id));
            };
            // Fixed catalogs only. A paginated or mutable catalog must be fully audited
            // before it can be advertised by the relay.
            tools.retain(|tool| {
                tool["name"].as_str().is_some_and(|name| self.tool_policy.allows(&self.name, name))
            });
            let mut names = HashMap::new();
            for tool in tools {
                let original =
                    tool.get("name").and_then(Value::as_str).ok_or_else(invalid_mount)?;
                let alias = format!("mcp__{}__{original}", self.name);
                if !literal_name(original) || alias.len() > 64 || names.contains_key(&alias) {
                    return Ok(rpc_error(id));
                }
                names.insert(alias.clone(), original.to_string());
                tool["name"] = Value::String(alias);
            }
            if response.pointer("/result/nextCursor").is_some_and(|cursor| !cursor.is_null()) {
                return Ok(rpc_error(id));
            }
            let mut active = catalog.lock().unwrap();
            if names.keys().any(|name| active.contains(name) && !self.tools.contains_key(name)) {
                return Ok(rpc_error(id));
            }
            for (name, _) in self.tools.drain() {
                active.remove(&name);
            }
            active.extend(names.keys().cloned());
            self.tools = names;
        }
        Ok(response)
    }
}

// RustCodeGraph opens SQLite using WAL. Keep its writeable database and sidecars
// under the private sandbox while resolving source files in the read-only workspace.
fn prepare_codegraph_workspace(workspace: &Path, scratch: &Path) -> Result<(), BackendError> {
    use std::os::unix::fs::{OpenOptionsExt, symlink};
    let index = workspace.join(".rustcodegraph");
    if index.canonicalize().map_err(failure)? != index
        || index.join("rustcodegraph.db-wal").metadata().is_ok_and(|entry| entry.len() != 0)
    {
        return Err(BackendError::InvalidRequest(
            "aionrs requires a checkpointed RustCodeGraph index".into(),
        ));
    }
    let private_index = scratch.join(".rustcodegraph");
    std::fs::create_dir(&private_index).map_err(failure)?;
    let mut database = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(index.join("rustcodegraph.db"))
        .map_err(failure)?;
    if !database.metadata().map_err(failure)?.is_file() {
        return Err(failure("invalid RustCodeGraph database"));
    }
    let mut copy =
        std::fs::File::create(private_index.join("rustcodegraph.db")).map_err(failure)?;
    std::io::copy(&mut database, &mut copy).map_err(failure)?;
    for entry in std::fs::read_dir(workspace).map_err(failure)? {
        let entry = entry.map_err(failure)?;
        if entry.file_name() != ".rustcodegraph" {
            symlink(entry.path(), scratch.join(entry.file_name())).map_err(failure)?;
        }
    }
    Ok(())
}

fn take_approval(approvals: &Approvals, name: &str, arguments: &Value) -> bool {
    let mut grants = approvals.lock().unwrap();
    if grants.get(name) != Some(arguments) {
        return false;
    }
    grants.remove(name);
    true
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;

fn literal_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
}

fn rpc_error(id: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32000,"message":"MCP request rejected"}})
}

struct Peer {
    _child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Peer {
    fn start(
        program: &Path,
        args: &[String],
        workspace: &Path,
        scratch: &Path,
        codegraph: bool,
    ) -> Result<Self, BackendError> {
        use std::process::Stdio;
        let program_literal = serde_json::to_string(program).map_err(failure)?;
        let workspace_literal = serde_json::to_string(workspace).map_err(failure)?;
        let scratch_literal = serde_json::to_string(scratch).map_err(failure)?;
        // MCP has no credentials or ambient HOME and cannot start subprocesses,
        // use the network or write any part of the workspace.
        let policy = format!(
            r#"(version 1)
(allow default)
(deny file-read-data (subpath "/Users") (subpath "/Volumes") (subpath "/private")
 (subpath "/opt") (subpath "/usr/local") (subpath "/Library") (subpath "/Applications"))
(allow file-read-data (literal {program_literal}) (subpath {workspace_literal}) (subpath {scratch_literal})
 (subpath "/System") (subpath "/usr/lib") (subpath "/usr/share") (subpath "/dev"))
(deny file-write*)
(allow file-write* (subpath {scratch_literal}) (literal "/dev/null"))
(deny process-fork process-exec)
(allow process-exec (literal {program_literal}))
(deny network*)"#
        );
        let mut command = Command::new("/usr/bin/sandbox-exec");
        command
            .args(["-p", &policy])
            .arg(program)
            .args(args)
            .current_dir(workspace)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch)
            .env("CFFIXED_USER_HOME", scratch)
            .env("TMPDIR", scratch);
        if codegraph {
            command.env("RUSTCODEGRAPH_NO_DAEMON", "1");
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(failure)?;
        let input = child.stdin.take().ok_or_else(|| failure("MCP stdin"))?;
        let output = BufReader::new(child.stdout.take().ok_or_else(|| failure("MCP stdout"))?);
        Ok(Self { _child: child, input, output })
    }

    async fn request(&mut self, body: Value, id: &Value) -> Result<Value, BackendError> {
        let mut request = serde_json::to_vec(&body).map_err(failure)?;
        request.push(b'\n');
        self.input.write_all(&request).await.map_err(failure)?;
        if id.is_null() {
            return Ok(Value::Null);
        }
        loop {
            let mut line = Vec::new();
            (&mut self.output)
                .take((LIMIT + 1) as u64)
                .read_until(b'\n', &mut line)
                .await
                .map_err(failure)?;
            if line.last() != Some(&b'\n') || line.len() > LIMIT {
                return Err(failure("invalid MCP frame"));
            }
            let response: Value = serde_json::from_slice(&line).map_err(failure)?;
            if response.get("id") == Some(id) && response.get("method").is_none() {
                return Ok(response);
            }
            // Notifications may be ignored. Server-initiated requests are never
            // granted access to model sampling, roots or user input.
            if response.get("id").is_some() {
                return Err(failure("unexpected MCP response"));
            }
        }
    }
}

async fn serve(
    listener: UnixListener,
    token: String,
    mut mounts: Vec<Mount>,
    catalog: Catalog,
    approvals: Approvals,
    cancel: CancellationToken,
) {
    loop {
        let mut stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(_) => break,
        };
        let request = async {
            let mut request = Vec::new();
            (&mut stream)
                .take((LIMIT + 1) as u64)
                .read_to_end(&mut request)
                .await
                .map_err(failure)?;
            if request.len() > LIMIT {
                return Err(failure("oversized MCP bridge request"));
            }
            let mut request: Value = serde_json::from_slice(&request).map_err(failure)?;
            if request["token"].as_str() != Some(&token) {
                return write_response(&mut stream, Value::Null).await;
            }
            let response = if request["catalog"] == true {
                json!(*catalog.lock().unwrap())
            } else {
                let mount =
                    request["mount"].as_u64().and_then(|index| mounts.get_mut(index as usize));
                let body = request["body"].take();
                match mount {
                    Some(mount) => mount.request(body, &catalog, &approvals).await?,
                    None => rpc_error(body["id"].clone()),
                }
            };
            write_response(&mut stream, response).await
        };
        let ok = tokio::select! {
            () = cancel.cancelled() => false,
            result = tokio::time::timeout(TIMEOUT, request) => matches!(result, Ok(Ok(()))),
        };
        if !ok {
            break;
        }
    }
    catalog.lock().unwrap().clear();
    approvals.lock().unwrap().clear();
}

async fn write_response(stream: &mut UnixStream, response: Value) -> Result<(), BackendError> {
    let response = serde_json::to_vec(&response).map_err(failure)?;
    if response.len() > LIMIT {
        return Err(failure("oversized MCP bridge response"));
    }
    stream.write_all(&response).await.map_err(failure)
}
