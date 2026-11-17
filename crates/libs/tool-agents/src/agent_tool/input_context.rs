//! Input/specification tests must never invoke runtime capabilities.
#![allow(unused_variables)]
use std::{path::{Path, PathBuf}, sync::Arc};
use tool_context::{context::ToolUseContext, host::{ContextHost, HostFuture}, PermissionDecision};
use tool_types::{Tool, ToolResult};
use serde_json::Value;
#[derive(Clone)]
pub(crate) struct InputContext;
impl ContextHost for InputContext {
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
fn task_local() -> &'static tokio::task::LocalKey<Arc<ToolUseContext<Self>>> { panic!("input/specification test reached context service: task_local") }
fn current_tenant_scope() -> vw_api_transport::TenantScope { panic!("input/specification test reached context service: current_tenant_scope") }
fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action { panic!("input/specification test reached context service: permission_action") }
fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool { panic!("input/specification test reached context service: catalog_authorizes") }
fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool { panic!("input/specification test reached context service: discovery_authorizes") }
fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>> { panic!("input/specification test reached context service: catalog_registry") }
fn security_workspace(security: &Self::SecurityPolicy) -> PathBuf { panic!("input/specification test reached context service: security_workspace") }
fn image_scope_key(scope: &Self::ImageAssetScope) -> String { panic!("input/specification test reached context service: image_scope_key") }
fn image_id_str(id: &Self::ImageAssetId) -> &str { panic!("input/specification test reached context service: image_id_str") }
#[cfg(not(target_arch = "wasm32"))]
    fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>> { panic!("input/specification test reached context service: use_lease") }
fn ensure_ready(context: &ToolUseContext<Self>) -> anyhow::Result<()> { panic!("input/specification test reached context service: ensure_ready") }
fn needs_approval(context: &ToolUseContext<Self>, id: &str) -> bool { panic!("input/specification test reached context service: needs_approval") }
fn can_act(security: &Self::SecurityPolicy) -> bool { panic!("input/specification test reached context service: can_act") }
fn is_rate_limited(security: &Self::SecurityPolicy) -> bool { panic!("input/specification test reached context service: is_rate_limited") }
fn is_path_allowed(security: &Self::SecurityPolicy, path: &str) -> bool { panic!("input/specification test reached context service: is_path_allowed") }
fn shell_security_decision(name: &str, input: Value, security: &Self::SecurityPolicy, command: &str, approved: bool) -> PermissionDecision { panic!("input/specification test reached context service: shell_security_decision") }
fn permission_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, input: &'a Value) -> impl Future<Output = anyhow::Result<Option<bool>>> + HostFuture + 'a { async move { panic!("input/specification test reached context service: permission_hook") } }
fn record_approval(manager: &Self::ApprovalManager, name: &str, display: &Value, response: tool_context::approval::ApprovalResponse, channel: &str) { panic!("input/specification test reached context service: record_approval") }
fn prompt_cli(manager: &Self::ApprovalManager, request: &tool_context::approval::ApprovalRequest) -> tool_context::approval::ApprovalResponse { panic!("input/specification test reached context service: prompt_cli") }
fn create_pending(manager: &Self::ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String { panic!("input/specification test reached context service: create_pending") }
fn reject_pending(manager: &Self::ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str) { panic!("input/specification test reached context service: reject_pending") }
fn await_approval<'a>(manager: &'a Self::ApprovalManager, request: &'a str, sender: &'a str, channel: &'a str, reply_target: &'a str, cancellation: Option<&'a tokio_util::sync::CancellationToken>) -> impl Future<Output = tool_context::approval::ApprovalResponse> + HostFuture + 'a { async move { panic!("input/specification test reached context service: await_approval") } }
fn pre_hook(runner: &Self::HookRunner, name: String, input: Value) -> impl Future<Output = tool_context::hooks::PreHookOutcome> + HostFuture { async move { panic!("input/specification test reached context service: pre_hook") } }
fn post_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, result: &'a ToolResult, duration: std::time::Duration) -> impl Future<Output = ()> + HostFuture + 'a { async move { panic!("input/specification test reached context service: post_hook") } }
}
