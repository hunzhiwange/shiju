//! Repository-only fixture. Any unexpected host capability fails closed.
use tool_context::{host::{ContextHost, HostFuture}, context::ToolUseContext, PermissionDecision};
use std::{path::{Path, PathBuf}, sync::Arc};
use tool_types::{Tool, ToolResult};
use serde_json::Value;
#[derive(Clone)]
pub(super) struct Host;
tokio::task_local! { static CONTEXT: Arc<ToolUseContext<Host>>; }
#[allow(unused_variables)]
impl ContextHost for Host {
    type SessionSkillSources = ();
    type AsyncHooks = ();
    type EffectiveDefaults = ();
    type PluginSessionSnapshot = ();
    type ToolDiscoverySession = ();
    type ToolCatalogSnapshot = ();
    type ToolCatalog = ();
    type InvokedSkillRecord = agent_skills::invoked::InvokedSkillRecord;
    type ApprovalManager = ();
    type HookRunner = ();
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
    fn permission_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, input: &'a Value) -> impl Future<Output = anyhow::Result<Option<bool>>> + HostFuture + 'a { async { panic!("toolset test reached host: permission_hook") } }
    fn record_approval(manager: &Self::ApprovalManager, name: &str, display: &Value, response: tool_context::approval::ApprovalResponse, channel: &str) { panic!("toolset test reached host: record_approval") }
    fn prompt_cli(manager: &Self::ApprovalManager, request: &tool_context::approval::ApprovalRequest) -> tool_context::approval::ApprovalResponse { panic!("toolset test reached host: prompt_cli") }
    fn create_pending(manager: &Self::ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String { panic!("toolset test reached host: create_pending") }
    fn reject_pending(manager: &Self::ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str) { panic!("toolset test reached host: reject_pending") }
    fn await_approval<'a>(manager: &'a Self::ApprovalManager, request: &'a str, sender: &'a str, channel: &'a str, reply_target: &'a str, cancellation: Option<&'a tokio_util::sync::CancellationToken>) -> impl Future<Output = tool_context::approval::ApprovalResponse> + HostFuture + 'a { async { panic!("toolset test reached host: await_approval") } }
    fn pre_hook(runner: &Self::HookRunner, name: String, input: Value) -> impl Future<Output = tool_context::hooks::PreHookOutcome> + HostFuture { async { panic!("toolset test reached host: pre_hook") } }
    fn post_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, result: &'a ToolResult, duration: std::time::Duration) -> impl Future<Output = ()> + HostFuture + 'a { async { panic!("toolset test reached host: post_hook") } }
}

impl tool_services::skill::SkillHost for Host {
 fn load_session_skills(workspace: &Path, config: &agent_config::runtime::Config, _: &(), full: bool) -> anyhow::Result<Vec<agent_skills::Skill>> { Ok(super::load_skills(workspace, config, full)) }
 fn load_skills_with_config(workspace: &Path, config: &agent_config::runtime::Config) -> Vec<agent_skills::Skill> { super::load_skills_full_with_config(workspace, config) }
 async fn default_agent() -> Result<String, String> { Ok("main".into()) }
 async fn permission_rules(_: &str) -> Option<permission_runtime::next::Ruleset> { None }
 async fn ask(_: question_types::AskInput) -> Result<Vec<question_types::Answer>, String> { panic!("unexpected approval") }
 #[cfg(not(target_arch = "wasm32"))]
 async fn inherit_scope<F: std::future::Future + Send>(future: F) -> F::Output where F::Output: Send { future.await }
}
