//! MCP 工具共享辅助模块。
//!
//! 本模块封装 MCP server 配置加载、传输连接、server 元信息序列化以及资源内容
//! 规范化输出逻辑。上层工具通过这些函数保持一致的连接生命周期与 JSON 返回结构。

use crate::app::agent::config::schema::{Config, ConfigExt};
use crate::plugins::{
    PluginDiscoveryOptions, merge_enabled_plugin_mcp_servers, plugin_mcp_discovery_options,
};
use agent_client_protocol::{McpServer, McpServerStdio};
use rmcp::service::{Peer, RoleClient};
use serde_json::Value;
use std::{future::Future, path::Path, time::Duration};

pub(crate) use mcp_client::client::{RunningMcpPeer, unsupported_mcp_capabilities_json};

pub use mcp_client::{
    auth_method_names, find_server_by_name, management_for_server, management_timeout,
    mcp_tool_allowed, resource_contents_json, server_name, server_summary, server_transport,
};
use mcp_client::{apply_mcp_server_override_values, merge_global_mcp_server_values};

mod wire_http;
mod wire_stdio;

#[cfg(not(target_arch = "wasm32"))]
use wire_stdio::ProbeChild;

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
    mcp_client::open_server_peer(std::sync::Arc::new(AgentMcpHost), server, timeout, closed, form_owner).await
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
    mcp_client::open_connection_check_peer(
        std::sync::Arc::new(AgentMcpHost),
        server,
        timeout,
        http_io,
        child,
        cleanup,
    )
    .await
}

struct AgentMcpHost;
impl mcp_client::client::ClientHost for AgentMcpHost {
    fn runtime_stdio_cwd(&self, server: &McpServerStdio) -> anyhow::Result<Option<std::path::PathBuf>> {
        runtime_stdio_cwd(server)
    }
    fn http_headers<'a>(&'a self, server: &'a agent_client_protocol::McpServerHttp,
        owner: Option<&'a mcp_client::FormOwner>)
        -> std::pin::Pin<Box<dyn Future<Output = anyhow::Result<Option<reqwest_012::header::HeaderMap>>> + Send + 'a>> {
        Box::pin(async move {
            let oauth = match owner {
                Some(owner) => crate::plugins::plugin_mcp_oauth_access(server, owner).await?,
                None => None,
            };
            match oauth.as_deref() {
                Some(token) => crate::plugins::plugin_mcp_oauth_headers(server, Some(token)),
                None => crate::plugins::plugin_mcp_http_headers(server),
            }
        })
    }
    fn request_elicitation(&self, owner: mcp_client::FormOwner, server: String,
        request: rmcp::model::CreateElicitationRequestParam, cancelled: tokio_util::sync::CancellationToken,
        closed: tokio_util::sync::CancellationToken, wire_schema: Option<Value>)
        -> std::pin::Pin<Box<dyn Future<Output = Result<rmcp::model::CreateElicitationResult, rmcp::model::ErrorData>> + Send + '_>> {
        Box::pin(crate::plugins::mcp_elicitation::request(owner, server, request, cancelled, closed, wire_schema))
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
