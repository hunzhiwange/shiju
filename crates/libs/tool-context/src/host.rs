//! Typed host capabilities retained by a tool context. No erased payloads or duplicate state.
use crate::{context::ToolUseContext, PermissionDecision};
use std::{path::{Path, PathBuf}, sync::Arc};
use tool_types::{Tool, ToolResult};
use serde_json::Value;

#[cfg(not(target_arch = "wasm32"))]
pub trait HostValue: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized + Send + Sync> HostValue for T {}
#[cfg(target_arch = "wasm32")]
pub trait HostValue {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> HostValue for T {}
#[cfg(not(target_arch = "wasm32"))]
pub trait HostFuture: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized + Send> HostFuture for T {}
#[cfg(target_arch = "wasm32")]
pub trait HostFuture {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> HostFuture for T {}

pub trait ContextHost: Clone + HostValue + 'static {
    type SessionSkillSources: Clone + Default + HostValue + 'static;
    type AsyncHooks: Default + HostValue + 'static;
    type EffectiveDefaults: HostValue + 'static;
    type PluginSessionSnapshot: HostValue + 'static;
    type ToolDiscoverySession: HostValue + 'static;
    type ToolCatalogSnapshot: HostValue + 'static;
    type ToolCatalog: HostValue + 'static;
    type InvokedSkillRecord: HostValue + 'static;
    type ApprovalManager: HostValue + 'static;
    type HookRunner: HostValue + 'static;
    type SecurityPolicy: HostValue + 'static;
    type Observer: ?Sized + HostValue + 'static;
    type ImageAssetId;
    type ImageAssetScope;

    fn task_local() -> &'static tokio::task::LocalKey<Arc<ToolUseContext<Self>>>;
    fn current_tenant_scope() -> vw_api_transport::TenantScope;
    fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action;
    fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool;
    fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool;
    fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>>;
    fn security_workspace(security: &Self::SecurityPolicy) -> PathBuf;
    fn image_scope_key(scope: &Self::ImageAssetScope) -> String;
    fn image_id_str(id: &Self::ImageAssetId) -> &str;
    #[cfg(not(target_arch = "wasm32"))]
    fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>>;
    fn ensure_ready(context: &ToolUseContext<Self>) -> anyhow::Result<()>;
    fn needs_approval(context: &ToolUseContext<Self>, id: &str) -> bool;
    fn can_act(security: &Self::SecurityPolicy) -> bool;
    fn is_rate_limited(security: &Self::SecurityPolicy) -> bool;
    fn is_path_allowed(security: &Self::SecurityPolicy, path: &str) -> bool;
    fn shell_security_decision(name: &str, input: Value, security: &Self::SecurityPolicy, command: &str, approved: bool) -> PermissionDecision;
    fn permission_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, input: &'a Value) -> impl Future<Output = anyhow::Result<Option<bool>>> + HostFuture + 'a;
    fn record_approval(manager: &Self::ApprovalManager, name: &str, display: &Value, response: crate::approval::ApprovalResponse, channel: &str);
    fn prompt_cli(manager: &Self::ApprovalManager, request: &crate::approval::ApprovalRequest) -> crate::approval::ApprovalResponse;
    fn create_pending(manager: &Self::ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String;
    fn reject_pending(manager: &Self::ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str);
    fn await_approval<'a>(manager: &'a Self::ApprovalManager, request: &'a str, sender: &'a str, channel: &'a str, reply_target: &'a str, cancellation: Option<&'a tokio_util::sync::CancellationToken>) -> impl Future<Output = crate::approval::ApprovalResponse> + HostFuture + 'a;
    fn pre_hook(runner: &Self::HookRunner, name: String, input: Value) -> impl Future<Output = crate::hooks::PreHookOutcome> + HostFuture;
    fn post_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, result: &'a ToolResult, duration: std::time::Duration) -> impl Future<Output = ()> + HostFuture + 'a;
}
