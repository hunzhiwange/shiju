//! MCP 工具共享辅助模块。
//!
//! 本模块封装 MCP server 配置加载、传输连接、server 元信息序列化以及资源内容
//! 规范化输出逻辑。上层工具通过这些函数保持一致的连接生命周期与 JSON 返回结构。

use crate::app::agent::config::schema::{Config, ConfigExt};
use crate::plugins::{
    PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers, plugin_mcp_discovery_options,
};
use agent_client_protocol::{HttpHeader, McpServer};
use anyhow::Context;
use reqwest_012::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::{
    ClientCapabilities, ClientInfo, CreateElicitationRequestParam, CreateElicitationResult,
    CreateMessageRequestParam, CreateMessageResult, ErrorData, ListRootsResult, ResourceContents,
};
use rmcp::service::{NotificationContext, Peer, RequestContext, RoleClient, ServiceExt};
use rmcp::transport::{
    ConfigureCommandExt, SseClientTransport, StreamableHttpClientTransport, TokioChildProcess,
    sse_client::SseClientConfig, streamable_http_client::StreamableHttpClientTransportConfig,
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::Path,
    time::Duration,
};
use tokio::process::Command;

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
    UnsupportedMcpCapability {
        scope: "server",
        capability: "resources.listChanged",
        reason: "vw-agent MCP client does not automatically refresh cached resources from listChanged notifications.",
    },
    UnsupportedMcpCapability {
        scope: "server",
        capability: "prompts.listChanged",
        reason: "vw-agent MCP client does not automatically refresh cached prompts from listChanged notifications.",
    },
    UnsupportedMcpCapability {
        scope: "server",
        capability: "tools.listChanged",
        reason: "vw-agent MCP client does not automatically refresh cached tools from listChanged notifications.",
    },
];

pub(crate) struct NoopClient {
    info: ClientInfo,
}

impl Default for NoopClient {
    fn default() -> Self {
        Self {
            info: ClientInfo {
                protocol_version: rmcp::model::ProtocolVersion::LATEST,
                capabilities: mcp_client_capabilities(),
                client_info: Default::default(),
            },
        }
    }
}

fn mcp_client_capabilities() -> ClientCapabilities {
    ClientCapabilities { experimental: None, roots: None, sampling: None, elicitation: None }
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
        _request: CreateElicitationRequestParam,
        _context: RequestContext<RoleClient>,
    ) -> impl Future<Output = Result<CreateElicitationResult, ErrorData>> + Send + '_ {
        std::future::ready(Err(unsupported_client_capability_error("elicitation")))
    }

    fn on_resource_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        tracing::debug!("ignoring unsupported MCP resources.listChanged notification");
        std::future::ready(())
    }

    fn on_tool_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        tracing::debug!("ignoring unsupported MCP tools.listChanged notification");
        std::future::ready(())
    }

    fn on_prompt_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) -> impl Future<Output = ()> + Send + '_ {
        tracing::debug!("ignoring unsupported MCP prompts.listChanged notification");
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
pub async fn load_resolved_acp_config() -> anyhow::Result<vw_acp::ResolvedAcpxConfig> {
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
) -> anyhow::Result<vw_acp::ResolvedAcpxConfig> {
    let cwd = cwd.as_ref();
    let mut config = vw_acp::load_resolved_config(cwd).await.map_err(anyhow::Error::from)?;
    merge_runtime_mcp_servers(&mut config, cwd).await?;
    Ok(config)
}

pub(crate) async fn load_resolved_acp_config_from_with_plugin_options(
    cwd: impl AsRef<Path>,
    plugin_options: &PluginDiscoveryOptions,
) -> anyhow::Result<vw_acp::ResolvedAcpxConfig> {
    let mut config = vw_acp::load_resolved_config(cwd).await.map_err(anyhow::Error::from)?;
    let agent_config = merge_runtime_global_mcp_servers(&mut config).await?;
    merge_enabled_plugin_mcp_servers(&mut config.mcp_servers, plugin_options)?;
    apply_global_mcp_server_overrides_from_agent_config(&mut config, &agent_config)?;
    Ok(config)
}

async fn merge_runtime_mcp_servers(
    config: &mut vw_acp::ResolvedAcpxConfig,
    workspace: &Path,
) -> anyhow::Result<()> {
    let mut agent_config = match Config::load_or_init().await {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("skipping runtime MCP servers: failed to load agent config: {error}");
            return Ok(());
        }
    };
    merge_global_mcp_servers_from_agent_config(config, &agent_config)?;
    agent_config.workspace_dir = workspace.to_path_buf();
    let plugin_options = plugin_mcp_discovery_options(&agent_config);
    merge_enabled_plugin_mcp_servers(&mut config.mcp_servers, &plugin_options)?;
    apply_global_mcp_server_overrides_from_agent_config(config, &agent_config)
}

async fn merge_runtime_global_mcp_servers(
    config: &mut vw_acp::ResolvedAcpxConfig,
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
    config: &mut vw_acp::ResolvedAcpxConfig,
    agent_config: &Config,
) -> anyhow::Result<()> {
    apply_mcp_server_override_values(
        config,
        &agent_config.mcp_server_overrides,
        &agent_config.config_path.to_string_lossy(),
    )
}

fn merge_global_mcp_servers_from_agent_config(
    config: &mut vw_acp::ResolvedAcpxConfig,
    agent_config: &Config,
) -> anyhow::Result<()> {
    merge_global_mcp_server_values(
        config,
        &agent_config.mcp_servers,
        &agent_config.config_path.to_string_lossy(),
    )
}

fn merge_global_mcp_server_values(
    config: &mut vw_acp::ResolvedAcpxConfig,
    raw_servers: &[Value],
    source_path: &str,
) -> anyhow::Result<()> {
    if raw_servers.is_empty() {
        return Ok(());
    }

    let parsed = vw_acp::parse_mcp_server_configs(&Value::Array(raw_servers.to_vec()), source_path)
        .map_err(anyhow::Error::from)?;
    let mut merged_names =
        config.mcp_servers.iter().map(server_name).map(ToString::to_string).collect::<HashSet<_>>();

    for vw_acp::McpServerConfig { server, management } in parsed {
        let name = server_name(&server);
        if !merged_names.insert(name.to_string()) {
            tracing::warn!(
                server = name,
                "skipping global MCP server because ACP config already defines it"
            );
            continue;
        }
        config.mcp_servers.push(server);
        config.mcp_server_management.push(management);
    }

    Ok(())
}

#[derive(Debug, Default)]
struct McpServerManagementOverride {
    name: String,
    enabled: Option<bool>,
    required: Option<bool>,
    timeout_ms: Option<Option<u64>>,
    allowed_tools: Option<Vec<String>>,
    denied_tools: Option<Vec<String>>,
}

fn parse_override_bool(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> anyhow::Result<Option<bool>> {
    match object.get(key) {
        None => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("Invalid {path}.{key}: expected boolean")),
    }
}

fn parse_override_timeout(
    object: &serde_json::Map<String, Value>,
    path: &str,
) -> anyhow::Result<Option<Option<u64>>> {
    let Some(value) = object.get("timeout") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(Some(None));
    }
    let Some(seconds) = value.as_f64() else {
        anyhow::bail!("Invalid {path}.timeout: expected positive seconds or null");
    };
    if !seconds.is_finite() || seconds <= 0.0 {
        anyhow::bail!("Invalid {path}.timeout: expected positive seconds or null");
    }
    Ok(Some(Some((seconds * 1_000.0).round() as u64)))
}

fn parse_override_tools(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> anyhow::Result<Option<Vec<String>>> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    let Some(array) = value.as_array() else {
        anyhow::bail!("Invalid {path}.{key}: expected array");
    };
    let mut tools = Vec::with_capacity(array.len());
    for (index, value) in array.iter().enumerate() {
        match value.as_str().map(str::trim).filter(|text| !text.is_empty()) {
            Some(tool) => tools.push(tool.to_string()),
            None => anyhow::bail!("Invalid {path}.{key}[{index}]: expected non-empty string"),
        }
    }
    Ok(Some(tools))
}

fn parse_mcp_server_management_override(
    value: &Value,
    source_path: &str,
    index: usize,
) -> anyhow::Result<McpServerManagementOverride> {
    let path = format!("mcpServerOverrides[{index}] in {source_path}");
    let object =
        value.as_object().ok_or_else(|| anyhow::anyhow!("Invalid {path}: expected object"))?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Invalid {path}.name: expected non-empty string"))?
        .to_string();

    Ok(McpServerManagementOverride {
        name,
        enabled: parse_override_bool(object, "enabled", &path)?,
        required: parse_override_bool(object, "required", &path)?,
        timeout_ms: parse_override_timeout(object, &path)?,
        allowed_tools: parse_override_tools(object, "allowedTools", &path)?,
        denied_tools: parse_override_tools(object, "deniedTools", &path)?,
    })
}

fn apply_mcp_management_override(
    management: &mut vw_acp::McpServerManagement,
    override_entry: McpServerManagementOverride,
) {
    if let Some(enabled) = override_entry.enabled {
        management.enabled = enabled;
    }
    if let Some(required) = override_entry.required {
        management.required = required;
    }
    if let Some(timeout_ms) = override_entry.timeout_ms {
        management.timeout_ms = timeout_ms;
    }
    if let Some(allowed_tools) = override_entry.allowed_tools {
        management.allowed_tools = allowed_tools;
    }
    if let Some(denied_tools) = override_entry.denied_tools {
        management.denied_tools = denied_tools;
    }
}

fn apply_mcp_server_override_values(
    config: &mut vw_acp::ResolvedAcpxConfig,
    raw_overrides: &[Value],
    source_path: &str,
) -> anyhow::Result<()> {
    if raw_overrides.is_empty() {
        return Ok(());
    }

    let server_names =
        config.mcp_servers.iter().map(server_name).map(ToString::to_string).collect::<HashSet<_>>();
    for (index, value) in raw_overrides.iter().enumerate() {
        let override_entry = parse_mcp_server_management_override(value, source_path, index)?;
        if !server_names.contains(&override_entry.name) {
            tracing::warn!(
                server = override_entry.name.as_str(),
                "skipping MCP server override because no runtime server has this name"
            );
            continue;
        }

        let mut management = config
            .mcp_server_management
            .iter()
            .find(|entry| entry.name == override_entry.name)
            .cloned()
            .unwrap_or_else(|| {
                vw_acp::McpServerManagement::default_for_server(override_entry.name.clone())
            });
        apply_mcp_management_override(&mut management, override_entry);

        if let Some(existing) =
            config.mcp_server_management.iter_mut().find(|entry| entry.name == management.name)
        {
            *existing = management;
        } else {
            config.mcp_server_management.push(management);
        }
    }

    Ok(())
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
    let connections = super::context::current_tool_use_context().and_then(|context| {
        context
            .workspace_root()
            .map(|root| crate::plugins::mcp_session_connections(context.session(), &root))
    });
    match connections {
        Some(connections) => connections.with_peer(server, timeout, f).await,
        None => {
            crate::plugins::McpSessionConnections::default().with_peer(server, timeout, f).await
        }
    }
}

pub(crate) type RunningMcpPeer = rmcp::service::RunningService<rmcp::RoleClient, NoopClient>;

/// The owner runs this handshake on a durable runtime, separate from synchronous tool bridges.
pub(crate) async fn open_server_peer(
    server: McpServer,
    timeout: Duration,
) -> anyhow::Result<RunningMcpPeer> {
    let server = &server;
    match server {
        McpServer::Stdio(server) => {
            let mut command = Command::new(&server.command);
            if let Some(cwd) = crate::plugins::plugin_mcp_runtime_cwd(server)? {
                command.current_dir(cwd);
            }
            let args = server.args.clone();
            let env = server
                .env
                .iter()
                .map(|env| (env.name.clone(), env.value.clone()))
                .collect::<Vec<_>>();
            let transport = TokioChildProcess::new(command.configure(move |cmd| {
                cmd.args(args);
                for (key, value) in &env {
                    cmd.env(key, value);
                }
            }))?;
            let running = with_connect_timeout(&server.name, timeout, async move {
                NoopClient::default().serve(transport).await.map_err(anyhow::Error::from)
            })
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?;
            Ok(running)
        }
        McpServer::Sse(server) => {
            let client = remote_http_client(&server.headers, timeout)
                .with_context(|| format!("failed to build SSE MCP client for '{}'", server.name))?;
            let config =
                SseClientConfig { sse_endpoint: server.url.clone().into(), ..Default::default() };
            let transport = with_connect_timeout(&server.name, timeout, async move {
                SseClientTransport::start_with_client(client, config)
                    .await
                    .map_err(anyhow::Error::from)
            })
            .await
            .with_context(|| format!("failed to start SSE MCP transport for '{}'", server.name))?;
            let running = with_connect_timeout(&server.name, timeout, async move {
                NoopClient::default().serve(transport).await.map_err(anyhow::Error::from)
            })
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?;
            Ok(running)
        }
        McpServer::Http(server) => {
            let scoped_headers = crate::plugins::plugin_mcp_http_headers(server)?;
            let client = if let Some(headers) = scoped_headers {
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
            let transport = StreamableHttpClientTransport::with_client(
                client,
                StreamableHttpClientTransportConfig::with_uri(server.url.clone()),
            );
            let running = with_connect_timeout(&server.name, timeout, async move {
                NoopClient::default().serve(transport).await.map_err(anyhow::Error::from)
            })
            .await
            .with_context(|| format!("failed to initialize MCP peer '{}'", server.name))?;
            Ok(running)
        }
        _ => anyhow::bail!("unsupported MCP server transport"),
    }
}

pub fn management_for_server(
    config: &vw_acp::ResolvedAcpxConfig,
    server: &McpServer,
) -> vw_acp::McpServerManagement {
    let name = server_name(server);
    config
        .mcp_server_management
        .iter()
        .find(|management| management.name == name)
        .cloned()
        .unwrap_or_else(|| vw_acp::McpServerManagement::default_for_server(name))
}

pub fn management_timeout(management: &vw_acp::McpServerManagement) -> Option<Duration> {
    management.timeout_ms.map(Duration::from_millis)
}

pub fn mcp_tool_allowed(
    management: &vw_acp::McpServerManagement,
    tool_name: &str,
    tool_id: &str,
) -> bool {
    if management.denied_tools.iter().any(|entry| tool_policy_matches(entry, tool_name, tool_id)) {
        return false;
    }
    management.allowed_tools.is_empty()
        || management
            .allowed_tools
            .iter()
            .any(|entry| tool_policy_matches(entry, tool_name, tool_id))
}

pub fn auth_method_names(auth: &HashMap<String, String>) -> Vec<String> {
    let mut methods = auth.keys().cloned().collect::<Vec<_>>();
    methods.sort_unstable();
    methods
}

fn tool_policy_matches(entry: &str, tool_name: &str, tool_id: &str) -> bool {
    entry == tool_name || entry == tool_id
}

/// 返回 MCP server 的稳定名称。
///
/// # 参数
///
/// - `server`: MCP server 配置枚举。
///
/// # 返回值
///
/// 对受支持传输返回配置中的名称；未知传输返回 `"unknown"`，用于诊断输出。
pub fn server_name(server: &McpServer) -> &str {
    match server {
        McpServer::Http(server) => &server.name,
        McpServer::Sse(server) => &server.name,
        McpServer::Stdio(server) => &server.name,
        _ => "unknown",
    }
}

pub fn server_transport(server: &McpServer) -> &'static str {
    match server {
        McpServer::Http(_) => "http",
        McpServer::Sse(_) => "sse",
        McpServer::Stdio(_) => "stdio",
        _ => "unknown",
    }
}

/// 按稳定名称查找 MCP server。
///
/// # 参数
///
/// - `servers`: 已解析配置中的 MCP server 列表。
/// - `name`: 用户提供的 server 名称，首尾空白会被忽略。
///
/// # 返回值
///
/// 找到时返回匹配的 server 引用。
///
/// # 错误
///
/// 当名称为空或配置中不存在对应 server 时返回明确错误。
pub fn find_server_by_name<'a>(
    servers: &'a [McpServer],
    name: &str,
) -> anyhow::Result<&'a McpServer> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("MCP server name must not be empty");
    }
    servers
        .iter()
        .find(|server| server_name(server) == name)
        .ok_or_else(|| anyhow::anyhow!("unknown MCP server '{}'", name))
}

/// 将 MCP server 的关键连接信息转换为可展示的 JSON 摘要。
///
/// # 参数
///
/// - `server`: MCP server 配置枚举。
///
/// # 返回值
///
/// 返回包含名称、传输类型及非敏感连接字段的 JSON 值。远程 headers 与 stdio 环境变量
/// 不会写入摘要，避免把潜在敏感配置暴露给工具输出。
pub fn server_summary(server: &McpServer) -> Value {
    match server {
        McpServer::Http(server) => json!({
            "name": server.name,
            "transport": "http",
            "url": server.url,
        }),
        McpServer::Sse(server) => json!({
            "name": server.name,
            "transport": "sse",
            "url": server.url,
        }),
        McpServer::Stdio(server) => json!({
            "name": server.name,
            "transport": "stdio",
            "command": server.command,
            "args": server.args,
        }),
        _ => json!({
            "name": "unknown",
            "transport": "unknown",
        }),
    }
}

/// 将 MCP 资源内容转换为工具输出使用的 JSON 数组。
///
/// # 参数
///
/// - `contents`: MCP `read_resource` 返回的资源内容列表。
///
/// # 返回值
///
/// 每个资源会保留 URI、MIME 类型、内容载荷以及 `kind` 字段，方便调用端区分文本与
/// 二进制资源。
pub fn resource_contents_json(contents: &[ResourceContents]) -> Value {
    Value::Array(
        contents
            .iter()
            .map(|content| match content {
                ResourceContents::TextResourceContents { uri, mime_type, text, .. } => {
                    json!({
                        "uri": uri,
                        "mime_type": mime_type,
                        "text": text,
                        "kind": "text",
                    })
                }
                ResourceContents::BlobResourceContents { uri, mime_type, blob, .. } => {
                    json!({
                        "uri": uri,
                        "mime_type": mime_type,
                        "blob": blob,
                        "kind": "blob",
                    })
                }
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
