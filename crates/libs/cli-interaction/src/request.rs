//! 运行层共享的 Gateway 请求构造与权限选项收紧。

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::json;
use cli_gateway_transport::gateway::normalize_optional_str_ref;
use cli_interaction_model::{PermissionMode, PromptSubmission, ReasoningEffort};
use cli_interaction_model::paste::{PromptAttachment, PromptAttachmentKind};
use cli_interaction_state::TuiState;
use {
    vw_gateway_client::GatewayChatStreamRequest,
    vw_api_transport::id::SessionId,
};
use session_ui_types::{ChatMessage, ChatRole};

/// 根据当前状态和提交快照构造 Gateway 请求，并在此收紧工具与权限选项。
pub(crate) fn gateway_stream_request_from_state(
    state: &TuiState,
    submission: &PromptSubmission,
) -> GatewayChatStreamRequest {
    let snapshot = state.to_chat_session();
    let acp_agent = submission.execution.acp_backend.clone();
    let full_access_enabled = submission.execution.full_access_enabled;
    let acp_permission_mode =
        if full_access_enabled || submission.execution.permission_mode == PermissionMode::Bypass {
            "approve-all"
        } else {
            "approve-reads"
        };
    let subagent_mode_enabled = submission.execution.subagent_mode_enabled;
    let mut allowed_tools = (!full_access_enabled
        && submission.execution.permission_mode == PermissionMode::Plan)
        .then(|| vec!["todowrite".to_string()]);
    if subagent_mode_enabled {
        // 子智能体模式只精确追加 AgentTool，不借此解锁其他受限工具。
        let tools = allowed_tools.get_or_insert_with(Vec::new);
        tools.retain(|tool| !tool.eq_ignore_ascii_case("AgentTool"));
        tools.push("AgentTool".to_string());
        tools.sort_unstable();
    }
    let mut options = acp_agent
        .as_ref()
        .map(|acp_agent| {
            let cwd = normalize_optional_str_ref(submission.root.as_deref())
                .map(ToOwned::to_owned)
                .or_else(|| {
                    state.project.workspace_root.as_ref().map(|path| path.display().to_string())
                })
                .unwrap_or_default();
            json!({
                "acp_test": true,
                "acp_agent": acp_agent,
                "cwd": cwd,
                "acp_force_new_session": submission.execution.acp_force_new_session,
                "acp_permission_mode": acp_permission_mode,
                "full_access": full_access_enabled,
            })
        })
        .or_else(|| {
            Some(json!({
                "full_access": full_access_enabled,
                "acp_permission_mode": acp_permission_mode,
            }))
        });
    if subagent_mode_enabled {
        let options = options.get_or_insert_with(|| json!({}));
        options["allowed_tools"] = json!(allowed_tools);
    }
    let system = subagent_mode_enabled.then(|| vec![SUBAGENT_MODE_SYSTEM_PROMPT.to_string()]);
    let acp_allowed_tools = acp_agent.is_some().then(|| allowed_tools.clone()).flatten();
    let mut messages: Vec<_> =
        snapshot.messages.iter().map(gateway_message_from_chat_message).collect();
    if !submission.attachments.is_empty()
        && let Some(message) = messages
            .iter_mut()
            .rev()
            .find(|message| message.get("role").and_then(serde_json::Value::as_str) == Some("user"))
    {
        message["content"] = gateway_structured_content(submission);
    }

    GatewayChatStreamRequest {
        acp_model_source:
            vw_api_acp_model::acp_model_source::AcpModelSource::SessionModel,
        session_id: normalize_optional_str_ref(submission.session_id.as_deref())
            .map(SessionId::from),
        messages,
        system,
        model: submission.model.clone(),
        reasoning_effort: match submission.execution.effort {
            ReasoningEffort::Auto => None,
            ReasoningEffort::None => {
                Some(vw_api_chat_identity::ReasoningEffortDto::None)
            }
            ReasoningEffort::Max => Some(vw_api_chat_identity::ReasoningEffortDto::Max),
            ReasoningEffort::Minimal => {
                Some(vw_api_chat_identity::ReasoningEffortDto::Minimal)
            }
            ReasoningEffort::Low => Some(vw_api_chat_identity::ReasoningEffortDto::Low),
            ReasoningEffort::Medium => {
                Some(vw_api_chat_identity::ReasoningEffortDto::Medium)
            }
            ReasoningEffort::High => {
                Some(vw_api_chat_identity::ReasoningEffortDto::High)
            }
            ReasoningEffort::XHigh => {
                Some(vw_api_chat_identity::ReasoningEffortDto::Xhigh)
            }
        },
        agent: submission.execution.delegate_agent.clone(),
        allowed_tools,
        acp_agent,
        acp_allowed_tools,
        options,
    }
}

fn gateway_structured_content(submission: &PromptSubmission) -> serde_json::Value {
    let mut blocks = Vec::with_capacity(submission.attachments.len().saturating_add(1));
    if !submission.text.trim().is_empty() {
        blocks.push(json!({"type": "text", "text": submission.text}));
    }
    blocks.extend(submission.attachments.iter().map(gateway_attachment_block));
    serde_json::Value::Array(blocks)
}

fn gateway_attachment_block(attachment: &PromptAttachment) -> serde_json::Value {
    match attachment.kind {
        PromptAttachmentKind::Image => json!({
            "type": "image",
            "mimeType": attachment.media_type.as_deref().unwrap_or("image/png"),
            "data": STANDARD.encode(&attachment.bytes),
        }),
        PromptAttachmentKind::PastedText => json!({
            "type": "text",
            "text": attachment.text.as_deref().unwrap_or_default(),
        }),
        PromptAttachmentKind::File | PromptAttachmentKind::Resource => json!({
            "type": "resource",
            "resource": {
                "uri": attachment.uri.as_deref().unwrap_or_default(),
                "text": attachment.text.as_deref().unwrap_or_default(),
            },
        }),
    }
}

const SUBAGENT_MODE_SYSTEM_PROMPT: &str = "## AgentTool-First Mode\n\n\
- Prefer AgentTool for meaningful independent workstreams when delegation improves speed, focus, or verification.\n\
- The parent agent may work directly when delegation would not help, and remains responsible for coordinating, verifying evidence, and synthesizing results.\n\
- If an AgentTool call fails, report the exact blocker and continue safely when the task can still be completed without that delegation.\n\
- Continue to follow all safety, permission, scope, and validation requirements.";

fn gateway_message_from_chat_message(message: &ChatMessage) -> serde_json::Value {
    json!({
        "role": gateway_role(message.role),
        "content": message.content,
    })
}

fn gateway_role(role: ChatRole) -> &'static str {
    match role {
        ChatRole::User => "user",
        ChatRole::Assistant => "assistant",
        ChatRole::System => "system",
        ChatRole::Developer => "developer",
        ChatRole::Tool => "tool",
    }
}
