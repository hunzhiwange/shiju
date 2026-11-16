//! No active session or skills; security and filesystem checks use the real domain implementations.
#![allow(unused_variables)]
use std::{path::{Path, PathBuf}, sync::Arc};
use serde_json::Value;
use tool_context::{context::ToolUseContext, host::ContextHost, PermissionDecision};
use tool_types::{Tool, ToolResult};
#[derive(Clone)]
pub struct FileReadHost;
tokio::task_local! { static CONTEXT: Arc<ToolUseContext<FileReadHost>>; }
impl ContextHost for FileReadHost {
type SessionSkillSources= ();
type AsyncHooks= ();
type EffectiveDefaults= ();
type PluginSessionSnapshot= ();
type ToolDiscoverySession= ();
type ToolCatalogSnapshot= ();
type ToolCatalog= ();
type InvokedSkillRecord= ();
type ApprovalManager= ();
type HookRunner= ();
type SecurityPolicy= ();
type Observer= ();
type ImageAssetId = ();
type ImageAssetScope = ();
fn task_local() -> &'static tokio::task::LocalKey<Arc<ToolUseContext<Self>>> { &CONTEXT }
fn current_tenant_scope() -> vw_api_transport::TenantScope { panic!("unused host port") }
fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action { panic!("unused host port") }
fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool { panic!("unused host port") }
fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool { panic!("unused host port") }
fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>> { panic!("unused host port") }
fn security_workspace(security: &Self::SecurityPolicy) -> PathBuf { panic!("unused host port") }
fn image_scope_key(scope: &Self::ImageAssetScope) -> String { panic!("unused host port") }
fn image_id_str(id: &Self::ImageAssetId) -> &str { panic!("unused host port") }
#[cfg(not(target_arch = "wasm32"))]
fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>> { panic!("unused host port") }
fn ensure_ready(context: &ToolUseContext<Self>) -> anyhow::Result<()> { panic!("unused host port") }
fn needs_approval(context: &ToolUseContext<Self>, id: &str) -> bool { panic!("unused host port") }
fn can_act(security: &Self::SecurityPolicy) -> bool { panic!("unused host port") }
fn is_rate_limited(security: &Self::SecurityPolicy) -> bool { panic!("unused host port") }
fn is_path_allowed(security: &Self::SecurityPolicy, path: &str) -> bool { panic!("unused host port") }
fn shell_security_decision(name: &str, input: Value, security: &Self::SecurityPolicy, command: &str, approved: bool) -> PermissionDecision { panic!("unused host port") }
async fn permission_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, input: &'a Value) -> anyhow::Result<Option<bool>> { panic!("unused host port") }
fn record_approval(manager: &Self::ApprovalManager, name: &str, display: &Value, response: tool_context::approval::ApprovalResponse, channel: &str) { panic!("unused host port") }
fn prompt_cli(manager: &Self::ApprovalManager, request: &tool_context::approval::ApprovalRequest) -> tool_context::approval::ApprovalResponse { panic!("unused host port") }
fn create_pending(manager: &Self::ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String { panic!("unused host port") }
fn reject_pending(manager: &Self::ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str) { panic!("unused host port") }
async fn await_approval<'a>(manager: &'a Self::ApprovalManager, request: &'a str, sender: &'a str, channel: &'a str, reply_target: &'a str, cancellation: Option<&'a tokio_util::sync::CancellationToken>) -> tool_context::approval::ApprovalResponse { panic!("unused host port") }
async fn pre_hook(runner: &Self::HookRunner, name: String, input: Value) -> tool_context::hooks::PreHookOutcome { panic!("unused host port") }
async fn post_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, result: &'a ToolResult, duration: std::time::Duration) -> () { panic!("unused host port") }
}
impl crate::FileToolHost for FileReadHost {
fn session_skill_allows_read(_: &Path) -> bool { false }
async fn skill_directories() -> Vec<String> { vec![] }
}
