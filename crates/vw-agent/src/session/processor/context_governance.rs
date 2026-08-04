//! Desktop Chat 发送模型前的确定性上下文治理。

use crate::session::session::{Role, Session};
use serde_json::Value;

pub(crate) const RECENT_TOOL_RESULTS_TO_KEEP: usize = 8;
pub(crate) const STALE_TOOL_RESULT_PLACEHOLDER: &str =
    "[Earlier tool result cleared to preserve conversation context]";

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
pub(crate) fn clear_stale_tool_results(
    messages: &mut [Value],
    keep_recent: usize,
) -> usize {
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
