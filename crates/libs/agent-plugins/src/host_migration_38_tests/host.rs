//! Repository-only fixture. Any unexpected host capability fails closed.
use tool_context::{host::{ContextHost, HostFuture}, context::ToolUseContext, PermissionDecision};
use std::{path::{Path, PathBuf}, sync::Arc};
use tool_types::{Tool, ToolResult};
use serde_json::Value;
#[derive(Clone)]
pub(crate) struct Host;
tokio::task_local! { static CONTEXT: Arc<ToolUseContext<Host>>; }
#[allow(unused_variables)]
impl ContextHost for Host {
    type SessionSkillSources = ();
    type AsyncHooks = plugin_hooks::hook_async::AsyncHooks;
    type EffectiveDefaults = ();
    type PluginSessionSnapshot = ();
    type ToolDiscoverySession = ();
    type ToolCatalogSnapshot = ();
    type ToolCatalog = ();
    type InvokedSkillRecord = agent_skills::invoked::InvokedSkillRecord;
    type ApprovalManager = ();
    type HookRunner = agent_hooks::HookRunner;
    type SecurityPolicy = agent_security::SecurityPolicy;
    type Observer = ();
    type ImageAssetId = agent_multimodal::assets::ImageAssetId;
    type ImageAssetScope = agent_multimodal::assets::ImageAssetScope;
    fn task_local() -> &'static tokio::task::LocalKey<Arc<ToolUseContext<Self>>> { &CONTEXT }
    fn current_tenant_scope() -> vw_api_transport::TenantScope { vw_api_transport::TenantScope::new("fixture-platform", "toolset-tests").unwrap() }
    fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action { panic!("toolset test reached host: permission_action") }
    fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool { panic!("unused catalog port") }
    fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool { panic!("toolset test reached host: discovery_authorizes") }
    fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>> { panic!("toolset test reached host: catalog_registry") }
    fn security_workspace(security: &Self::SecurityPolicy) -> PathBuf { security.workspace_dir.clone() }
    fn image_scope_key(scope: &Self::ImageAssetScope) -> String { scope.key().to_string() }
    fn image_id_str(id: &Self::ImageAssetId) -> &str { id.as_str() }
    #[cfg(not(target_arch = "wasm32"))]
    fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>> { Ok(None) }
    fn ensure_ready(context: &ToolUseContext<Self>) -> anyhow::Result<()> { Ok(()) }
    fn needs_approval(context: &ToolUseContext<Self>, id: &str) -> bool { false }
    fn can_act(security: &Self::SecurityPolicy) -> bool { security.can_act() }
    fn is_rate_limited(security: &Self::SecurityPolicy) -> bool { security.is_rate_limited() }
    fn is_path_allowed(security: &Self::SecurityPolicy, path: &str) -> bool { security.is_path_allowed(path) }
    fn shell_security_decision(name: &str, input: Value, security: &Self::SecurityPolicy, command: &str, approved: bool) -> PermissionDecision { panic!("toolset test reached host: shell_security_decision") }
    fn permission_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, input: &'a Value) -> impl Future<Output = anyhow::Result<Option<bool>>> + HostFuture + 'a { async move { runner.run_permission_request(name, input).await } }
    fn record_approval(manager: &Self::ApprovalManager, name: &str, display: &Value, response: tool_context::approval::ApprovalResponse, channel: &str) { panic!("toolset test reached host: record_approval") }
    fn prompt_cli(manager: &Self::ApprovalManager, request: &tool_context::approval::ApprovalRequest) -> tool_context::approval::ApprovalResponse { panic!("toolset test reached host: prompt_cli") }
    fn create_pending(manager: &Self::ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String { panic!("toolset test reached host: create_pending") }
    fn reject_pending(manager: &Self::ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str) { panic!("toolset test reached host: reject_pending") }
    fn await_approval<'a>(manager: &'a Self::ApprovalManager, request: &'a str, sender: &'a str, channel: &'a str, reply_target: &'a str, cancellation: Option<&'a tokio_util::sync::CancellationToken>) -> impl Future<Output = tool_context::approval::ApprovalResponse> + HostFuture + 'a { async { panic!("toolset test reached host: await_approval") } }
    fn pre_hook(runner: &Self::HookRunner, name: String, input: Value) -> impl Future<Output = tool_context::hooks::PreHookOutcome> + HostFuture { async move { match runner.run_before_tool_call(name, input).await { agent_hooks::HookResult::Continue(value) => tool_context::hooks::PreHookOutcome::Continue(value), agent_hooks::HookResult::Cancel(reason) => tool_context::hooks::PreHookOutcome::Cancel(reason) } } }
    fn post_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, result: &'a ToolResult, duration: std::time::Duration) -> impl Future<Output = ()> + HostFuture + 'a { async move { runner.fire_after_tool_call(name, result, duration).await; } }
}

impl tool_mcp::McpToolHost for Host {
 type Connections = ();
 fn new_connections() {}
 async fn load_config(_: Option<&Path>) -> anyhow::Result<acp_config::ResolvedAcpxConfig> { anyhow::bail!("unused MCP port") }
 fn connection_required(_: &agent_client_protocol::McpServer) -> anyhow::Result<bool> { anyhow::bail!("unused MCP port") }
 fn missing_executable(_: &agent_client_protocol::McpServer) -> anyhow::Result<Option<(String,String)>> { anyhow::bail!("unused MCP port") }
 fn tool_allowed(_: &agent_client_protocol::McpServer, _: &str) -> anyhow::Result<bool> { anyhow::bail!("unused MCP port") }
 fn tool_binding(_: &(), _: &agent_client_protocol::McpServer) -> Option<mcp_client::ToolCatalogBinding> { panic!("unused MCP port") }
 async fn with_client<T: Send,F>(_: &agent_client_protocol::McpServer, _: Option<std::time::Duration>, _: F) -> anyhow::Result<T>
 where F: for<'a> FnOnce(&'a mcp_client::RunningMcpPeer) -> tool_mcp::ClientFuture<'a,T> + Send { anyhow::bail!("unused MCP port") }
 async fn with_connection<T: Send,F>(_: &(), _: &agent_client_protocol::McpServer, _: Option<std::time::Duration>, _: F) -> anyhow::Result<T>
 where F: for<'a> FnOnce(&'a mcp_client::RunningMcpPeer) -> tool_mcp::ClientFuture<'a,T> + Send { anyhow::bail!("unused MCP port") }
 fn app_resource<'a>(_: &'a mcp_client::RunningMcpPeer, _: &'a mut Value) -> tool_mcp::ClientFuture<'a,()> { Box::pin(async { anyhow::bail!("unused MCP port") }) }
 fn call_resource<'a>(_: &'a acp_config::ResolvedAcpxConfig, _: &'a Arc<()>, _: Value) -> tool_mcp::ClientFuture<'a,tool_types::ToolCallResult> { Box::pin(async { anyhow::bail!("unused MCP port") }) }
}
impl plugin_hooks::host::HookHost for Host {
 fn sessions() -> &'static plugin_hooks::hook_session::HookSessions<Self> {
 static S: std::sync::OnceLock<plugin_hooks::hook_session::HookSessions<Host>> = std::sync::OnceLock::new(); S.get_or_init(Default::default)
 }
 fn directory() -> String { panic!("unexpected project lookup") }
 async fn execution_runner(_: &Path) -> anyhow::Result<Option<Arc<agent_hooks::HookRunner>>> { anyhow::bail!("unexpected runner lookup") }
 async fn cleanup(_: &Arc<ToolUseContext<Self>>) { panic!("unexpected cleanup") }
 fn new_runner() -> agent_hooks::HookRunner { agent_hooks::HookRunner::new() }
 fn connections(_: &str, _: &Path) -> Arc<()> { panic!("unused MCP port") }
 fn hook_source_matches(_: &agent_client_protocol::McpServerHttp, _: &Path, _: &str) -> anyhow::Result<bool> { anyhow::bail!("unused MCP port") }
 async fn with_peer<T: Send,F>(_: &(), _: &agent_client_protocol::McpServer, _: Option<std::time::Duration>, _: F) -> anyhow::Result<T>
 where F: for<'a> FnOnce(&'a rmcp::service::Peer<rmcp::service::RoleClient>) -> tool_mcp::ClientFuture<'a,T> + Send { anyhow::bail!("unused MCP port") }
}
