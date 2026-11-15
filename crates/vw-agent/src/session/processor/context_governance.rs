//! Desktop Chat 发送模型前的确定性上下文治理。

use crate::app::agent::provider::provider::ModelLimit;
// Invoked skill replay restoration belongs to session-engine.
use crate::app::agent::tools::ToolSpec;
use crate::session::session::{Role, Session};
use serde_json::Value;

pub(crate) const RECENT_TOOL_RESULTS_TO_KEEP: usize = 8;
pub(crate) const STALE_TOOL_RESULT_PLACEHOLDER: &str =
    "[Earlier tool result cleared to preserve conversation context]";
// Compaction threshold constants are owned by session_engine::compaction_budget.

// Stable invoked skill source key belongs to session-engine.

/// 重放技能时恢复的上下文更新，顺序与技能调用顺序一致。
// InvokedSkillContextUpdate has a single owner in session-engine.

const ESTIMATED_CHARS_PER_TOKEN: u64 = 4;
// Image data is encoded as a transport payload, not as text tokens. Counting
// the whole base64 body here can reject an otherwise valid vision request
// before the provider gets a chance to apply its image-token accounting.
const ESTIMATED_IMAGE_TOKENS: u64 = 4_096;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ContextCompactionResult {
    pub(crate) before_tokens: u64,
    pub(crate) after_tokens: u64,
    pub(crate) needs_internal_compaction: bool,
}

/// Pure sizing result consumed by the runtime compaction service.  This module intentionally
/// does not decide which conversation records may be discarded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ContextCompactionDecision {
    pub(crate) input_tokens: u64,
    pub(crate) needs_internal_compaction: bool,
}

pub(crate) use session_engine::compaction_budget::{effective_input_window, should_compact};
#[cfg(test)]
pub(crate) use session_engine::compaction_budget::summary_output_budget;

fn estimate_bytes_as_tokens(bytes: usize) -> u64 {
    (bytes as u64).saturating_add(ESTIMATED_CHARS_PER_TOKEN - 1) / ESTIMATED_CHARS_PER_TOKEN
}

fn estimate_value_tokens(value: &Value) -> u64 {
    let (value, image_count) = compact_image_payload_for_estimation(value);
    serde_json::to_vec(&value)
        .map(|bytes| estimate_bytes_as_tokens(bytes.len()))
        .unwrap_or(u64::MAX)
        .saturating_add(ESTIMATED_IMAGE_TOKENS.saturating_mul(image_count as u64))
}

fn compact_image_payload_for_estimation(value: &Value) -> (Value, usize) {
    let Some(content) = value.get("content").and_then(Value::as_str) else {
        return (value.clone(), 0);
    };
    let (cleaned, image_refs) = crate::app::agent::multimodal::parse_image_markers(content);
    let image_count = image_refs.iter().filter(|image_ref| is_data_image_uri(image_ref)).count();
    if image_count == 0 {
        return (value.clone(), 0);
    }

    let mut estimated_content = cleaned;
    for image_ref in image_refs {
        if is_data_image_uri(&image_ref) {
            estimated_content.push_str("\n[image attachment]");
        } else {
            estimated_content.push_str("\n[IMAGE:");
            estimated_content.push_str(&image_ref);
            estimated_content.push(']');
        }
    }

    // Keep the original message untouched; this value exists only for the
    // deterministic size estimate.
    let mut estimated = value.clone();
    if let Some(object) = estimated.as_object_mut() {
        object.insert("content".to_string(), Value::String(estimated_content));
    }
    (estimated, image_count)
}

fn is_data_image_uri(value: &str) -> bool {
    value
        .get(.."data:image/".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:image/"))
}

fn estimate_context_tokens(messages: &[Value], system: &[String], tools: &[ToolSpec]) -> u64 {
    let message_tokens = messages
        .iter()
        .fold(0_u64, |total, message| total.saturating_add(estimate_value_tokens(message)));
    let system_tokens = system
        .iter()
        .fold(0_u64, |total, prompt| total.saturating_add(estimate_bytes_as_tokens(prompt.len())));
    let tool_tokens = tools.iter().fold(0_u64, |total, tool| {
        total.saturating_add(estimate_value_tokens(
            &serde_json::to_value(tool).unwrap_or(Value::Null),
        ))
    });

    message_tokens.saturating_add(system_tokens).saturating_add(tool_tokens)
}

/// 以实际 input usage 为下限，缺失或非正 usage 时回退到保守估算。
pub(crate) fn context_usage_tokens(
    messages: &[Value],
    system: &[String],
    tools: &[ToolSpec],
    observed_input_tokens: Option<i64>,
) -> u64 {
    let estimated = estimate_context_tokens(messages, system, tools);
    observed_input_tokens
        .filter(|tokens| *tokens > 0)
        .map(|tokens| tokens as u64)
        .map_or(estimated, |observed| observed.max(estimated))
}

/// Estimate whether a request must be compacted without mutating its protocol history.
pub(crate) fn context_governance_decision(
    messages: &[Value],
    system: &[String],
    tools: &[ToolSpec],
    limit: &ModelLimit,
    observed_input_tokens: Option<i64>,
) -> ContextCompactionDecision {
    let input_tokens = context_usage_tokens(messages, system, tools, observed_input_tokens);
    ContextCompactionDecision {
        input_tokens,
        needs_internal_compaction: should_compact(input_tokens, limit),
    }
}

fn message_role(message: &Value) -> Option<&str> {
    message.get("role").and_then(Value::as_str)
}

#[cfg(test)]
pub(crate) use session_engine::invoked_skill_context::restore_invoked_skill_context;

/// 在下一次 LLM 请求前计算是否必须调用内部压缩服务。
///
/// 历史替换属于 `session::compaction`：这里绝不能用确定性裁剪替代模型摘要。
pub(crate) fn compact_context_for_next_request(
    messages: &mut Vec<Value>,
    system: &[String],
    tools: &[ToolSpec],
    limit: &ModelLimit,
    observed_input_tokens: Option<i64>,
) -> Result<ContextCompactionResult, String> {
    let decision =
        context_governance_decision(messages, system, tools, limit, observed_input_tokens);
    Ok(ContextCompactionResult {
        before_tokens: decision.input_tokens,
        after_tokens: decision.input_tokens,
        needs_internal_compaction: decision.needs_internal_compaction,
    })
}

/// 裁剪旧的非系统历史消息，保留系统消息和最近的对话。
pub(crate) fn trim_session_history(session: &mut Session, max_non_system_messages: usize) -> usize {
    let non_system_count = session
        .messages
        .iter()
        .filter(|message| !matches!(message.role, Role::System | Role::Developer))
        .count();
    let removed = non_system_count.saturating_sub(max_non_system_messages);
    if removed == 0 {
        return 0;
    }

    let mut remaining = removed;
    session.messages.retain(|message| {
        if matches!(message.role, Role::System | Role::Developer) {
            return true;
        }
        if remaining == 0 {
            return true;
        }
        remaining = remaining.saturating_sub(1);
        false
    });
    removed
}

/// 清理较旧的结构化工具结果内容，但保留协议配对字段和最近结果。
pub(crate) fn clear_stale_tool_results(messages: &mut [Value], keep_recent: usize) -> usize {
    let tool_result_count = messages
        .iter()
        .filter(|message| message.get("role").and_then(Value::as_str) == Some("tool"))
        .count();
    let mut remaining_to_clear = tool_result_count.saturating_sub(keep_recent);
    let mut cleared = 0;

    for message in messages {
        if remaining_to_clear == 0 {
            break;
        }
        let Some(object) = message.as_object_mut() else {
            continue;
        };
        if object.get("role").and_then(Value::as_str) != Some("tool") {
            continue;
        }

        remaining_to_clear = remaining_to_clear.saturating_sub(1);
        let content = object.entry("content").or_insert(Value::Null);
        if content.as_str() == Some(STALE_TOOL_RESULT_PLACEHOLDER) {
            continue;
        }
        *content = Value::String(STALE_TOOL_RESULT_PLACEHOLDER.to_string());
        cleared += 1;
    }

    cleared
}

#[cfg(test)]
#[path = "context_governance_tests.rs"]
mod context_governance_tests;
