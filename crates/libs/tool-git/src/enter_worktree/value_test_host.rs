//! Fail-closed host for input/specification checks; no permission or runtime mocks.
use std::{path::{Path, PathBuf}, sync::Arc};
use serde_json::Value;
use tool_context::{context::ToolUseContext, host::{ContextHost, HostFuture}, PermissionDecision};
use tool_types::{Tool, ToolResult};

#[derive(Clone)]
pub(super) struct InputHost;
tokio::task_local! { static CONTEXT: Arc<ToolUseContext<InputHost>>; }

#[allow(unused_variables)]
impl ContextHost for InputHost {

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
    fn current_tenant_scope() -> vw_api_transport::TenantScope { panic!("local contract test reached host service") }
    fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action { panic!("local contract test reached host service") }
    fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool { panic!("local contract test reached host service") }
    fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool { panic!("local contract test reached host service") }
    fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>> { panic!("local contract test reached host service") }
    fn security_workspace(security: &Self::SecurityPolicy) -> PathBuf { panic!("local contract test reached host service") }
    fn image_scope_key(scope: &Self::ImageAssetScope) -> String { panic!("local contract test reached host service") }
    fn image_id_str(id: &Self::ImageAssetId) -> &str { panic!("local contract test reached host service") }
    #[cfg(not(target_arch = "wasm32"))]
    fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>> { panic!("local contract test reached host service") }
    fn ensure_ready(context: &ToolUseContext<Self>) -> anyhow::Result<()> { panic!("local contract test reached host service") }
    fn needs_approval(context: &ToolUseContext<Self>, id: &str) -> bool { panic!("local contract test reached host service") }
    fn can_act(security: &Self::SecurityPolicy) -> bool { panic!("local contract test reached host service") }
    fn is_rate_limited(security: &Self::SecurityPolicy) -> bool { panic!("local contract test reached host service") }
    fn is_path_allowed(security: &Self::SecurityPolicy, path: &str) -> bool { panic!("local contract test reached host service") }
    fn shell_security_decision(name: &str, input: Value, security: &Self::SecurityPolicy, command: &str, approved: bool) -> PermissionDecision { panic!("local contract test reached host service") }
    fn permission_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, input: &'a Value) -> impl Future<Output = anyhow::Result<Option<bool>>> + HostFuture + 'a { async move { panic!("local contract test reached host service") } }
    fn record_approval(manager: &Self::ApprovalManager, name: &str, display: &Value, response: tool_context::approval::ApprovalResponse, channel: &str) { panic!("local contract test reached host service") }
    fn prompt_cli(manager: &Self::ApprovalManager, request: &tool_context::approval::ApprovalRequest) -> tool_context::approval::ApprovalResponse { panic!("local contract test reached host service") }
    fn create_pending(manager: &Self::ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String { panic!("local contract test reached host service") }
    fn reject_pending(manager: &Self::ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str) { panic!("local contract test reached host service") }
    fn await_approval<'a>(manager: &'a Self::ApprovalManager, request: &'a str, sender: &'a str, channel: &'a str, reply_target: &'a str, cancellation: Option<&'a tokio_util::sync::CancellationToken>) -> impl Future<Output = tool_context::approval::ApprovalResponse> + HostFuture + 'a { async move { panic!("local contract test reached host service") } }
    fn pre_hook(runner: &Self::HookRunner, name: String, input: Value) -> impl Future<Output = tool_context::hooks::PreHookOutcome> + HostFuture { async move { panic!("local contract test reached host service") } }
    fn post_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, result: &'a ToolResult, duration: std::time::Duration) -> impl Future<Output = ()> + HostFuture + 'a { async move { panic!("local contract test reached host service") } }
}
impl crate::GitToolHost for InputHost {
    #[cfg(not(target_arch = "wasm32"))]
    fn initialize_worktree_host() { panic!("local contract test reached worktree host") }
}
