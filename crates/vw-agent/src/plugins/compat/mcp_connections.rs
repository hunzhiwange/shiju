//! 会话拥有连接与取消权；旧协议连接始终私有，不借用其他会话的权限。
use super::mcp_connection_settings::settings;
use agent_client_protocol::McpServer;
use anyhow::{Context, Result, bail};
use rmcp::service::{Peer, RoleClient};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    future::Future,
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use tokio_util::sync::CancellationToken;

type RunningPeer = crate::tools::mcp_common::RunningMcpPeer;
type PeerFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
struct Entry {
    configuration: String,
    connection: String,
    source_root: Option<std::path::PathBuf>,
    closed: CancellationToken,
    client: Arc<AsyncMutex<Option<RunningPeer>>>,
    catalogs: Mutex<std::sync::Weak<super::mcp_notifications::McpCatalogs>>,
    forms: Option<super::mcp_elicitation::FormBinding>,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.closed.cancel();
    }
}
#[derive(Default)]
struct State {
    closed: bool,
    allowed: Option<HashMap<String, String>>,
    entries: HashMap<String, Arc<Entry>>,
    required_failure: Option<String>,
}
#[derive(Default)]
pub(crate) struct McpSessionConnections {
    state: Mutex<State>,
}

// Discovery creates temporary synchronous bridge runtimes. Transport tasks must outlive them.
// Request permissions and cancellation stay with their caller, never on this runtime.
fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("mcp-connections")
            .enable_all()
            .build()
            .expect("failed to create MCP connection runtime")
    })
}
struct AbortOnDrop(tokio::task::AbortHandle);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
struct ActiveConnection {
    client: OwnedMutexGuard<Option<RunningPeer>>,
    keep: bool,
}
impl Drop for ActiveConnection {
    fn drop(&mut self) {
        if !self.keep {
            if let Some(client) = self.client.take() {
                client.service().catalogs.close();
            }
        }
    }
}
pub(crate) fn configuration_key(server: &McpServer) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(server)?)))
}
fn connection_key(
    server: &McpServer,
    configuration: &str,
    forms: Option<&super::mcp_elicitation::FormHost>,
) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(configuration);
    if let Some(forms) = forms {
        hash.update(forms.generation());
    }
    match server {
        McpServer::Stdio(server) => {
            if let Some(cwd) = crate::plugins::plugin_mcp_runtime_cwd(server)? {
                hash.update(cwd.as_os_str().as_encoded_bytes());
            }
        }
        McpServer::Http(server) => {
            if let Some(headers) = crate::plugins::plugin_mcp_http_headers(server)? {
                for (key, value) in &headers {
                    hash.update(key.as_str());
                    hash.update([0]);
                    hash.update(value.as_bytes());
                    hash.update([0]);
                }
            }
        }
        _ => {}
    }
    Ok(hex::encode(hash.finalize()))
}
impl McpSessionConnections {
    #[cfg(test)]
    pub(crate) async fn list_tools(
        &self,
        server: &McpServer,
        timeout: Option<Duration>,
    ) -> Result<Vec<rmcp::model::Tool>> {
        self.with_client(server, timeout, |client| {
            Box::pin(async move {
                Ok(client.service().catalogs.tools(client.peer()).await?.as_ref().clone())
            })
        })
        .await
    }
    pub(crate) fn tool_binding(
        &self,
        server: &McpServer,
    ) -> Option<super::mcp_notifications::ToolCatalogBinding> {
        let state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = state.entries.get(crate::tools::mcp_common::server_name(server))?;
        let catalogs =
            entry.catalogs.lock().unwrap_or_else(std::sync::PoisonError::into_inner).upgrade()?;
        catalogs.binding()
    }
    pub(crate) fn synchronize(&self, servers: &[McpServer]) -> Result<()> {
        let mut allowed = HashMap::new();
        for server in servers {
            let name = crate::tools::mcp_common::server_name(server);
            if allowed.insert(name.to_owned(), configuration_key(server)?).is_some() {
                bail!("duplicate MCP server name in session");
            }
        }
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.entries.retain(|name, entry| {
            let keep = allowed.get(name) == Some(&entry.configuration);
            if !keep {
                entry.closed.cancel();
            }
            keep
        });
        state.allowed = Some(allowed);
        state.required_failure = None;
        Ok(())
    }
    pub(crate) fn record_required_failure(&self, name: &str) {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner).required_failure =
            Some(format!("required MCP server '{name}' failed to initialize"));
    }
    pub(crate) fn ensure_ready(&self) -> Result<()> {
        let state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed {
            bail!("MCP session is closed");
        }
        if let Some(error) = &state.required_failure {
            bail!("{error}");
        }
        Ok(())
    }
    pub(crate) fn shutdown(&self) {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        state.closed = true;
        for (_, entry) in state.entries.drain() {
            entry.closed.cancel();
        }
    }
    pub(crate) fn close_plugin(&self, root: &Path) {
        let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let removed = state
            .entries
            .iter()
            .filter(|(_, e)| e.source_root.as_deref() == Some(root))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        for name in removed {
            if let Some(entry) = state.entries.remove(&name) {
                entry.closed.cancel();
            }
            if let Some(allowed) = &mut state.allowed {
                allowed.remove(&name);
            }
        }
    }
    pub(crate) async fn with_peer<T>(
        &self,
        server: &McpServer,
        timeout: Option<Duration>,
        f: impl for<'a> FnOnce(&'a Peer<RoleClient>) -> PeerFuture<'a, T>,
    ) -> Result<T> {
        self.with_client(server, timeout, |client| f(client.peer())).await
    }
    pub(crate) async fn with_client<T>(
        &self,
        server: &McpServer,
        timeout: Option<Duration>,
        f: impl for<'a> FnOnce(&'a RunningPeer) -> PeerFuture<'a, T>,
    ) -> Result<T> {
        let abort = crate::tools::current_tool_use_context().and_then(|c| c.abort_token().cloned());
        if abort.as_ref().is_some_and(CancellationToken::is_cancelled) {
            bail!("MCP call cancelled");
        }
        let settings = settings(server)?;
        let startup = timeout
            .or(settings.startup_timeout_ms.map(Duration::from_millis))
            .unwrap_or(Duration::from_secs(30));
        let call = timeout
            .or(settings.tool_timeout_ms.map(Duration::from_millis))
            .unwrap_or(Duration::from_secs(60));
        if startup.is_zero() || call.is_zero() {
            bail!("MCP timeouts must be positive");
        }
        let configuration = configuration_key(server)?;
        let name = crate::tools::mcp_common::server_name(server);
        let form_host = super::mcp_elicitation::current_host();
        let connection = match connection_key(server, &configuration, form_host.as_ref()) {
            Ok(key) => key,
            Err(error) => {
                if let Some(entry) = self
                    .state
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .entries
                    .remove(name)
                {
                    entry.closed.cancel();
                }
                return Err(error);
            }
        };
        if name.trim().is_empty() {
            bail!("MCP server name must not be empty");
        }
        let entry = {
            let mut state = self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed {
                bail!("MCP session is closed");
            }
            if state.allowed.as_ref().is_some_and(|a| a.get(name) != Some(&configuration)) {
                bail!("MCP server configuration is no longer enabled in this session");
            }
            if state.entries.get(name).is_some_and(|e| e.connection != connection) {
                state.entries.remove(name).expect("entry checked").closed.cancel();
            }
            state
                .entries
                .entry(name.to_owned())
                .or_insert_with(|| {
                    let closed = CancellationToken::new();
                    Arc::new(Entry {
                        configuration,
                        connection,
                        source_root: settings.source_root.clone(),
                        forms: form_host.map(|host| host.bind(name, closed.clone())),
                        closed,
                        client: Arc::new(AsyncMutex::new(None)),
                        catalogs: Mutex::new(std::sync::Weak::new()),
                    })
                })
                .clone()
        };
        let client = tokio::select! {
            biased;
            _ = entry.closed.cancelled() => bail!("MCP session connection was closed"),
            _ = async { match &abort { Some(token) => token.cancelled().await, None => std::future::pending().await } } => bail!("MCP call cancelled"),
            client = entry.client.clone().lock_owned() => client,
        };
        let mut active = ActiveConnection { client, keep: false };
        // Replies use the independent host responder, never the peer mutex held by this call.
        // An operation's end revokes pending form authority before the next call can start.
        let _form_operation = entry.forms.as_ref().map(|forms| forms.activate());
        if active.client.as_ref().is_some_and(|c| c.peer().is_transport_closed()) {
            active.client.take();
        }
        if active.client.is_none() {
            let server = server.clone();
            let closed = entry.closed.child_token();
            let forms = entry.forms.clone();
            let task = runtime().spawn(async move {
                let operation = async {
                    let client =
                        crate::tools::mcp_common::open_server_peer(server, startup, closed, forms)
                            .await?;
                    if client.peer_info().is_some_and(|info| info.capabilities.tools.is_some()) {
                        client
                            .service()
                            .catalogs
                            .tools(client.peer())
                            .await
                            .context("failed to obtain initial MCP tool catalog")?;
                    }
                    Ok::<_, anyhow::Error>(client)
                };
                tokio::time::timeout(startup, operation)
                    .await
                    .context("MCP startup or initial tool catalog timed out")?
            });
            let _abort = AbortOnDrop(task.abort_handle());
            let client = tokio::select! {
                biased;
                _ = entry.closed.cancelled() => bail!("MCP session connection was closed"),
                _ = async { match &abort { Some(token) => token.cancelled().await, None => std::future::pending().await } } => bail!("MCP startup cancelled"),
                result = task => match result.context("MCP connection task stopped").and_then(|result| result) {
                    Ok(client) => client,
                    Err(error) => {
                        if settings.required { self.record_required_failure(name); }
                        return Err(error);
                    }
                },
            };
            *entry.catalogs.lock().unwrap_or_else(std::sync::PoisonError::into_inner) =
                Arc::downgrade(&client.service().catalogs);
            *active.client = Some(client);
        }
        let client = active.client.as_ref().expect("initialized MCP connection");
        let result = tokio::select! {
            biased;
            _ = entry.closed.cancelled() => Err(anyhow::anyhow!("MCP session connection was closed")),
            _ = async { match &abort { Some(token) => token.cancelled().await, None => std::future::pending().await } } => Err(anyhow::anyhow!("MCP call cancelled")),
            result = tokio::time::timeout(call, f(client)) => result.context("MCP operation timed out")?,
        };
        active.keep = result.is_ok() && !client.peer().is_transport_closed();
        result
    }
}
impl Drop for McpSessionConnections {
    fn drop(&mut self) {
        self.shutdown();
    }
}
