//! Repository-only fixture. Any unexpected host capability fails closed.
use tool_context::{host::{ContextHost, HostFuture}, context::ToolUseContext, PermissionDecision};
use std::{path::{Path, PathBuf}, sync::Arc};
use tool_types::{Tool, ToolResult};
use serde_json::Value;
#[derive(Clone)]
pub(super) struct RepositoryHost;
tokio::task_local! { static CONTEXT: Arc<ToolUseContext<RepositoryHost>>; }
#[allow(unused_variables)]
impl ContextHost for RepositoryHost {
    type SessionSkillSources = ();
    type AsyncHooks = ();
    type EffectiveDefaults = ();
    type PluginSessionSnapshot = ();
    type ToolDiscoverySession = ();
    type ToolCatalogSnapshot = ();
    type ToolCatalog = ();
    type InvokedSkillRecord = ();
    type ApprovalManager = ();
    type HookRunner = ();
    type SecurityPolicy = ();
    type Observer = ();
    type ImageAssetId = ();
    type ImageAssetScope = ();
    fn task_local() -> &'static tokio::task::LocalKey<Arc<ToolUseContext<Self>>> { &CONTEXT }
    fn current_tenant_scope() -> vw_api_transport::TenantScope { panic!("repository test reached host: current_tenant_scope") }
    fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action { panic!("repository test reached host: permission_action") }
    fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool { panic!("repository test reached host: catalog_authorizes") }
    fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool { panic!("repository test reached host: discovery_authorizes") }
    fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>> { panic!("repository test reached host: catalog_registry") }
    fn security_workspace(security: &Self::SecurityPolicy) -> PathBuf { panic!("repository test reached host: security_workspace") }
    fn image_scope_key(scope: &Self::ImageAssetScope) -> String { panic!("repository test reached host: image_scope_key") }
    fn image_id_str(id: &Self::ImageAssetId) -> &str { panic!("repository test reached host: image_id_str") }
    #[cfg(not(target_arch = "wasm32"))]
    fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>> { panic!("repository test reached host: use_lease") }
    fn ensure_ready(context: &ToolUseContext<Self>) -> anyhow::Result<()> { panic!("repository test reached host: ensure_ready") }
    fn needs_approval(context: &ToolUseContext<Self>, id: &str) -> bool { panic!("repository test reached host: needs_approval") }
    fn can_act(security: &Self::SecurityPolicy) -> bool { panic!("repository test reached host: can_act") }
    fn is_rate_limited(security: &Self::SecurityPolicy) -> bool { panic!("repository test reached host: is_rate_limited") }
    fn is_path_allowed(security: &Self::SecurityPolicy, path: &str) -> bool { panic!("repository test reached host: is_path_allowed") }
    fn shell_security_decision(name: &str, input: Value, security: &Self::SecurityPolicy, command: &str, approved: bool) -> PermissionDecision { panic!("repository test reached host: shell_security_decision") }
    fn permission_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, input: &'a Value) -> impl Future<Output = anyhow::Result<Option<bool>>> + HostFuture + 'a { async { panic!("repository test reached host: permission_hook") } }
    fn record_approval(manager: &Self::ApprovalManager, name: &str, display: &Value, response: tool_context::approval::ApprovalResponse, channel: &str) { panic!("repository test reached host: record_approval") }
    fn prompt_cli(manager: &Self::ApprovalManager, request: &tool_context::approval::ApprovalRequest) -> tool_context::approval::ApprovalResponse { panic!("repository test reached host: prompt_cli") }
    fn create_pending(manager: &Self::ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String { panic!("repository test reached host: create_pending") }
    fn reject_pending(manager: &Self::ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str) { panic!("repository test reached host: reject_pending") }
    fn await_approval<'a>(manager: &'a Self::ApprovalManager, request: &'a str, sender: &'a str, channel: &'a str, reply_target: &'a str, cancellation: Option<&'a tokio_util::sync::CancellationToken>) -> impl Future<Output = tool_context::approval::ApprovalResponse> + HostFuture + 'a { async { panic!("repository test reached host: await_approval") } }
    fn pre_hook(runner: &Self::HookRunner, name: String, input: Value) -> impl Future<Output = tool_context::hooks::PreHookOutcome> + HostFuture { async { panic!("repository test reached host: pre_hook") } }
    fn post_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, result: &'a ToolResult, duration: std::time::Duration) -> impl Future<Output = ()> + HostFuture + 'a { async { panic!("repository test reached host: post_hook") } }
}
