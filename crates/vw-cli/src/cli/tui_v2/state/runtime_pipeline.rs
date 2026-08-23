//! runtime 事件到状态动作的标准化桥。
//!
//! 本模块负责把 `UiRuntimeEvent` 进一步规整为状态层可直接消费的动作序列：
//! - delta/step/terminal 落到既有 reducer 动作，统一汇聚成 `UiMessage`
//! - delta 内部的 `<think>` / `<thinking>` 与 `tool <name>\n{json}` 片段在此转换为 typed UI 消息
//! - runtime 侧尚未识别的事件，显式转换为 UI warning，而不是静默吞掉
//! - timeout/error 终态补一条错误消息，便于 renderer 在 transcript 中稳定展示

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use vw_gateway_client::GatewayChatUsage;

use super::{
    TuiAction, TuiState, TuiTerminalUpdate, TuiToolCallUpdate, TuiToolResultUpdate,
    reduce_tui_state, split_reasoning_marker,
};
use crate::cli::tui_v2::model::{
    UiErrorMessage, UiMessage, UiMessageBase, UiMessageId, UiSystemMessage, UiSystemMessageLevel,
    UiThinkingBlock, UiThinkingPhase, UiThinkingTiming, UiTokenUsage, UiToolCallState,
    UiTurnTerminal,
};
use crate::cli::tui_v2::runtime::stream_adapter::{UiRuntimeEvent, UiRuntimeTerminalEvent};

/// Canonical and API-compat thinking open tags. Prefer longer match when both could apply.
const THINK_OPEN_TAGS: &[&str] = &["<thinking>", "<think>"];
/// Canonical and API-compat thinking close tags.
const THINK_CLOSE_TAGS: &[&str] = &["</thinking>", "</think>"];
const REDACTED_THINK_OPEN_TAG: &str = "<redacted-thinking>";
const REDACTED_THINK_CLOSE_TAG: &str = "</redacted-thinking>";

fn find_earliest_tag(haystack: &str, tags: &[&str]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for tag in tags {
        if let Some(index) = haystack.find(tag) {
            best = Some(match best {
                Some((best_index, best_len)) if best_index < index => (best_index, best_len),
                Some((best_index, best_len)) if best_index == index && best_len >= tag.len() => {
                    (best_index, best_len)
                }
                _ => (index, tag.len()),
            });
        }
    }
    best
}

/// 将单个 runtime 事件应用到状态层。
pub(crate) fn apply_runtime_event(state: &mut TuiState, event: UiRuntimeEvent) {
    apply_runtime_event_at(state, event, wall_clock_ms());
}

/// 使用调用方提供的确定时间应用 runtime 事件。
pub(crate) fn apply_runtime_event_at(
    state: &mut TuiState,
    event: UiRuntimeEvent,
    event_time_ms: u64,
) {
    state.runtime.event_time_ms = Some(event_time_ms);
    state.runtime.last_event_time_ms = Some(event_time_ms);
    match event {
        UiRuntimeEvent::PlanRunTaskStarted { task_id } => {
            state.runtime.active_plan_run_task_id = Some(task_id);
        }
        UiRuntimeEvent::Delta(delta) => {
            apply_runtime_delta(state, delta);
        }
        UiRuntimeEvent::ContextLoaded { paths } => {
            if !paths.is_empty() {
                reduce_tui_state(
                    state,
                    TuiAction::MessagePushed(UiMessage::System(UiSystemMessage {
                        base: next_ui_message_base(state, "context-loaded"),
                        text: format!("已加载上下文文件：\n- {}", paths.join("\n- ")),
                        level: UiSystemMessageLevel::Info,
                    })),
                );
            }
        }
        UiRuntimeEvent::Message(message) => {
            reduce_tui_state(state, TuiAction::MessagePushed(message));
        }
        UiRuntimeEvent::StepStart { step_index, created_ms, model } => {
            reduce_tui_state(
                state,
                TuiAction::StepStarted { step_index, started_ms: created_ms, model },
            );
        }
        UiRuntimeEvent::StepFinish { step_index, finished_ms, usage, finish_reason, model } => {
            reduce_tui_state(
                state,
                TuiAction::StepFinished {
                    step_index,
                    finished_ms,
                    usage: token_usage_from_gateway(&usage),
                    finish_reason,
                    model,
                },
            );
        }
        UiRuntimeEvent::Terminal(terminal) => {
            if state.runtime.thinking_open {
                reduce_tui_state(state, TuiAction::ThinkingClosed);
                state.runtime.thinking_open = false;
                state.runtime.thinking_depth = 0;
                state.runtime.thinking_redacted = false;
            }
            apply_runtime_terminal(state, terminal);
            state.runtime.active_plan_run_task_id = None;
        }
        UiRuntimeEvent::PostToolRound
        | UiRuntimeEvent::TaskStateChanged { .. }
        | UiRuntimeEvent::UsageUpdated { .. } => {}
        UiRuntimeEvent::SessionMetadataChanged { session_id, title } => {
            if runtime_event_targets_current_session(state, session_id.as_deref())
                && let Some(title) = title
            {
                reduce_tui_state(state, TuiAction::SessionTitleSet(title));
            }
        }
        UiRuntimeEvent::Unknown { event_type } => {
            let event_type = event_type.unwrap_or_else(|| "unknown".to_string());
            reduce_tui_state(
                state,
                TuiAction::MessagePushed(UiMessage::System(UiSystemMessage {
                    base: next_ui_message_base(state, "runtime-warning"),
                    text: format!("Unsupported gateway event: {event_type}."),
                    level: UiSystemMessageLevel::Warning,
                })),
            );
        }
    }
    state.runtime.event_time_ms = None;
}

fn apply_runtime_delta(state: &mut TuiState, delta: String) {
    if state.runtime.thinking_redacted {
        if let Some(close_index) = delta.find(REDACTED_THINK_CLOSE_TAG) {
            state.runtime.thinking_open = false;
            state.runtime.thinking_depth = 0;
            state.runtime.thinking_redacted = false;
            let remaining = delta[close_index + REDACTED_THINK_CLOSE_TAG.len()..]
                .trim_start_matches(['\r', '\n']);
            if !remaining.is_empty() {
                apply_runtime_delta(state, remaining.to_string());
            }
        }
        return;
    }

    if let Some(open_index) = delta.find(REDACTED_THINK_OPEN_TAG) {
        let visible_prefix = &delta[..open_index];
        if !visible_prefix.is_empty() {
            apply_runtime_delta(state, visible_prefix.to_string());
        }

        append_redacted_thinking_placeholder(state);
        let after_open = &delta[open_index + REDACTED_THINK_OPEN_TAG.len()..];
        if let Some(close_index) = after_open.find(REDACTED_THINK_CLOSE_TAG) {
            let remaining = after_open[close_index + REDACTED_THINK_CLOSE_TAG.len()..]
                .trim_start_matches(['\r', '\n']);
            if !remaining.is_empty() {
                apply_runtime_delta(state, remaining.to_string());
            }
        } else {
            state.runtime.thinking_open = true;
            state.runtime.thinking_depth = 1;
            state.runtime.thinking_redacted = true;
        }
        return;
    }

    if !state.runtime.thinking_open
        && let Some(tool_update) = parse_tool_delta(delta.as_str())
    {
        reduce_tui_state(state, TuiAction::ToolCallUpdated(tool_update));
        return;
    }

    if !state.runtime.thinking_open
        && let Some((visible, reasoning)) = split_reasoning_marker(delta.as_str())
    {
        if !visible.trim().is_empty() {
            reduce_tui_state(state, TuiAction::AssistantDeltaReceived(visible.to_string()));
        }
        if !reasoning.is_empty() {
            reduce_tui_state(state, TuiAction::ThinkingDeltaReceived(reasoning.to_string()));
            reduce_tui_state(state, TuiAction::ThinkingClosed);
        }
        return;
    }

    let mut remaining = delta.as_str();
    while !remaining.is_empty() {
        let Some((tag_index, tag_len, is_open)) = find_thinking_boundary(remaining) else {
            let action = if state.runtime.thinking_open {
                TuiAction::ThinkingDeltaReceived(remaining.to_string())
            } else {
                TuiAction::AssistantDeltaReceived(remaining.to_string())
            };
            reduce_tui_state(state, action);
            break;
        };

        let prefix = &remaining[..tag_index];
        if state.runtime.thinking_open {
            if !prefix.is_empty() {
                reduce_tui_state(state, TuiAction::ThinkingDeltaReceived(prefix.to_string()));
            }
            if is_open {
                state.runtime.thinking_depth = state.runtime.thinking_depth.max(1).saturating_add(1);
            } else {
                state.runtime.thinking_depth = state.runtime.thinking_depth.max(1).saturating_sub(1);
                if state.runtime.thinking_depth == 0 {
                    reduce_tui_state(state, TuiAction::ThinkingClosed);
                    state.runtime.thinking_open = false;
                }
            }
        } else if is_open {
            if !prefix.trim().is_empty() {
                reduce_tui_state(state, TuiAction::AssistantDeltaReceived(prefix.to_string()));
            }
            state.runtime.thinking_open = true;
            state.runtime.thinking_depth = 1;
        } else if !prefix.trim().is_empty() {
            reduce_tui_state(state, TuiAction::AssistantDeltaReceived(prefix.to_string()));
        }

        remaining = &remaining[tag_index + tag_len..];
        if !state.runtime.thinking_open {
            remaining = remaining.trim_start_matches(['\r', '\n']);
        }
    }
}

fn find_thinking_boundary(input: &str) -> Option<(usize, usize, bool)> {
    let open = find_earliest_tag(input, THINK_OPEN_TAGS);
    let close = find_earliest_tag(input, THINK_CLOSE_TAGS);
    match (open, close) {
        (Some((open_index, open_len)), Some((close_index, close_len))) => {
            if open_index < close_index {
                Some((open_index, open_len, true))
            } else {
                Some((close_index, close_len, false))
            }
        }
        (Some((index, len)), None) => Some((index, len, true)),
        (None, Some((index, len))) => Some((index, len, false)),
        (None, None) => None,
    }
}

fn append_redacted_thinking_placeholder(state: &mut TuiState) {
    let event_time_ms = state.runtime.event_time_ms.unwrap_or_default();
    let base = next_ui_message_base(state, "thinking-redacted").with_created_ms(event_time_ms);
    reduce_tui_state(
        state,
        TuiAction::MessagePushed(UiMessage::Thinking(UiThinkingBlock {
            base,
            summary: Some("Thinking redacted".to_string()),
            content: String::new(),
            timing: vec![UiThinkingTiming {
                start_ms: event_time_ms,
                end_ms: Some(event_time_ms),
                last_update_ms: event_time_ms,
            }],
            collapsed: true,
            phase: UiThinkingPhase::Redacted,
        })),
    );
}

fn wall_clock_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

fn parse_tool_delta(delta: &str) -> Option<TuiToolCallUpdate> {
    let (first, rest) = delta.split_once('\n')?;
    let tool_name = first.trim().strip_prefix("tool ")?.trim();
    if tool_name.is_empty() {
        return None;
    }

    let payload = serde_json::from_str::<Value>(rest.trim()).ok()?;
    let status = payload
        .get("status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|status| !status.is_empty())?;

    let append_arguments = status == "input_json_delta";
    let arguments = if append_arguments {
        runtime_text_value(payload.get("input_json_delta"))
    } else {
        runtime_text_value(payload.get("input"))
    };
    let summary = runtime_text_value(payload.get("title"))
        .filter(|title| !title.eq_ignore_ascii_case(tool_name));

    let (state, result) = match status {
        "queued" => (UiToolCallState::Queued, None),
        "running" | "input_json_delta" => (UiToolCallState::Running, None),
        "completed" => (
            UiToolCallState::Complete,
            Some(TuiToolResultUpdate {
                content: runtime_text_value(payload.get("output")).unwrap_or_default(),
                is_error: false,
            }),
        ),
        "denied" | "error" | "failed" => (
            UiToolCallState::Failed,
            Some(TuiToolResultUpdate {
                content: runtime_text_value(payload.get("error"))
                    .or_else(|| runtime_text_value(payload.get("output")))
                    .unwrap_or_else(|| format!("tool {tool_name} failed")),
                is_error: true,
            }),
        ),
        "cancelled" | "canceled" => (UiToolCallState::Cancelled, None),
        _ => return None,
    };

    Some(TuiToolCallUpdate {
        tool_name: tool_name.to_string(),
        summary,
        arguments,
        append_arguments,
        state,
        result,
    })
}

fn runtime_text_value(value: Option<&Value>) -> Option<String> {
    let value = value?;
    match value {
        Value::Null => None,
        Value::String(text) => normalize_optional_string(text.to_string()),
        other => normalize_optional_string(other.to_string()),
    }
}

fn apply_runtime_terminal(state: &mut TuiState, terminal: UiRuntimeTerminalEvent) {
    reduce_tui_state(
        state,
        TuiAction::AssistantTerminalUpdated(TuiTerminalUpdate {
            terminal: ui_turn_terminal_from_runtime(&terminal),
            usage: terminal_usage(&terminal),
            message_id: terminal_message_id(&terminal),
            parent_message_id: terminal_parent_message_id(&terminal),
        }),
    );

    if let Some(message) = terminal_side_message(state, &terminal) {
        reduce_tui_state(state, TuiAction::MessagePushed(message));
    }
}

fn terminal_side_message(state: &TuiState, terminal: &UiRuntimeTerminalEvent) -> Option<UiMessage> {
    match terminal {
        UiRuntimeTerminalEvent::TimedOut { message, .. } => {
            Some(UiMessage::Error(UiErrorMessage {
                base: next_ui_message_base(state, "runtime-timeout"),
                message: message.clone(),
                recoverable: true,
            }))
        }
        UiRuntimeTerminalEvent::Error(message) => Some(UiMessage::Error(UiErrorMessage {
            base: next_ui_message_base(state, "runtime-error"),
            message: message.clone(),
            recoverable: true,
        })),
        UiRuntimeTerminalEvent::Done { .. } | UiRuntimeTerminalEvent::Cancelled { .. } => None,
    }
}

fn ui_turn_terminal_from_runtime(terminal: &UiRuntimeTerminalEvent) -> UiTurnTerminal {
    match terminal {
        UiRuntimeTerminalEvent::Done { finish_reason, .. } => {
            UiTurnTerminal::Done { finish_reason: finish_reason.clone() }
        }
        UiRuntimeTerminalEvent::Cancelled { reason, .. } => {
            UiTurnTerminal::Cancelled { reason: reason.clone() }
        }
        UiRuntimeTerminalEvent::TimedOut { message, .. } => {
            UiTurnTerminal::TimedOut { message: message.clone() }
        }
        UiRuntimeTerminalEvent::Error(message) => {
            UiTurnTerminal::Error { message: message.clone() }
        }
    }
}

fn terminal_usage(terminal: &UiRuntimeTerminalEvent) -> Option<UiTokenUsage> {
    match terminal {
        UiRuntimeTerminalEvent::Done { usage, .. }
        | UiRuntimeTerminalEvent::Cancelled { usage, .. }
        | UiRuntimeTerminalEvent::TimedOut { usage, .. } => {
            usage.as_ref().map(token_usage_from_gateway)
        }
        UiRuntimeTerminalEvent::Error(_) => None,
    }
}

fn terminal_message_id(terminal: &UiRuntimeTerminalEvent) -> Option<String> {
    match terminal {
        UiRuntimeTerminalEvent::Done { message_id, .. }
        | UiRuntimeTerminalEvent::Cancelled { message_id, .. }
        | UiRuntimeTerminalEvent::TimedOut { message_id, .. } => message_id.clone(),
        UiRuntimeTerminalEvent::Error(_) => None,
    }
}

fn terminal_parent_message_id(terminal: &UiRuntimeTerminalEvent) -> Option<String> {
    match terminal {
        UiRuntimeTerminalEvent::Done { parent_message_id, .. }
        | UiRuntimeTerminalEvent::Cancelled { parent_message_id, .. }
        | UiRuntimeTerminalEvent::TimedOut { parent_message_id, .. } => parent_message_id.clone(),
        UiRuntimeTerminalEvent::Error(_) => None,
    }
}

fn token_usage_from_gateway(usage: &GatewayChatUsage) -> UiTokenUsage {
    UiTokenUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cached_tokens: usage.cached_tokens,
        reasoning_tokens: usage.reasoning_tokens,
    }
}

fn next_ui_message_base(state: &TuiState, seed: &str) -> UiMessageBase {
    let mut base =
        UiMessageBase::new(UiMessageId::local(format!("ui-{seed}-{}", state.messages.len())));
    if let Some(session_id) = state.session.session_id.as_deref() {
        base = base.with_session_id(session_id);
    }
    base
}

fn normalize_optional_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn runtime_event_targets_current_session(state: &TuiState, session_id: Option<&str>) -> bool {
    match session_id.map(str::trim).filter(|value| !value.is_empty()) {
        Some(session_id) => state.session.session_id.as_deref() == Some(session_id),
        None => true,
    }
}
#[cfg(test)]
#[path = "runtime_pipeline_tests.rs"]
mod runtime_pipeline_tests;
