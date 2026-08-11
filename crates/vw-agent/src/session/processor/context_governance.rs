//! Desktop Chat 发送模型前的确定性上下文治理。

use crate::app::agent::provider::provider::ModelLimit;
use crate::app::agent::skills::InvokedSkillReplay;
use crate::app::agent::tools::ToolSpec;
use crate::session::session::{Role, Session};
use serde_json::Value;

pub(crate) const RECENT_TOOL_RESULTS_TO_KEEP: usize = 8;
pub(crate) const STALE_TOOL_RESULT_PLACEHOLDER: &str =
    "[Earlier tool result cleared to preserve conversation context]";
pub(crate) const AUTO_COMPACTION_THRESHOLD_PERCENT: u64 = 90;

const INVOKED_SKILL_SOURCE_FIELD: &str = "vw_invoked_skill_source";

/// 重放技能时恢复的上下文更新，顺序与技能调用顺序一致。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InvokedSkillContextUpdate {
    pub(crate) key: String,
    pub(crate) value: Value,
}

const ESTIMATED_CHARS_PER_TOKEN: u64 = 4;
// Image data is encoded as a transport payload, not as text tokens. Counting
// the whole base64 body here can reject an otherwise valid vision request
// before the provider gets a chance to apply its image-token accounting.
const ESTIMATED_IMAGE_TOKENS: u64 = 4_096;
const AUTO_COMPACTION_MAX_PASSES: usize = 2;
const RECENT_NON_SYSTEM_MESSAGES_TO_KEEP: usize = 6;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ContextCompactionResult {
    pub(crate) before_tokens: u64,
    pub(crate) after_tokens: u64,
    pub(crate) cleared_tool_results: usize,
    pub(crate) trimmed_messages: usize,
    pub(crate) passes: usize,
}

/// 计算可用于当前请求输入的窗口。
///
/// 显式输入限制优先；没有输入限制时，从总窗口中预留最大输出空间。
pub(crate) fn effective_input_window(limit: &ModelLimit) -> u64 {
    vw_shared::provider::state::effective_input_window(limit)
}

/// 判断当前输入是否达到自动压缩边界。
pub(crate) fn should_compact(input_tokens: u64, limit: &ModelLimit) -> bool {
    if limit.context == 0 && limit.input.is_none() {
        return false;
    }
    let window = effective_input_window(limit);
    if window == 0 {
        return input_tokens > 0;
    }

    let threshold =
        window.saturating_mul(AUTO_COMPACTION_THRESHOLD_PERCENT).saturating_add(99) / 100;
    input_tokens >= threshold
}

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

fn message_role(message: &Value) -> Option<&str> {
    message.get("role").and_then(Value::as_str)
}

fn is_transient_protocol_message(message: &Value) -> bool {
    message_role(message) == Some("user")
        && message
            .get("content")
            .and_then(Value::as_str)
            .is_some_and(|content| content.trim_start().starts_with("<system-reminder>"))
}

/// 在压缩清理后将已调用技能的临时元用户消息放回历史前方。
///
/// 该函数按稳定来源标识替换旧注入，因此可以在每次压缩后调用而不会重复注入。
pub(crate) fn restore_invoked_skill_context(
    messages: &mut Vec<Value>,
    replays: &[InvokedSkillReplay],
) -> Vec<InvokedSkillContextUpdate> {
    messages.retain(|message| message.get(INVOKED_SKILL_SOURCE_FIELD).is_none());

    let mut restored_messages = replays
        .iter()
        .map(|replay| {
            serde_json::json!({
                "role": "user",
                "content": replay.content,
                INVOKED_SKILL_SOURCE_FIELD: replay.stable_id,
            })
        })
        .collect::<Vec<_>>();
    restored_messages.append(messages);
    *messages = restored_messages;

    replays
        .iter()
        .flat_map(|replay| {
            let mut updates = vec![InvokedSkillContextUpdate {
                key: "skill.name".to_string(),
                value: Value::String(replay.stable_id.clone()),
            }];
            if !replay.allowed_tools.is_empty() {
                updates.push(InvokedSkillContextUpdate {
                    key: "skill.allowed_tools".to_string(),
                    value: serde_json::json!(replay.allowed_tools),
                });
            }
            if let Some(model) = &replay.model {
                updates.push(InvokedSkillContextUpdate {
                    key: "skill.model".to_string(),
                    value: Value::String(model.clone()),
                });
            }
            if let Some(effort) = &replay.effort {
                updates.push(InvokedSkillContextUpdate {
                    key: "skill.effort".to_string(),
                    value: Value::String(effort.clone()),
                });
            }
            updates
        })
        .collect()
}

fn tool_call_ids(message: &Value) -> Vec<String> {
    message
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|call| {
            call.get("id")
                .and_then(Value::as_str)
                .or_else(|| call.get("tool_call_id").and_then(Value::as_str))
                .map(ToOwned::to_owned)
        })
        .collect()
}

/// 按消息组裁剪旧历史，工具调用和对应结果始终一起保留或移除。
fn trim_llm_history(messages: &mut Vec<Value>, max_non_system_messages: usize) -> usize {
    let non_system_count = messages
        .iter()
        .filter(|message| {
            message_role(message) != Some("system") && !is_transient_protocol_message(message)
        })
        .count();
    let mut remaining_to_remove = non_system_count.saturating_sub(max_non_system_messages);
    if remaining_to_remove == 0 {
        return 0;
    }

    let mut consumed = vec![false; messages.len()];
    let mut groups = Vec::<Vec<usize>>::new();
    for index in 0..messages.len() {
        if consumed[index]
            || message_role(&messages[index]) == Some("system")
            || is_transient_protocol_message(&messages[index])
        {
            continue;
        }

        consumed[index] = true;
        let mut group = vec![index];
        let call_ids = if message_role(&messages[index]) == Some("assistant") {
            tool_call_ids(&messages[index])
        } else {
            Vec::new()
        };
        if !call_ids.is_empty() {
            for candidate in index + 1..messages.len() {
                if consumed[candidate] || message_role(&messages[candidate]) != Some("tool") {
                    continue;
                }
                let tool_call_id = messages[candidate].get("tool_call_id").and_then(Value::as_str);
                if tool_call_id.is_some_and(|id| call_ids.iter().any(|call_id| call_id == id)) {
                    consumed[candidate] = true;
                    group.push(candidate);
                }
            }
        }
        groups.push(group);
    }

    let mut remove = vec![false; messages.len()];
    for group in groups {
        if remaining_to_remove == 0 {
            break;
        }
        for index in &group {
            remove[*index] = true;
        }
        remaining_to_remove = remaining_to_remove.saturating_sub(group.len());
    }

    let removed = remove.iter().filter(|removed| **removed).count();
    if removed > 0 {
        let mut index = 0;
        messages.retain(|_| {
            let keep = !remove[index];
            index += 1;
            keep
        });
    }
    removed
}

/// 在下一次 LLM 请求前执行有限次数的确定性上下文压缩。
pub(crate) fn compact_context_for_next_request(
    messages: &mut Vec<Value>,
    system: &[String],
    tools: &[ToolSpec],
    limit: &ModelLimit,
    observed_input_tokens: Option<i64>,
) -> Result<ContextCompactionResult, String> {
    let before_tokens = context_usage_tokens(messages, system, tools, observed_input_tokens);
    if !should_compact(before_tokens, limit) {
        return Ok(ContextCompactionResult {
            before_tokens,
            after_tokens: before_tokens,
            ..ContextCompactionResult::default()
        });
    }

    let mut result = ContextCompactionResult { before_tokens, ..Default::default() };
    for pass in 1..=AUTO_COMPACTION_MAX_PASSES {
        result.passes = pass;
        result.cleared_tool_results += clear_stale_tool_results(
            messages,
            if pass == 1 { RECENT_TOOL_RESULTS_TO_KEEP } else { 0 },
        );
        if pass == AUTO_COMPACTION_MAX_PASSES {
            result.trimmed_messages +=
                trim_llm_history(messages, RECENT_NON_SYSTEM_MESSAGES_TO_KEEP);
        }

        result.after_tokens = estimate_context_tokens(messages, system, tools);
        if !should_compact(result.after_tokens, limit) {
            return Ok(result);
        }
    }

    Err(format!(
        "上下文压缩后仍超过限制（估算 {} tokens，输入窗口 {} tokens）",
        result.after_tokens,
        effective_input_window(limit)
    ))
}

/// 裁剪旧的非系统历史消息，保留系统消息和最近的对话。
pub(crate) fn trim_session_history(session: &mut Session, max_non_system_messages: usize) -> usize {
    let non_system_count =
        session.messages.iter().filter(|message| message.role != Role::System).count();
    let removed = non_system_count.saturating_sub(max_non_system_messages);
    if removed == 0 {
        return 0;
    }

    let mut remaining = removed;
    session.messages.retain(|message| {
        if message.role == Role::System {
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
