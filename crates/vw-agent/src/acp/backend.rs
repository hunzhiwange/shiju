//! `vw-acp` 后端 trait 到 VibeWindow 原生会话运行时的直接适配。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use acp_server::server::{
    BackendError, BackendPrompt, PromptContent,
};
use vw_api_transport::id::TenantScope;

use crate::agent::loop_::approval::NonCliApprovalContext;
use crate::app::agent::approval::ApprovalManager;
use crate::session::message;
use crate::session::processor::{Request, StreamEvent};
use crate::session::runtime::{
    CreateSessionInput, PromptTurn,
    SessionScope, SessionToolset, create_session, load_session, start_prompt,
    start_prompt_with_runner,
};
use crate::session::ui_types::{ChatMessage, TokenUsage};


#[path = "models.rs"]
mod models;

#[derive(Default)]
pub struct NativeAcpHost;
pub struct NativeManagedMounts(crate::agent_apps::mcp::acp_mount::ResolvedCatalogMounts);
pub type VibeWindowBackend = agent_acp::protocol::VibeWindowBackend<NativeAcpHost>;
type BoundSession = agent_acp::protocol::BoundSession<NativeAcpHost>;
use agent_acp::backend::{prompt_query, processor_history};
use agent_acp::protocol::{backend_history, lifecycle_error};

fn runtime_request(
    session_id: &str,
    scope: &SessionScope,
    query: String,
    history: Vec<ChatMessage>,
    approval: Arc<ApprovalManager>,
    non_cli_approval_context: NonCliApprovalContext,
    session_tools: SessionToolset,
    reliability: crate::app::agent::config::ReliabilityConfig,
) -> Request {
    Request {
        stream: 0,
        session: session_id.to_string(),
        query,
        root: Some(scope.directory().to_string_lossy().into_owned()),
        model: None,
        reliability,
        // `acp_agent` 用于选择外部 ACP Agent；入站请求使用原生模型与本地工具，
        // 传输来源由 channel_name 记录。
        options: json!({}),
        approval: Some(approval),
        channel_name: Some("acp".to_string()),
        non_cli_approval_context: Some(non_cli_approval_context),
        assistant_message_id: None,
        history,
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools,
        session_skills: Default::default(),
    }
}

#[cfg(test)]
pub(crate) use agent_acp::backend::map_runtime_event;

async fn persist_turn(
    scope: SessionScope,
    session_id: String,
    query: String,
    assistant_text: String,
    usage: TokenUsage,
    error: Option<String>,
    cancelled: bool,
) -> Result<(), BackendError> {
    let tenant = scope.tenant().clone();
    let directory = scope.directory().to_path_buf();
    let directory_text = directory.to_string_lossy().into_owned();
    crate::project::instance::provide(directory, None, move || {
        Box::pin(async move {
            crate::session::ui_store::with_tenant_scope(tenant, async move {
                let now = crate::session::session::now_ms();
                let user_id = crate::id::ascending(crate::id::Prefix::Message, None)
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                let user_part_id = crate::id::ascending(crate::id::Prefix::Part, None)
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                let assistant_id = crate::id::ascending(crate::id::Prefix::Message, None)
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                let assistant_part_id = crate::id::ascending(crate::id::Prefix::Part, None)
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                let user_info = message::Info::User(Box::new(message::UserInfo {
                    id: user_id.clone(),
                    session_id: session_id.clone(),
                    time: message::UserTime { created: now },
                    summary: None,
                    agent: "acp".to_string(),
                    model: message::ModelRef {
                        provider_id: String::new(),
                        model_id: String::new(),
                    },
                    system: None,
                    tools: Some(HashMap::new()),
                    variant: None,
                }));
                let user_part = message::Part::Text(message::TextPart {
                    base: message::PartBase {
                        id: user_part_id,
                        session_id: session_id.clone(),
                        message_id: user_id.clone(),
                    },
                    text: query,
                    synthetic: None,
                    ignored: None,
                    time: Some(message::PartTime { start: now, end: Some(now) }),
                    metadata: None,
                });
                let assistant_error =
                    error.clone().map(|message| message::AssistantError::Unknown { message });
                let assistant_info = message::Info::Assistant(Box::new(message::AssistantInfo {
                    id: assistant_id.clone(),
                    session_id: session_id.clone(),
                    time: message::AssistantTime {
                        created: now,
                        completed: Some(now),
                        processing_elapsed_ms: None,
                        llm_elapsed_ms: None,
                    },
                    error: assistant_error,
                    parent_id: user_id,
                    model_id: String::new(),
                    provider_id: String::new(),
                    mode: "chat".to_string(),
                    agent: "acp".to_string(),
                    path: message::PathInfo { cwd: directory_text.clone(), root: directory_text },
                    summary: None,
                    cost: 0.0,
                    tokens: message::TokenInfo {
                        total: Some(
                            usage
                                .input_tokens
                                .saturating_add(usage.output_tokens)
                                .saturating_add(usage.reasoning_tokens),
                        ),
                        input: usage.input_tokens,
                        output: usage.output_tokens,
                        reasoning: usage.reasoning_tokens,
                        cache: message::TokenCacheInfo { read: usage.cached_tokens, write: 0 },
                    },
                    variant: None,
                    finish: Some(
                        if cancelled {
                            "cancelled"
                        } else if error.is_some() {
                            "error"
                        } else {
                            "stop"
                        }
                        .to_string(),
                    ),
                }));
                let assistant_part = message::Part::Text(message::TextPart {
                    base: message::PartBase {
                        id: assistant_part_id,
                        session_id: session_id.clone(),
                        message_id: assistant_id,
                    },
                    text: assistant_text,
                    synthetic: None,
                    ignored: None,
                    time: Some(message::PartTime { start: now, end: Some(now) }),
                    metadata: None,
                });
                let _history = message::lock_history(&session_id).await;
                message::update_message(&user_info)
                    .await
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                message::update_part(&user_part)
                    .await
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                message::update_message(&assistant_info)
                    .await
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                message::update_part(&assistant_part)
                    .await
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                Ok(())
            })
            .await
        })
    })
    .await
    .map_err(|error| BackendError::Internal(error.to_string()))?
}

async fn start_prompt_core(
    bound: BoundSession,
    operation: tokio::sync::OwnedMutexGuard<()>,
    session_id: &str,
    query: String,
    starter: impl FnOnce(Request) -> PromptTurn,
) -> Result<BackendPrompt, BackendError> {
    let BoundSession { scope, tools, skills, model, .. } = bound;
    let model = model.lock().await.clone();
    let snapshot = load_session(scope.clone(), session_id).await.map_err(lifecycle_error)?;
    let tools = super::mcp::prune_revoked_mounts(tools)?;
    let history = backend_history(snapshot.history);
    let config = crate::config::get_project_for_tenant(scope.tenant(), scope.directory())
        .await
        .map_err(|error| BackendError::Internal(error.to_string()))?;
    let approval = Arc::new(ApprovalManager::from_config(&config.autonomy));
    let (approval_tx, approval_rx) = tokio::sync::mpsc::unbounded_channel();
    let mut request = runtime_request(
        session_id,
        &scope,
        query.clone(),
        processor_history(&history),
        Arc::clone(&approval),
        NonCliApprovalContext {
            sender: session_id.to_string(),
            reply_target: session_id.to_string(),
            prompt_tx: approval_tx,
        },
        tools,
        config.reliability,
    );
    request.model = Some(model);
    request.session_skills = skills;
    let runtime = crate::session::ui_store::with_tenant_scope(scope.tenant().clone(), async move {
        starter(request)
    })
    .await;
    Ok(agent_acp::prompt::bridge_prompt(
        NativeTurnPersistence,
        runtime.cancel.cancellation_token(),
        runtime,
        operation,
        tokio::runtime::Handle::current(),
        scope,
        session_id.to_string(),
        query,
        approval,
        approval_rx,
    ))
}

pub(crate) async fn start_prompt_with_runtime_runner<R>(
    backend: &VibeWindowBackend,
    session_id: &str,
    prompt: Vec<PromptContent>,
    runner: R,
) -> Result<BackendPrompt, BackendError>
where
    R: FnOnce(Request, Box<dyn FnMut(StreamEvent) -> bool + Send>) + Send + 'static,
{
    let (bound, operation) = backend.lock_prompt(session_id)?;
    start_prompt_core(bound, operation, session_id, prompt_query(prompt), |request| {
        start_prompt_with_runner(request, runner)
    })
    .await
}


#[async_trait]
impl agent_acp::protocol::Host for NativeAcpHost {
    type Tools = SessionToolset;
    type Skills = super::mounts::SessionSkillSources;
    type Managed = NativeManagedMounts;
    async fn create_session(&self, input: CreateSessionInput) -> Result<crate::session::runtime::SessionSnapshot, crate::session::runtime::LifecycleError> { create_session(input).await }
    async fn load_session(&self, scope: SessionScope, id: &str) -> Result<crate::session::runtime::SessionSnapshot, crate::session::runtime::LifecycleError> { load_session(scope, id).await }
    fn empty_mounts(&self) -> Self::Managed { NativeManagedMounts(Default::default()) }
    fn resolve_mounts(&self, scope: &SessionScope, id: &str, project: &str, catalog: &[acp_types::McpCatalogMount], restore: bool) -> Result<Self::Managed, BackendError> {
        crate::agent_apps::mcp::acp_mount::resolve_mounts(scope, id, project, catalog, restore).map(NativeManagedMounts)
    }
    async fn prepare_revision(&self, scope: &SessionScope, id: &str, request: acp_server::server::BackendMountRequest, saved: &[acp_types::SessionSkillMount], revision: acp_types::SessionMountRevision, managed: Self::Managed) -> Result<agent_acp::protocol::PreparedMounts<Self>, BackendError> {
        let prepared = super::mounts::prepare_revision(scope, id, request, saved, revision, managed.0).await?;
        Ok(agent_acp::protocol::PreparedMounts { skills: prepared.skills, tools: prepared.tools, response: prepared.response, mcp_catalog: prepared.mcp_catalog })
    }
    async fn save_mounts(&self, scope: &SessionScope, id: &str, prepared: &agent_acp::protocol::PreparedMounts<Self>) -> Result<(), BackendError> {
        super::mount_store::save(scope, id, &super::mounts::PreparedMounts { skills: prepared.skills.clone(), tools: prepared.tools.clone(), response: prepared.response.clone(), mcp_catalog: prepared.mcp_catalog.clone() }).await
    }
    fn revoke(&self, skills: &Self::Skills) { skills.revoke(); }
    fn fingerprint<'a>(&self, skills: &'a Self::Skills) -> Option<&'a str> { skills.fingerprint() }
    fn shutdown(&self, tools: &Self::Tools) { if let Some(connections) = tools.mcp_connections() { connections.shutdown(); } }
    fn tool_count(&self, tools: &Self::Tools) -> usize { tools.len() }
    async fn session_models(&self, scope: SessionScope, selected: String) -> Result<agent_client_protocol::SessionModelState, BackendError> { models::session_models(scope, selected).await }
    async fn start_bound_prompt(&self, bound: BoundSession, operation: tokio::sync::OwnedMutexGuard<()>, id: &str, query: String) -> Result<BackendPrompt, BackendError> { start_prompt_core(bound, operation, id, query, start_prompt).await }
}

struct NativeTurnPersistence;
#[async_trait]
impl agent_acp::prompt::TurnPersistence for NativeTurnPersistence {
    async fn persist_turn(&self, scope: SessionScope, session_id: String, query: String, assistant_text: String, usage: TokenUsage, error: Option<String>, cancelled: bool) -> Result<(), BackendError> {
        persist_turn(scope, session_id, query, assistant_text, usage, error, cancelled).await
    }
}
