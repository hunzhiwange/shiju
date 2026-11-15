//! MCP 工具共享辅助模块。
//!
//! 本模块封装 MCP server 配置加载、传输连接、server 元信息序列化以及资源内容
//! 规范化输出逻辑。上层工具通过这些函数保持一致的连接生命周期与 JSON 返回结构。

use crate::app::agent::config::schema::{Config, ConfigExt};
use crate::plugins::{
    PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers, plugin_mcp_discovery_options,
};
use agent_client_protocol::{HttpHeader, McpServer, McpServerStdio};
use anyhow::Context;
use reqwest_012::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::{
    ClientInfo, CreateElicitationRequestParam, CreateElicitationResult, CreateMessageRequestParam,
    CreateMessageResult, ErrorData, ListRootsResult,
};
use rmcp::service::{NotificationContext, Peer, RequestContext, RoleClient};
use rmcp::transport::{
    SseClientTransport, StreamableHttpClientTransport, sse_client::SseClientConfig,
    streamable_http_client::StreamableHttpClientTransportConfig,
};
use serde_json::{Value, json};
use std::{future::Future, path::Path, time::Duration};
use tokio::process::Command;

pub use mcp_client::{
    auth_method_names, find_server_by_name, management_for_server, management_timeout,
    mcp_tool_allowed, resource_contents_json, server_name, server_summary, server_transport,
};
use mcp_client::{apply_mcp_server_override_values, merge_global_mcp_server_values};

mod wire_check;
mod wire_http;
mod wire_stdio;

#[cfg(not(target_arch = "wasm32"))]
use wire_stdio::ProbeChild;

struct UnsupportedMcpCapability {
    scope: &'static str,
    capability: &'static str,
    reason: &'static str,
}

const UNSUPPORTED_MCP_CAPABILITIES: &[UnsupportedMcpCapability] = &[
    UnsupportedMcpCapability {
        scope: "client",
        capability: "sampling",
        reason: "vw-agent MCP client does not broker model sampling requests from servers.",
    },
    UnsupportedMcpCapability {
        scope: "client",
        capability: "elicitation",
        reason: "vw-agent MCP client does not collect interactive user input for servers.",
    },
    UnsupportedMcpCapability {
        scope: "client",
        capability: "roots",
        reason: "vw-agent MCP client does not expose workspace roots through the MCP roots API.",
    },
    UnsupportedMcpCapability {
        scope: "server",
        capability: "resources.subscribe",
        reason: "vw-agent MCP client performs one-shot resource reads and does not keep resource subscriptions open.",
    },
];

pub(crate) struct NoopClient {
    info: ClientInfo,
    form_owner: Option<crate::plugins::mcp_elicitation::FormOwner>,
    form_server: String,
    closed: tokio_util::sync::CancellationToken,
    pub(crate) catalogs: std::sync::Arc<crate::plugins::McpCatalogs>,
    pub(crate) raw_responses: std::sync::Arc<crate::plugins::RawMcpResponse>,
}

impl Default for NoopClient {
    fn default() -> Self {
        Self::with_cancellation(tokio_util::sync::CancellationToken::new())
    }
}
impl NoopClient {
    fn with_cancellation(closed: tokio_util::sync::CancellationToken) -> Self {
        Self {
            form_owner: None,
            form_server: String::new(),
            closed: closed.clone(),
            raw_responses: Default::default(),
            catalogs: std::sync::Arc::new(crate::plugins::McpCatalogs::new(closed)),
            info: crate::plugins::mcp_client_info(),
        }
    }
}

pub(crate) fn unsupported_mcp_capabilities_json() -> Value {
    Value::Array(
        UNSUPPORTED_MCP_CAPABILITIES
            .iter()
            .map(|capability| {
                json!({
                    "scope": capability.scope,
                    "capability": capability.capability,
                    "reason": capability.reason,
                })
            })
            .collect(),
    )
}

fn unsupported_client_capability_error(capability: &str) -> ErrorData {
    ErrorData::invalid_request(
        format!("MCP client capability '{capability}' is not supported by vw-agent"),
        Some(json!({ "capability": capability })),
    )
}

impl rmcp::ClientHandler for NoopClient {
    fn get_info(&self) -> ClientInfo {
        self.info.clone()
    }

    fn create_message(
        &self,
        _params: CreateMessageRequestParam,
        _context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<CreateMessageResult, ErrorData>> + Send + '_ {
        std::future::ready(Err(unsupported_client_capability_error("sampling")))
    }

    fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<ListRootsResult, ErrorData>> + Send + '_ {
        std::future::ready(Err(unsupported_client_capability_error("roots")))
    }

    fn create_elicitation(
        &self,
        request: CreateElicitationRequestParam,
        context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<CreateElicitationResult, ErrorData>> + Send + '_ {
        async move {
            let schema = self.raw_responses.take_form(&context.id);
            let owner = self
                .form_owner
                .clone()
                .ok_or_else(|| unsupported_client_capability_error("elicitation"))?;
            crate::plugins::mcp_elicitation::request(
                owner,
                self.form_server.clone(),
                request,
                context.ct,
                self.closed.clone(),
                Some(schema.ok_or_else(|| {
                    ErrorData::invalid_params("MCP form wire schema is unavailable", None)
                })?),
            )
            .await
        }
    }

    fn on_resource_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        if context
            .peer
            .peer_info()
            .and_then(|i| i.capabilities.resources.as_ref())
            .and_then(|c| c.list_changed)
            == Some(true)
        {
            self.catalogs.resources_changed();
        } else {
            tracing::debug!("ignoring unadvertised MCP resources.listChanged notification");
        }
        std::future::ready(())
    }

    fn on_tool_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        if context
            .peer
            .peer_info()
            .and_then(|i| i.capabilities.tools.as_ref())
            .and_then(|c| c.list_changed)
            == Some(true)
        {
            self.catalogs.tools_changed();
        } else {
            tracing::debug!("ignoring unadvertised MCP tools.listChanged notification");
        }
        std::future::ready(())
    }

    fn on_prompt_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        if context
            .peer
            .peer_info()
            .and_then(|i| i.capabilities.prompts.as_ref())
            .and_then(|c| c.list_changed)
            == Some(true)
        {
            self.catalogs.prompts_changed();
        } else {
            tracing::debug!("ignoring unadvertised MCP prompts.listChanged notification");
        }
        std::future::ready(())
    }
}

/// 加载当前工作目录下解析完成的 ACP 配置。
///
/// # 返回值
///
/// 成功时返回已合并默认值和本地配置的 `ResolvedAcpxConfig`。
///
/// # 错误
///
/// 当当前目录不可读取，或 ACP 配置加载、解析失败时返回错误。
pub async fn load_resolved_acp_config() -> anyhow::Result<acp_config::ResolvedAcpxConfig> {
    load_resolved_acp_config_from(std::env::current_dir()?).await
}

/// 从指定工作目录加载解析完成的 ACP 配置。
///
/// # 参数
///
/// - `cwd`: 用于解析项目级 `.vwacprc.json` 的工作目录。
///
/// # 返回值
///
/// 成功时返回已合并默认值和本地配置的 `ResolvedAcpxConfig`。
///
/// # 错误
///
/// 当 ACP 配置加载或解析失败时返回错误。
pub async fn load_resolved_acp_config_from(
    cwd: impl AsRef<Path>,
) -> anyhow::Result<acp_config::ResolvedAcpxConfig> {
    load_resolved_mcp_config(cwd.as_ref(), None).await
}

/// Native tool discovery uses the project established at the request boundary.
pub(crate) fn load_resolved_runtime_mcp_config_from(
    cwd: &Path,
) -> impl Future<Output = anyhow::Result<acp_config::ResolvedAcpxConfig>> + '_ {
    // Capture the project before the toolset's synchronous bridge changes threads.
    let project_id = crate::project::instance::project()
        .filter(|_| {
            crate::tools::current_tool_use_context()
                .is_none_or(|context| context.channel() != Some("acp"))
        })
        .map(|project| project.id);
    load_resolved_mcp_config(cwd, project_id)
}

async fn load_resolved_mcp_config(
    cwd: &Path,
    project_id: Option<String>,
) -> anyhow::Result<acp_config::ResolvedAcpxConfig> {
    let mut config = acp_config::load_resolved_config(cwd).await.map_err(anyhow::Error::from)?;
    merge_runtime_mcp_servers(&mut config, cwd, project_id).await?;
    if let Some(context) = crate::tools::current_tool_use_context()
        && let Some(selected) = context.native_mcp_selection()
    {
        config.mcp_servers.retain(|server| selected.contains(server_name(server)));
    }
    Ok(config)
}

pub(crate) async fn load_resolved_acp_config_from_with_plugin_options(
    cwd: impl AsRef<Path>,
    plugin_options: &PluginDiscoveryOptions,
) -> anyhow::Result<acp_config::ResolvedAcpxConfig> {
    let mut config = acp_config::load_resolved_config(cwd).await.map_err(anyhow::Error::from)?;
    let agent_config = merge_runtime_global_mcp_servers(&mut config).await?;
    merge_enabled_plugin_mcp_servers(&mut config.mcp_servers, plugin_options)?;
    apply_global_mcp_server_overrides_from_agent_config(&mut config, &agent_config)?;
    Ok(config)
}

async fn merge_runtime_mcp_servers(
    config: &mut acp_config::ResolvedAcpxConfig,
    workspace: &Path,
    project_id: Option<String>,
) -> anyhow::Result<()> {
    let tenant = crate::tools::current_tool_use_context()
        .map(|context| context.tenant_scope())
        .unwrap_or_else(crate::session::ui_store::current_tenant_scope);
    let mut agent_config = crate::config::get_project_for_tenant(&tenant, workspace)
        .await
        .map_err(|error| anyhow::anyhow!("failed to load scoped MCP configuration: {error}"))?;
    merge_global_mcp_servers_from_agent_config(config, &agent_config)?;
    agent_config.workspace_dir = workspace.to_path_buf();
    let plugin_options = plugin_mcp_discovery_options(&agent_config);
    merge_enabled_plugin_mcp_servers(&mut config.mcp_servers, &plugin_options)?;
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(project_id) = project_id {
        let project = vw_api_transport::ProjectScope::new(tenant, project_id.into())?;
        let selected = crate::agent_apps::mcp::runtime::selected_runtime_servers(
            &project,
            workspace,
            &config.mcp_servers,
        )?;
        config.mcp_servers.extend(selected);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = project_id;
    apply_global_mcp_server_overrides_from_agent_config(config, &agent_config)
}

async fn merge_runtime_global_mcp_servers(
    config: &mut acp_config::ResolvedAcpxConfig,
) -> anyhow::Result<Config> {
    let agent_config = match Config::load_or_init().await {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("skipping global MCP servers: failed to load agent config: {error}");
            return Err(error);
        }
    };
    merge_global_mcp_servers_from_agent_config(config, &agent_config)?;
    Ok(agent_config)
}

fn apply_global_mcp_server_overrides_from_agent_config(
    config: &mut acp_config::ResolvedAcpxConfig,
    agent_config: &Config,
) -> anyhow::Result<()> {
    apply_mcp_server_override_values(
        config,
        &agent_config.mcp_server_overrides,
        &agent_config.config_path.to_string_lossy(),
    )
}

fn merge_global_mcp_servers_from_agent_config(
    config: &mut acp_config::ResolvedAcpxConfig,
    agent_config: &Config,
) -> anyhow::Result<()> {
    merge_global_mcp_server_values(
        config,
        &agent_config.mcp_servers,
        &agent_config.config_path.to_string_lossy(),
    )
}

fn remote_header_map(headers: &[HttpHeader]) -> anyhow::Result<HeaderMap> {
    let mut header_map = HeaderMap::new();
    for header in headers {
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .with_context(|| format!("invalid MCP remote header name '{}'", header.name))?;
        let value = HeaderValue::from_str(&header.value)
            .with_context(|| format!("invalid MCP remote header value for '{}'", header.name))?;
        header_map.append(name, value);
    }
    Ok(header_map)
}

fn remote_http_client(
    headers: &[HttpHeader],
    connect_timeout: Duration,
) -> anyhow::Result<reqwest_012::Client> {
    reqwest_012::Client::builder()
        .default_headers(remote_header_map(headers)?)
        .connect_timeout(connect_timeout)
        .build()
        .context("failed to build MCP remote HTTP client")
}

fn connection_check_http_client(
    headers: &[HttpHeader],
    timeout: Duration,
) -> anyhow::Result<reqwest_012::Client> {
    let mut headers = remote_header_map(headers)?;
    for (_, value) in &mut headers {
        value.set_sensitive(true);
    }
    reqwest_012::Client::builder()
        .default_headers(headers)
        .connect_timeout(timeout)
        .read_timeout(timeout)
        .retry(reqwest_012::retry::never())
        .redirect(reqwest_012::redirect::Policy::none())
        .build()
        .context("failed to build MCP connection check HTTP client")
}

async fn with_connect_timeout<T>(
    server_name: &str,
    timeout: Duration,
    operation: impl Future<Output = anyhow::Result<T>>,
) -> anyhow::Result<T> {
    match tokio::time::timeout(timeout, operation).await {
        Ok(result) => result,
        Err(_) => anyhow::bail!(
            "timed out connecting to MCP server '{}' after {} ms",
            server_name,
            timeout.as_millis()
        ),
    }
}

/// 为指定 MCP server 建立客户端 peer，并在 peer 生命周期内执行回调。
///
/// # 参数
///
/// - `server`: 已解析配置中的 MCP server 定义。
/// - `f`: 接收活动 peer 的异步回调，所有 MCP 请求应在该回调内完成。
///
/// # 返回值
///
/// 返回回调的执行结果。
///
/// # 错误
///
/// 当传输不受支持、进程启动失败、网络连接失败、MCP 握手失败或回调自身失败时
/// 返回错误。
pub async fn with_server_peer<T>(
    server: &McpServer,
    f: impl for<'a> FnOnce(
        &'a Peer<RoleClient>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<T>> + Send + 'a>,
    >,
) -> anyhow::Result<T> {
    with_server_peer_timeout(server, None, f).await
}

pub async fn with_server_peer_timeout<T>(
    server: &McpServer,
    timeout: Option<Duration>,
    f: impl for<'a> FnOnce(
        &'a Peer<RoleClient>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = anyhow::Result<T>> + Send + 'a>,
    >,
) -> anyhow::Result<T> {
    with_server_client_timeout(server, timeout, |client| f(client.peer())).await
}

pub(crate) async fn with_server_client_timeout<T>(
    server: &McpServer,
    timeout: Option<Duration>,
    f: impl for<'a> FnOnce(
        &'a RunningMcpPeer,
    )
        -> std::pin::Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'a>>,
) -> anyhow::Result<T> {
    let connections = super::context::current_tool_use_context().and_then(|context| {
        context
            .workspace_root()
            .map(|root| crate::plugins::mcp_session_connections(context.session(), &root))
    });
    match connections {
        Some(connections) => connections.with_client(server, timeout, f).await,
        None => {
            crate::plugins::McpSessionConnections::default().with_client(server, timeout, f).await
        }
    }
}

pub(crate) type RunningMcpPeer = rmcp::service::RunningService<rmcp::RoleClient, NoopClient>;

pub(crate) fn runtime_stdio_cwd(
    server: &McpServerStdio,
) -> anyhow::Result<Option<std::path::PathBuf>> {
    let cwd = crate::plugins::plugin_mcp_runtime_cwd(server)?;
    #[cfg(not(target_arch = "wasm32"))]
    if cwd.is_none() {
        return crate::agent_apps::mcp::runtime::managed_stdio_cwd(server);
    }
    Ok(cwd)
}

/// The owner runs this handshake on a durable runtime, separate from synchronous tool bridges.
pub(crate) async fn open_server_peer(
    server: McpServer,
    timeout: Duration,
    closed: tokio_util::sync::CancellationToken,
    form_owner: Option<crate::plugins::mcp_elicitation::FormOwner>,
) -> anyhow::Result<RunningMcpPeer> {
    open_server_peer_inner(server, timeout, closed, form_owner, None, None, None).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use wire_http::ConnectionCheckCleanup;

/// Open a short-lived peer for an explicit catalog check, with no session privileges.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn open_connection_check_peer(
    server: McpServer,
    timeout: Duration,
    http_io: tokio_util::sync::CancellationToken,
    child: Option<&mut Option<ProbeChild>>,
    cleanup: Option<ConnectionCheckCleanup>,
) -> anyhow::Result<RunningMcpPeer> {
    open_server_peer_inner(
        server,
        timeout,
        tokio_util::sync::CancellationToken::new(),
        None,
        Some(http_io),
        child,
        cleanup,
    )
    .await
}

async fn open_server_peer_inner(
    server: McpServer,
    timeout: Duration,
    closed: tokio_util::sync::CancellationToken,
    form_owner: Option<crate::plugins::mcp_elicitation::FormOwner>,
    check_http_io: Option<tokio_util::sync::CancellationToken>,
    child: Option<&mut Option<wire_stdio::ProbeChild>>,
    cleanup: Option<wire_http::ConnectionCheckCleanup>,
) -> anyhow::Result<RunningMcpPeer> {
    let connection_check = check_http_io.is_some();
    let mut peer_client = NoopClient::with_cancellation(closed);
    if form_owner.is_some() {
        peer_client.info.capabilities.elicitation =
            Some(rmcp::model::ElicitationCapability { schema_validation: Some(true) });
    }
    peer_client.form_owner = form_owner;
    peer_client.form_server = server_name(&server).to_owned();
    let responses = peer_client.raw_responses.clone();
    let server = &server;
    let running = match server {
        McpServer::Stdio(server) => {
            let mut command = Command::new(&server.command);
            if connection_check && !server.env.iter().any(|env| env.name == "PATH") {
                shell_command::apply_augmented_path_tokio_command(&mut command);
            }
            if let Some(cwd) = runtime_stdio_cwd(server)? {
                command.current_dir(cwd);
            }
            let args = server.args.clone();
            let env = server
                .env
                .iter()
                .map(|env| (env.name.clone(), env.value.clone()))
                .collect::<Vec<_>>();
            command.args(args);
            for (key, value) in &env {
                command.env(key, value);
            }
            let transport =
                wire_stdio::StdioTransport::new(command, responses, connection_check, child)?;
            with_connect_timeout(
                &server.name,
                timeout,
                wire_check::serve(peer_client, transport, connection_check),
            )
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?
        }
        McpServer::Sse(server) => {
            let client = if connection_check {
                connection_check_http_client(&server.headers, timeout)
            } else {
                remote_http_client(&server.headers, timeout)
            }
            .with_context(|| format!("failed to build SSE MCP client for '{}'", server.name))?;
            let mut client = wire_http::HttpClient::new(client, responses);
            let mut config =
                SseClientConfig { sse_endpoint: server.url.clone().into(), ..Default::default() };
            if let Some(http_io) = check_http_io {
                client = client
                    .with_check_origin(reqwest_012::Url::parse(&server.url)?.origin())
                    .with_check_cancellation(http_io);
                config.retry_policy =
                    std::sync::Arc::new(rmcp::transport::common::client_side_sse::NeverRetry);
            }
            let transport = with_connect_timeout(&server.name, timeout, async move {
                SseClientTransport::start_with_client(client, config)
                    .await
                    .map_err(anyhow::Error::from)
            })
            .await
            .with_context(|| format!("failed to start SSE MCP transport for '{}'", server.name))?;
            with_connect_timeout(
                &server.name,
                timeout,
                wire_check::serve(peer_client, transport, connection_check),
            )
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?
        }
        McpServer::Http(server) => {
            let oauth = match peer_client.form_owner.as_ref() {
                Some(owner) => crate::plugins::plugin_mcp_oauth_access(server, owner).await?,
                None => None,
            };
            let scoped_headers = match oauth.as_deref() {
                Some(token) => crate::plugins::plugin_mcp_oauth_headers(server, Some(token))?,
                None => crate::plugins::plugin_mcp_http_headers(server)?,
            };
            let client = if connection_check {
                connection_check_http_client(&server.headers, timeout)
            } else if let Some(headers) = scoped_headers {
                reqwest_012::Client::builder()
                    .default_headers(headers)
                    .connect_timeout(timeout)
                    // A redirect must not forward custom credential headers to another endpoint.
                    .redirect(reqwest_012::redirect::Policy::none())
                    .build()
                    .context("failed to build source-bound plugin MCP HTTP client")
            } else {
                remote_http_client(&server.headers, timeout)
            }
            .with_context(|| format!("failed to build HTTP MCP client for '{}'", server.name))?;
            let mut config = StreamableHttpClientTransportConfig::with_uri(server.url.clone());
            if connection_check {
                config.retry_config =
                    std::sync::Arc::new(rmcp::transport::common::client_side_sse::NeverRetry);
            }
            let mut client = wire_http::HttpClient::new(client, responses);
            if let Some(http_io) = check_http_io {
                client = client
                    .with_check_origin(reqwest_012::Url::parse(&server.url)?.origin())
                    .with_check_cancellation(http_io)
                    .with_check_cleanup(cleanup);
            }
            let transport = StreamableHttpClientTransport::with_client(client, config);
            with_connect_timeout(
                &server.name,
                timeout,
                wire_check::serve(peer_client, transport, connection_check),
            )
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?
        }
        _ => anyhow::bail!("unsupported MCP server transport"),
    };
    // rmcp 0.8 accepts arbitrary initialize versions. Retire an unsupported peer before
    // catalog discovery or tool execution can use it.
    if let Err(error) = crate::plugins::validate_mcp_server_protocol(running.peer_info()) {
        let _ = running.cancel().await;
        return Err(error);
    }
    Ok(running)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
