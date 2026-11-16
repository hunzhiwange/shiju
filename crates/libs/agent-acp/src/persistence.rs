//! ACP 完整轮次的消息、用量及终态持久化。
use std::collections::HashMap;
use session_engine::message;
use session_port::{SessionScope, ui_types::TokenUsage};
use acp_server::server::BackendError;

pub async fn persist_turn(
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
    agent_project::instance::provide(directory, None, move || {
        Box::pin(async move {
            session_store::ui_store::with_tenant_scope(tenant, async move {
                let now = session_engine::session_lifecycle::now_ms();
                let user_id = ordered_id::ascending(ordered_id::Prefix::Message, None)
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                let user_part_id = ordered_id::ascending(ordered_id::Prefix::Part, None)
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                let assistant_id = ordered_id::ascending(ordered_id::Prefix::Message, None)
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                let assistant_part_id = ordered_id::ascending(ordered_id::Prefix::Part, None)
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
