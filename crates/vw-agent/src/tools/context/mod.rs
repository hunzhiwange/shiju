//! Concrete host capability wiring for the shared tool runtime.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod run_worktree;
pub(crate) mod shell_policy;
use std::{path::{Path, PathBuf}, sync::Arc};
use serde_json::Value;
use super::Tool;
use crate::app::agent::multimodal::assets::{ImageAssetId, ImageAssetScope};
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::hooks::HookRunner;
use crate::app::agent::security::SecurityPolicy;

use tool_context::host::ContextHost;
pub use tool_context::context::{WorkflowState, WorktreeBindingState};
pub use tool_context::{MAX_PENDING_IMAGE_RELOADS, PendingImageReloadQueue};
pub use super::read_state::{FileReadStateCache, FileReadStateEntry, FileSnapshot};
pub type ToolUseContext = tool_context::context::ToolUseContext<AgentContextHost>;
#[derive(Clone)]
pub struct AgentContextHost;
tokio::task_local! { static ACTIVE_TOOL_USE_CONTEXT: Arc<ToolUseContext>; }
impl ContextHost for AgentContextHost {
    type SessionSkillSources = crate::acp::mounts::SessionSkillSources;
    type AsyncHooks = crate::plugins::AsyncHooks;
    type EffectiveDefaults = crate::agent::defaults::EffectiveDefaults;
    type PluginSessionSnapshot = crate::plugins::session_snapshot::PluginSessionSnapshot;
    type ToolDiscoverySession = super::tool_search::ToolDiscoverySession;
    type ToolCatalogSnapshot = super::catalog::ToolCatalogSnapshot;
    type ToolCatalog = super::catalog::ToolCatalog;
    type InvokedSkillRecord = crate::skills::InvokedSkillRecord;
    type ApprovalManager = ApprovalManager;
    type HookRunner = HookRunner;
    type SecurityPolicy = SecurityPolicy;
    type Observer = dyn crate::observability::Observer;
    type ImageAssetId = ImageAssetId;
    type ImageAssetScope = ImageAssetScope;
    fn task_local() -> &'static tokio::task::LocalKey<Arc<ToolUseContext>> { &ACTIVE_TOOL_USE_CONTEXT }
    fn current_tenant_scope() -> vw_api_transport::TenantScope { crate::session::ui_store::current_tenant_scope() }
    fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action { policy.permission_action_with_aliases(permission, aliases, pattern) }
    fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool { catalog.authorizes(id) }
    fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool { discovery.authorizes(id) }
    fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>> { super::toolset::boxed_registry_from_arcs(catalog.tools().to_vec()) }
    fn security_workspace(security: &SecurityPolicy) -> PathBuf { security.workspace_dir.clone() }
    fn image_scope_key(scope: &ImageAssetScope) -> String { scope.key().to_string() }
    fn image_id_str(id: &ImageAssetId) -> &str { id.as_str() }
    #[cfg(not(target_arch = "wasm32"))]
    fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>> { crate::worktree::registry::use_lease(path) }
    fn ensure_ready(context: &ToolUseContext) -> anyhow::Result<()> {
        if let Some(root) = context.workspace_root() { crate::plugins::mcp_session_connections(context.session(), &root).ensure_ready()?; }
        Ok(())
    }
    fn needs_approval(context: &ToolUseContext, id: &str) -> bool { context.approval_manager().is_some_and(|manager| manager.needs_approval(id)) }
    fn can_act(security: &SecurityPolicy) -> bool { security.can_act() }
    fn is_rate_limited(security: &SecurityPolicy) -> bool { security.is_rate_limited() }
    fn is_path_allowed(security: &SecurityPolicy, path: &str) -> bool { security.is_path_allowed(path) }
    fn shell_security_decision(name: &str, input: Value, security: &SecurityPolicy, command: &str, approved: bool) -> tool_context::PermissionDecision { shell_policy::shell_security_decision(name, input, security, command, approved) }
    async fn permission_hook(runner: &HookRunner, name: &str, input: &Value) -> anyhow::Result<Option<bool>> { runner.run_permission_request(name, input).await }
    fn record_approval(manager: &ApprovalManager, name: &str, display: &Value, response: tool_context::approval::ApprovalResponse, channel: &str) { manager.record_decision(name, display, response, channel); }
    fn prompt_cli(manager: &ApprovalManager, request: &tool_context::approval::ApprovalRequest) -> tool_context::approval::ApprovalResponse { manager.prompt_cli(request) }
    fn create_pending(manager: &ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String { manager.create_non_cli_pending_request(name, sender, channel, reply_target, reason, display, message_id, call_id).request_id }
    fn reject_pending(manager: &ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str) { let _ = manager.reject_non_cli_pending_request(request, sender, channel, reply_target); }
    async fn await_approval(manager: &ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str, cancellation: Option<&tokio_util::sync::CancellationToken>) -> tool_context::approval::ApprovalResponse { crate::agent::loop_::approval::await_non_cli_approval_decision(manager, request, sender, channel, reply_target, cancellation).await }
    async fn pre_hook(runner: &HookRunner, name: String, input: Value) -> tool_context::hooks::PreHookOutcome {
        match runner.run_before_tool_call(name, input).await {
            crate::hooks::HookResult::Continue(value) => tool_context::hooks::PreHookOutcome::Continue(value),
            crate::hooks::HookResult::Cancel(reason) => tool_context::hooks::PreHookOutcome::Cancel(reason),
        }
    }
    async fn post_hook(runner: &HookRunner, name: &str, result: &super::ToolResult, duration: std::time::Duration) { runner.fire_after_tool_call(name, result, duration).await }

}
pub async fn scope_tool_use_context<F: std::future::Future>(context: Arc<ToolUseContext>, future: F) -> F::Output { tool_context::context::scope_tool_use_context(context, future).await }
pub fn scope_tool_use_context_sync<T>(context: Arc<ToolUseContext>, body: impl FnOnce() -> T) -> T { tool_context::context::scope_tool_use_context_sync(context, body) }
pub fn current_tool_use_context() -> Option<Arc<ToolUseContext>> { tool_context::context::current_tool_use_context::<AgentContextHost>() }
pub fn enqueue_current_pending_image_reload(id: &ImageAssetId) -> Result<(), String> { tool_context::context::enqueue_current_pending_image_reload::<AgentContextHost>(id) }
pub fn current_read_state_for_path(path: impl AsRef<Path>) -> Option<FileReadStateEntry> { tool_context::context::current_read_state_for_path::<AgentContextHost>(path) }
pub fn note_current_read_state(path: impl AsRef<Path>, bytes_read: usize, partial_view: bool, offset: Option<usize>, limit: Option<usize>, snapshot: Option<FileSnapshot>) -> Option<PathBuf> { tool_context::context::note_current_read_state::<AgentContextHost>(path, bytes_read, partial_view, offset, limit, snapshot) }
pub fn invalidate_current_read_state(path: impl AsRef<Path>) -> Option<FileReadStateEntry> { tool_context::context::invalidate_current_read_state::<AgentContextHost>(path) }
pub fn normalize_path_for_context(context: &ToolUseContext, path: impl AsRef<Path>) -> PathBuf { tool_context::context::normalize_path_for_context(context, path) }
