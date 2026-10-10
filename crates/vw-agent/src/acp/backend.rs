//! `vw-acp` 后端 trait 到 VibeWindow 原生会话运行时的直接适配。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use agent_client_protocol::McpServer;
use async_trait::async_trait;
use serde_json::{Value, json};
use vw_acp::server::{
    BackendCancelHandle, BackendError, BackendEvent, BackendHistoryEntry, BackendPrompt,
    BackendRole, BackendSession, BackendTermination, BackendToolStatus, PromptContent,
    SessionBackend,
};
use vw_api_types::id::TenantScope;

use crate::agent::loop_::approval::NonCliApprovalContext;
use crate::app::agent::approval::ApprovalManager;
use crate::session::message;
use crate::session::processor::{Request, StreamEvent, ToolCallStatus, ToolEvent, ToolEventKind};
use crate::session::runtime::{
    CreateSessionInput, PromptCancelHandle, PromptEvent, PromptTermination, PromptTurn,
    SessionScope, SessionToolset, create_session, load_session, start_prompt,
    start_prompt_with_runner,
};
use crate::session::ui_types::{ChatMessage, ChatRole, TokenUsage};

use super::approval::ApprovalBridge;
use super::mcp::prepare_session_tools;

#[derive(Clone)]
struct BoundSession {
    scope: SessionScope,
    tools: SessionToolset,
}

/// VibeWindow 原生会话和提示运行时的 ACP 后端。
pub struct VibeWindowBackend {
    tenant: TenantScope,
    sessions: Mutex<HashMap<String, BoundSession>>,
}

impl VibeWindowBackend {
    /// 创建绑定固定租户作用域的 ACP 后端。
    pub fn new(tenant: TenantScope) -> Self {
        Self { tenant, sessions: Mutex::new(HashMap::new()) }
    }

    fn scope(&self, cwd: PathBuf) -> Result<SessionScope, BackendError> {
        SessionScope::new(self.tenant.clone(), cwd).map_err(lifecycle_error)
    }

    fn remember_scope(&self, session_id: &str, scope: SessionScope, tools: SessionToolset) {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(session_id.to_string(), BoundSession { scope, tools });
    }

    fn bound_session(&self, session_id: &str) -> Result<BoundSession, BackendError> {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .cloned()
            .ok_or_else(|| BackendError::NotFound(session_id.to_string()))
    }

    #[cfg(test)]
    pub(crate) fn session_tool_count(&self, session_id: &str) -> Option<usize> {
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            .map(|bound| bound.tools.len())
    }
}

impl Default for VibeWindowBackend {
    fn default() -> Self {
        Self::new(TenantScope::default())
    }
}

fn lifecycle_error(error: crate::session::runtime::LifecycleError) -> BackendError {
    use crate::session::runtime::LifecycleError;
    let message = error.to_string();
    match error {
        LifecycleError::SessionNotFound { session_id } => BackendError::NotFound(session_id),
        LifecycleError::SessionOwnershipMismatch { session_id, .. } => {
            BackendError::OwnershipMismatch(session_id)
        }
        LifecycleError::DirectoryMustBeAbsolute { .. }
        | LifecycleError::DirectoryUnavailable { .. }
        | LifecycleError::DirectoryNotDirectory { .. } => BackendError::InvalidRequest(message),
        LifecycleError::Project(_) | LifecycleError::Session(_) => BackendError::Internal(message),
    }
}

fn message_time(info: &message::Info) -> u64 {
    match info {
        message::Info::User(info) => info.time.created,
        message::Info::Assistant(info) => info.time.created,
    }
}

fn message_text(parts: &[message::Part]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            message::Part::Text(text) if text.synthetic != Some(true) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn backend_history(mut history: Vec<message::WithParts>) -> Vec<BackendHistoryEntry> {
    history.sort_by_key(|item| message_time(&item.info));
    history
        .into_iter()
        .filter_map(|item| {
            let text = message_text(&item.parts);
            if text.trim().is_empty() {
                return None;
            }
            let role = match item.info {
                message::Info::User(_) => BackendRole::User,
                message::Info::Assistant(_) => BackendRole::Assistant,
            };
            Some(BackendHistoryEntry { role, text })
        })
        .collect()
}

fn processor_history(history: &[BackendHistoryEntry]) -> Vec<ChatMessage> {
    history
        .iter()
        .map(|entry| ChatMessage {
            role: match entry.role {
                BackendRole::User => ChatRole::User,
                BackendRole::Assistant => ChatRole::Assistant,
            },
            content: entry.text.clone(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect()
}

fn backend_session(
    snapshot: crate::session::runtime::SessionSnapshot,
    scope: &SessionScope,
) -> BackendSession {
    BackendSession {
        id: snapshot.info.id,
        cwd: scope.directory().to_path_buf(),
        history: backend_history(snapshot.history),
    }
}

fn prompt_query(prompt: Vec<PromptContent>) -> String {
    prompt
        .into_iter()
        .map(|content| match content {
            PromptContent::Text(text) => text,
            PromptContent::ResourceLink { name, uri, mime_type } => json!({
                "type": "resource_link",
                "name": name,
                "uri": uri,
                "mime_type": mime_type,
            })
            .to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

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
        options: json!({
            "acp_agent": "vibewindow",
        }),
        approval: Some(approval),
        channel_name: Some("acp".to_string()),
        non_cli_approval_context: Some(non_cli_approval_context),
        assistant_message_id: None,
        history,
        persist_app_session_artifacts: false,
        app_session_scope: None,
        abort_token: None,
        session_tools,
    }
}

fn backend_tool_status(status: &ToolCallStatus) -> BackendToolStatus {
    match status {
        ToolCallStatus::Running | ToolCallStatus::Waiting => BackendToolStatus::Running,
        ToolCallStatus::Completed => BackendToolStatus::Completed,
        ToolCallStatus::Failed | ToolCallStatus::Denied => BackendToolStatus::Failed,
        ToolCallStatus::Cancelled => BackendToolStatus::Cancelled,
        ToolCallStatus::Other(_) => BackendToolStatus::Failed,
    }
}

fn payload_text(payload: &Value) -> Option<String> {
    payload
        .get("output")
        .or_else(|| payload.get("error"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn map_tool_event(event: ToolEvent) -> BackendEvent {
    let status = backend_tool_status(&event.status);
    match event.kind {
        ToolEventKind::Start => BackendEvent::ToolStart {
            call_id: event.call_id,
            title: event
                .payload
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(&event.name)
                .to_string(),
            raw_input: event.payload.get("input").cloned().unwrap_or(Value::Null),
        },
        ToolEventKind::Update => BackendEvent::ToolUpdate {
            call_id: event.call_id,
            status,
            content: payload_text(&event.payload),
            raw_output: event.payload.get("output").cloned(),
        },
        ToolEventKind::Complete => BackendEvent::ToolComplete {
            call_id: event.call_id,
            status,
            content: payload_text(&event.payload),
            raw_output: event.payload.get("output").cloned(),
        },
    }
}

pub(crate) fn map_runtime_event(event: PromptEvent) -> Option<BackendEvent> {
    match event {
        PromptEvent::LlmRawArtifact(_) => None,
        PromptEvent::ContextLoaded { paths } => {
            Some(BackendEvent::Status(format!("loaded context: {}", paths.join(", "))))
        }
        PromptEvent::Text(text) => Some(BackendEvent::Text(text)),
        PromptEvent::Tool(event) => Some(map_tool_event(event)),
        PromptEvent::Compaction { .. } => None,
        PromptEvent::StepStart { step_index, .. } => {
            Some(BackendEvent::Status(format!("model step {step_index} started")))
        }
        PromptEvent::StepFinish { step_index, .. } => {
            Some(BackendEvent::Status(format!("model step {step_index} finished")))
        }
        PromptEvent::PostToolRound { step_index } => {
            Some(BackendEvent::Status(format!("tool round {step_index} finished")))
        }
        PromptEvent::Terminated(PromptTermination::Completed(_)) => {
            Some(BackendEvent::Terminal(BackendTermination::EndTurn))
        }
        PromptEvent::Terminated(PromptTermination::Cancelled) => {
            Some(BackendEvent::Terminal(BackendTermination::Cancelled))
        }
        PromptEvent::Terminated(PromptTermination::Failed(error)) => {
            Some(BackendEvent::Error(error))
        }
    }
}

struct RuntimeCancel(PromptCancelHandle);

impl BackendCancelHandle for RuntimeCancel {
    fn cancel(&self) -> bool {
        self.0.cancel()
    }
}

async fn persist_turn(
    scope: SessionScope,
    session_id: String,
    query: String,
    assistant_text: String,
    usage: TokenUsage,
    error: Option<String>,
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
                    finish: Some(if error.is_some() { "error" } else { "stop" }.to_string()),
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

fn bridge_prompt(
    runtime: PromptTurn,
    runtime_handle: tokio::runtime::Handle,
    scope: SessionScope,
    session_id: String,
    query: String,
    approval: Arc<ApprovalManager>,
    approval_prompts: tokio::sync::mpsc::UnboundedReceiver<
        crate::agent::loop_::approval::NonCliApprovalPrompt,
    >,
) -> BackendPrompt {
    let (events_tx, events_rx) = tokio::sync::mpsc::unbounded_channel();
    let (permissions_tx, permissions_rx) = tokio::sync::mpsc::unbounded_channel();
    let cancel = runtime.cancel.clone();
    runtime_handle.spawn(
        ApprovalBridge::new(
            approval,
            approval_prompts,
            permissions_tx,
            runtime.cancel.token.clone(),
        )
        .run(),
    );
    std::thread::spawn(move || {
        let mut assistant_text = String::new();
        while let Ok(event) = runtime.events.recv() {
            if let PromptEvent::Text(text) = &event {
                assistant_text.push_str(text);
            }
            match &event {
                PromptEvent::Terminated(PromptTermination::Completed(usage)) => {
                    if let Err(error) = runtime_handle.block_on(persist_turn(
                        scope.clone(),
                        session_id.clone(),
                        query.clone(),
                        assistant_text.clone(),
                        usage.clone(),
                        None,
                    )) {
                        let _ = events_tx.send(BackendEvent::Error(error.to_string()));
                        break;
                    }
                }
                PromptEvent::Terminated(PromptTermination::Failed(error)) => {
                    let stored_text = if assistant_text.trim().is_empty() {
                        error.clone()
                    } else {
                        assistant_text.clone()
                    };
                    if let Err(persist_error) = runtime_handle.block_on(persist_turn(
                        scope.clone(),
                        session_id.clone(),
                        query.clone(),
                        stored_text,
                        TokenUsage::default(),
                        Some(error.clone()),
                    )) {
                        let _ = events_tx.send(BackendEvent::Error(persist_error.to_string()));
                        break;
                    }
                }
                PromptEvent::Terminated(PromptTermination::Cancelled) => {}
                _ => {}
            }
            if let Some(event) = map_runtime_event(event) {
                let terminal = matches!(event, BackendEvent::Terminal(_) | BackendEvent::Error(_));
                if events_tx.send(event).is_err() || terminal {
                    break;
                }
            }
        }
    });
    BackendPrompt::new(events_rx, std::sync::Arc::new(RuntimeCancel(cancel)))
        .with_permissions(permissions_rx)
}

async fn start_prompt_core(
    backend: &VibeWindowBackend,
    session_id: &str,
    prompt: Vec<PromptContent>,
    starter: impl FnOnce(Request) -> PromptTurn,
) -> Result<BackendPrompt, BackendError> {
    let BoundSession { scope, tools } = backend.bound_session(session_id)?;
    let snapshot = load_session(scope.clone(), session_id).await.map_err(lifecycle_error)?;
    let history = backend_history(snapshot.history);
    let query = prompt_query(prompt);
    let config = crate::config::get_project_for_tenant(&backend.tenant, scope.directory())
        .await
        .map_err(|error| BackendError::Internal(error.to_string()))?;
    let approval = Arc::new(ApprovalManager::from_config(&config.autonomy));
    let (approval_tx, approval_rx) = tokio::sync::mpsc::unbounded_channel();
    let request = runtime_request(
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
    let runtime = starter(request);
    Ok(bridge_prompt(
        runtime,
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
    start_prompt_core(backend, session_id, prompt, |request| {
        start_prompt_with_runner(request, runner)
    })
    .await
}

#[async_trait]
impl SessionBackend for VibeWindowBackend {
    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError> {
        self.create_session_with_mcp(cwd, Vec::new()).await
    }

    async fn create_session_with_mcp(
        &self,
        cwd: PathBuf,
        mcp_servers: Vec<McpServer>,
    ) -> Result<BackendSession, BackendError> {
        let tools = prepare_session_tools(&mcp_servers).await?;
        let scope = self.scope(cwd)?;
        let snapshot = create_session(CreateSessionInput {
            scope: scope.clone(),
            parent_id: None,
            title: None,
            permission: None,
        })
        .await
        .map_err(lifecycle_error)?;
        let session = backend_session(snapshot, &scope);
        self.remember_scope(&session.id, scope, tools);
        Ok(session)
    }

    async fn load_session(
        &self,
        session_id: &str,
        cwd: PathBuf,
    ) -> Result<BackendSession, BackendError> {
        self.load_session_with_mcp(session_id, cwd, Vec::new()).await
    }

    async fn load_session_with_mcp(
        &self,
        session_id: &str,
        cwd: PathBuf,
        mcp_servers: Vec<McpServer>,
    ) -> Result<BackendSession, BackendError> {
        let tools = prepare_session_tools(&mcp_servers).await?;
        let scope = self.scope(cwd)?;
        let snapshot = load_session(scope.clone(), session_id).await.map_err(lifecycle_error)?;
        let session = backend_session(snapshot, &scope);
        self.remember_scope(&session.id, scope, tools);
        Ok(session)
    }

    async fn start_prompt(
        &self,
        session_id: &str,
        prompt: Vec<PromptContent>,
    ) -> Result<BackendPrompt, BackendError> {
        start_prompt_core(self, session_id, prompt, start_prompt).await
    }
}
