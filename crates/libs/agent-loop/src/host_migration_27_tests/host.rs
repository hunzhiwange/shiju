//! Limited context for scripted loop tests; real executor and history rules are reused.
use std::{path::{Path, PathBuf}, sync::Arc};
use serde_json::Value;
use tool_context::{context::ToolUseContext, host::{ContextHost, HostFuture}, PermissionDecision};
use tool_types::{Tool, ToolResult};

#[derive(Clone)]
pub(super) struct DomainHost;
tokio::task_local! { static CONTEXT: Arc<ToolUseContext<DomainHost>>; }

#[allow(unused_variables)]
impl ContextHost for DomainHost {

    type SessionSkillSources = ();
    type AsyncHooks = plugin_hooks::hook_async::AsyncHooks;
    type EffectiveDefaults = ();
    type PluginSessionSnapshot = ();
    type ToolDiscoverySession = Vec<tool_types::ToolSpec>;
    type ToolCatalogSnapshot = tool_assembly::catalog::ToolCatalogSnapshot;
    type ToolCatalog = tool_assembly::catalog::ToolCatalog;
    type InvokedSkillRecord = agent_skills::invoked::InvokedSkillRecord;
    type ApprovalManager = agent_approval::ApprovalManager;
    type HookRunner = agent_hooks::HookRunner;
    type SecurityPolicy = agent_security::SecurityPolicy;
    type Observer = ();
    type ImageAssetId = agent_multimodal::assets::ImageAssetId;
    type ImageAssetScope = agent_multimodal::assets::ImageAssetScope;

    fn task_local() -> &'static tokio::task::LocalKey<Arc<ToolUseContext<Self>>> { &CONTEXT }
    fn current_tenant_scope() -> vw_api_transport::TenantScope { Default::default() }
    fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action { panic!("local contract test reached host service") }
    fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool { panic!("local contract test reached host service") }
    fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool { discovery.iter().any(|spec| spec.id == id) }
    fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>> { panic!("local contract test reached host service") }
    fn security_workspace(security: &Self::SecurityPolicy) -> PathBuf { panic!("local contract test reached host service") }
    fn image_scope_key(scope: &Self::ImageAssetScope) -> String { panic!("local contract test reached host service") }
    fn image_id_str(id: &Self::ImageAssetId) -> &str { panic!("local contract test reached host service") }
    #[cfg(not(target_arch = "wasm32"))]
    fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>> { Ok(None) }
    fn ensure_ready(context: &ToolUseContext<Self>) -> anyhow::Result<()> { Ok(()) }
    fn needs_approval(context: &ToolUseContext<Self>, id: &str) -> bool { assert!(matches!(id, "echo_tool" | "testcov_0091_context_capture"), "unexpected tool in scripted executor"); false }
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
use agent_config::runtime::Config;
use agent_skills::{types::Skill, invoked::InvokedSkillRecord};
use std::collections::BTreeSet;

#[allow(unused_variables)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl crate::core::skills::CoreSkillHost for DomainHost {
type Context = Self;
fn current_tenant_scope() -> vw_api_transport::id::TenantScope { panic!("domain assertion reached host service") }
fn load_skills_with_config(directory: &std::path::Path, config: &Config) -> Vec<Skill> { panic!("domain assertion reached host service") }
async fn get_model(provider: &str, model: &str) -> std::result::Result<model_types::Model, agent_provider::provider::CatalogError> { panic!("domain assertion reached host service") }
fn explicitly_referenced_skill_names(skills: &[Skill], text: &str) -> BTreeSet<String> { panic!("domain assertion reached host service") }
fn user_explicitly_requests_skill_use(text: &str) -> bool { panic!("domain assertion reached host service") }
fn user_invocable_skill_names(skills: &[Skill]) -> BTreeSet<String> { panic!("domain assertion reached host service") }
#[cfg(not(target_arch = "wasm32"))]
fn accessible_skills(skills: Vec<Skill>, context: Option<&ToolUseContext<Self::Context>>) -> Vec<Skill> { panic!("domain assertion reached host service") }
fn description_for_skills(directory: &std::path::Path, skills: Vec<Skill>) -> String { panic!("domain assertion reached host service") }
async fn restore(session: &str, agent: &str, history: &[Value]) -> std::result::Result<Vec<InvokedSkillRecord>, String> { panic!("domain assertion reached host service") }
async fn save(session: &str, agent: &str, history: &[Value], records: &[InvokedSkillRecord]) -> std::result::Result<(), String> { panic!("domain assertion reached host service") }
}
