//! Real metadata/message storage; unrelated external tool capabilities fail explicitly.
use std::{path::{Path, PathBuf}, sync::Arc};
use serde_json::Value;
use tool_context::{host::ContextHost, context::ToolUseContext, PermissionDecision};
use tool_types::{Tool, ToolResult};
use permission_runtime::next as permission_types;
#[derive(Clone, Debug)]
pub struct Host;
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
    type InvokedSkillRecord = ();
    type ApprovalManager = ();
    type HookRunner = agent_hooks::HookRunner;
    type SecurityPolicy = ();
    type Observer = ();
    type ImageAssetId = String;
    type ImageAssetScope = String;
    fn task_local() -> &'static tokio::task::LocalKey<Arc<ToolUseContext<Self>>> { &CONTEXT }
    fn current_tenant_scope() -> vw_api_transport::TenantScope { session_store::ui_store::current_tenant_scope() }
    fn permission_action(policy: &Self::EffectiveDefaults, permission: &str, aliases: &[String], pattern: &str) -> permission_types::Action { panic!("unused external capability: permission_action") }
    fn catalog_authorizes(catalog: &Self::ToolCatalogSnapshot, id: &str) -> bool { panic!("unused external capability: catalog_authorizes") }
    fn discovery_authorizes(discovery: &Self::ToolDiscoverySession, id: &str) -> bool { panic!("unused external capability: discovery_authorizes") }
    fn catalog_registry(catalog: &Self::ToolCatalogSnapshot) -> Vec<Box<dyn Tool>> { panic!("unused external capability: catalog_registry") }
    fn security_workspace(security: &Self::SecurityPolicy) -> PathBuf { panic!("unused external capability: security_workspace") }
    fn image_scope_key(scope: &Self::ImageAssetScope) -> String { panic!("unused external capability: image_scope_key") }
    fn image_id_str(id: &Self::ImageAssetId) -> &str { panic!("unused external capability: image_id_str") }
    #[cfg(not(target_arch = "wasm32"))]
    fn use_lease(path: &Path) -> anyhow::Result<Option<std::fs::File>> { panic!("unused external capability: use_lease") }
    fn ensure_ready(context: &ToolUseContext<Self>) -> anyhow::Result<()> { panic!("unused external capability: ensure_ready") }
    fn needs_approval(context: &ToolUseContext<Self>, id: &str) -> bool { panic!("unused external capability: needs_approval") }
    fn can_act(security: &Self::SecurityPolicy) -> bool { panic!("unused external capability: can_act") }
    fn is_rate_limited(security: &Self::SecurityPolicy) -> bool { panic!("unused external capability: is_rate_limited") }
    fn is_path_allowed(security: &Self::SecurityPolicy, path: &str) -> bool { panic!("unused external capability: is_path_allowed") }
    fn shell_security_decision(name: &str, input: Value, security: &Self::SecurityPolicy, command: &str, approved: bool) -> PermissionDecision { panic!("unused external capability: shell_security_decision") }
    async fn permission_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, input: &'a Value) -> anyhow::Result<Option<bool>> { panic!("unused external capability: permission_hook") }
    fn record_approval(manager: &Self::ApprovalManager, name: &str, display: &Value, response: tool_context::approval::ApprovalResponse, channel: &str) { panic!("unused external capability: record_approval") }
    fn prompt_cli(manager: &Self::ApprovalManager, request: &tool_context::approval::ApprovalRequest) -> tool_context::approval::ApprovalResponse { panic!("unused external capability: prompt_cli") }
    fn create_pending(manager: &Self::ApprovalManager, name: &str, sender: &str, channel: &str, reply_target: &str, reason: Option<String>, display: Value, message_id: Option<String>, call_id: Option<String>) -> String { panic!("unused external capability: create_pending") }
    fn reject_pending(manager: &Self::ApprovalManager, request: &str, sender: &str, channel: &str, reply_target: &str) { panic!("unused external capability: reject_pending") }
    async fn await_approval<'a>(manager: &'a Self::ApprovalManager, request: &'a str, sender: &'a str, channel: &'a str, reply_target: &'a str, cancellation: Option<&'a tokio_util::sync::CancellationToken>) -> tool_context::approval::ApprovalResponse { panic!("unused external capability: await_approval") }
    async fn pre_hook(runner: &Self::HookRunner, name: String, input: Value) -> tool_context::hooks::PreHookOutcome { panic!("unused external capability: pre_hook") }
    async fn post_hook<'a>(runner: &'a Self::HookRunner, name: &'a str, result: &'a ToolResult, duration: std::time::Duration) { panic!("unused external capability: post_hook") }
}

impl crate::session_lifecycle::LifecycleHost for Host {
    fn home() -> PathBuf { agent_paths::paths().home.clone() }
    fn version() -> String { "test".into() }
    async fn close_hooks(_: &str, _: &Path) { panic!("unused hook cleanup") }
    fn close_mcp(_: &str, _: &Path) { panic!("unused MCP cleanup") }
}
pub type Context = ToolUseContext<Host>;
pub mod session {
    pub use crate::session_lifecycle::{Info, Error, CreateInput};
    pub async fn create_next(input: CreateInput) -> Result<Info, Error> { crate::session_lifecycle::create_next::<super::Host>(input).await }
    pub async fn get(id: &str) -> Result<Info, Error> { crate::session_lifecycle::get::<super::Host>(id).await }
    pub async fn update(id: &str, f: impl FnOnce(&mut Info) + Send) -> Result<Info, Error> { crate::session_lifecycle::update::<super::Host>(id, f).await }
}
impl crate::child_agent::run::ChildHost for Host {
    async fn get_session(id: &str) -> anyhow::Result<session_info::Info> { Ok(session::get(id).await?) }
    async fn reserve_child_agent(parent: String, directory: String, permission: Option<permission_types::Ruleset>) -> anyhow::Result<session_info::Info> {
        Ok(crate::session_lifecycle::reserve_child_agent::<Self>(session::CreateInput { parent_id: Some(parent), directory, permission, title: None }).await?)
    }
    fn execution_hook_runner(_: &agent_config::runtime::Config) -> anyhow::Result<Option<Arc<agent_hooks::HookRunner>>> { Ok(None) }
    async fn permission_rules(_: &str) -> Option<permission_types::Ruleset> { None }
    async fn collect_gateway_chat_stream(_: String, _: vw_api_chat::chat::GatewayChatStreamRequest) -> Result<session_port::chat_stream::GatewayChatStreamCollected, gateway_error::ApiError> { panic!("tests inject the external model boundary") }
    async fn persist_child_agent_turn(id: &str, prompt: &str, output: &str, agent: &str, error: Option<&str>) -> Result<(String, String), gateway_error::ApiError> {
        persist(id, prompt, output, agent, error).await.map_err(|e| gateway_error::ApiError::internal(e.to_string()))
    }
    fn commit_collected_child_agent_turn(_: &str, _: &str, _: &str, _: (String, String)) -> anyhow::Result<()> {
        // UI publication is a separate host boundary, covered by the retained gateway tests.
        Ok(())
    }
}
async fn persist(id: &str, prompt: &str, output: &str, agent: &str, error: Option<&str>) -> anyhow::Result<(String,String)> {
    use crate::message as m;
    let user_id = ordered_id::ascending(ordered_id::Prefix::Message, None)?;
    let assistant_id = ordered_id::ascending(ordered_id::Prefix::Message, None)?;
    let now = crate::session_lifecycle::now_ms();
    let user = m::Info::User(Box::new(m::UserInfo { id: user_id.clone(), session_id: id.into(), time: m::UserTime { created: now }, summary: None, agent: agent.into(), model: m::ModelRef { provider_id: "fixture".into(), model_id: "fixture".into() }, system: None, tools: None, variant: None }));
    let assistant = m::Info::Assistant(Box::new(m::AssistantInfo { id: assistant_id.clone(), session_id: id.into(), time: m::AssistantTime { created: now, completed: Some(now), processing_elapsed_ms: None, llm_elapsed_ms: None }, error: error.map(|e| m::AssistantError::Unknown { message: e.into() }), parent_id: user_id.clone(), model_id: "fixture".into(), provider_id: "fixture".into(), mode: "chat".into(), agent: agent.into(), path: m::PathInfo { cwd: agent_project::instance::directory(), root: agent_project::instance::worktree() }, summary: None, cost: 0.0, tokens: m::TokenInfo { total: None, input: 0, output: 0, reasoning: 0, cache: m::TokenCacheInfo { read: 0, write: 0 } }, variant: None, finish: error.map(|_| "error".into()) }));
    let _history = m::lock_history(id).await;
    for (info, message_id, text) in [(&user, &user_id, prompt), (&assistant, &assistant_id, output)] {
        m::update_message(info).await?;
        m::update_part(&m::Part::Text(m::TextPart { base: m::PartBase { id: ordered_id::ascending(ordered_id::Prefix::Part, None)?, session_id: id.into(), message_id: message_id.clone() }, text: text.into(), synthetic: None, ignored: None, time: None, metadata: None })).await?;
    }
    Ok((assistant_id, user_id))
}
