//! Gateway 消息、工具结果与审批结果的界面投影。

use super::{
    CrosstermEvent,
    KeyCode,
    KeyEvent,
    KeyModifiers,
    PermissionDecision,
    PermissionMode,
    ToolResultContentDto,
    ToolResultDto,
    TuiController,
    TuiControllerCommand,
    TuiControllerEvent,
    TuiOverlayCommand,
    TuiState,
    UiMessage,
    UiOverlay,
    UiRuntimeEvent,
    UiRuntimeTerminalEvent,
    V3ApprovalDecision,
    V3Event,
    V3FileChange,
    V3FileDiff,
    V3FileDiffEntry,
    V3UserInputAnswer,
};

pub(in crate::cli::tui) fn structured_patch_diff(payload: &serde_json::Value) -> Option<V3FileDiff> {
    let result = payload.get("result")?;
    let result: ToolResultDto = serde_json::from_value(result.clone()).ok()?;
    if result.success == Some(false) {
        return None;
    }

    let mut files: Vec<V3FileDiffEntry> = Vec::new();
    for content in result.content {
        let ToolResultContentDto::StructuredPatch { hunks } = content else {
            continue;
        };
        for hunk in hunks {
            let Some(path) = hunk.path.as_deref().map(str::trim).filter(|path| !path.is_empty())
            else {
                continue;
            };
            let mut unified_diff = String::new();
            if !hunk.header.trim().is_empty() {
                unified_diff.push_str(&hunk.header);
                unified_diff.push('\n');
            }
            for line in hunk.lines {
                unified_diff.push_str(&line);
                unified_diff.push('\n');
            }
            if unified_diff.trim().is_empty() {
                continue;
            }

            if let Some(file) = files.iter_mut().find(|file| file.path == path) {
                let V3FileChange::Update { unified_diff: existing } = &mut file.change else {
                    unreachable!("structured patch entries are always update diffs");
                };
                existing.push_str(&unified_diff);
            } else {
                files.push(V3FileDiffEntry {
                    path: path.to_string(),
                    change: V3FileChange::Update { unified_diff },
                });
            }
        }
    }

    (!files.is_empty()).then_some(V3FileDiff { files })
}

pub(super) fn tool_command(payload: &serde_json::Value) -> String {
    tool_payload_text(payload, "input")
        .or_else(|| tool_payload_text(payload, "title"))
        .unwrap_or_default()
}

pub(super) fn tool_payload_text(payload: &serde_json::Value, key: &str) -> Option<String> {
    let value = payload.get(key)?;
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::String(text) => {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_string())
        }
        value => Some(value.to_string()),
    }
}

pub(super) fn tool_exit_code(payload: &serde_json::Value) -> Option<i32> {
    payload
        .get("exit_code")
        .or_else(|| payload.get("exitCode"))
        .and_then(serde_json::Value::as_i64)
        .and_then(|code| i32::try_from(code).ok())
}

pub(super) fn tool_error(payload: &serde_json::Value, output: String, fallback: &str) -> String {
    tool_payload_text(payload, "error")
        .or_else(|| (!output.is_empty()).then_some(output))
        .unwrap_or_else(|| fallback.to_string())
}

pub(super) fn non_negative_tokens(tokens: i64) -> u64 {
    u64::try_from(tokens).unwrap_or_default()
}

pub(super) fn permission_display(state: &TuiState) -> String {
    let mode = match state.session_options.permission_mode {
        PermissionMode::Default => "Default",
        PermissionMode::Plan => "Plan",
        PermissionMode::AcceptEdits => "Accept edits",
        PermissionMode::Bypass => "Bypass",
    };
    let access =
        if state.session_options.full_access_enabled() { "full access" } else { "workspace write" };
    format!("{mode} · {access}")
}

pub(super) fn controller_key(
    controller: &mut TuiController,
    state: &mut TuiState,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> TuiControllerCommand {
    controller.handle_event(
        state,
        TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(code, modifiers))),
    )
}

pub(super) fn permission_command(
    state: &TuiState,
    request_id: &str,
    decision: V3ApprovalDecision,
) -> Result<TuiControllerCommand, String> {
    let Some(UiOverlay::Permissions(overlay)) = state.overlays.active() else {
        return Err("no permission approval is active".to_string());
    };
    let request =
        overlay.queue.current().ok_or_else(|| "no pending permission request".to_string())?;
    if state.session.session_id.as_deref() != Some(request.session_id.as_str()) {
        return Err("approval request does not belong to the current session".to_string());
    }
    if request.id != request_id {
        return Err("approval request does not match the active permission".to_string());
    }

    let decision = match decision {
        V3ApprovalDecision::Allow => PermissionDecision::Once,
        V3ApprovalDecision::AllowForSession => PermissionDecision::Always,
        V3ApprovalDecision::Deny => PermissionDecision::Reject { reason: None },
        V3ApprovalDecision::Cancel => {
            PermissionDecision::Reject { reason: Some("cancelled by user".to_string()) }
        }
        V3ApprovalDecision::Custom(reason) => PermissionDecision::Reject { reason: Some(reason) },
    };
    if matches!(decision, PermissionDecision::Always) && !request.always_available() {
        return Err("the active permission does not allow a session-wide grant".to_string());
    }
    Ok(TuiControllerCommand::Overlay(TuiOverlayCommand::PermissionDecision {
        request_id: request.id.clone(),
        decision,
    }))
}

pub(super) fn confirmation_command(
    state: &TuiState,
    request_id: &str,
    decision: V3ApprovalDecision,
) -> Result<TuiControllerCommand, String> {
    let Some(UiOverlay::Confirm(confirmation)) = state.overlays.active() else {
        return Err("no session confirmation is active".to_string());
    };
    if confirmation.request_id != request_id {
        return Err("confirmation request does not match the active session intent".to_string());
    }
    let confirmed = match decision {
        V3ApprovalDecision::Allow => true,
        V3ApprovalDecision::Deny | V3ApprovalDecision::Cancel => false,
        V3ApprovalDecision::AllowForSession => {
            return Err("session mutations cannot be approved for the whole session".to_string());
        }
        V3ApprovalDecision::Custom(_) => {
            return Err(
                "session mutation confirmation does not accept custom decisions".to_string()
            );
        }
    };
    Ok(TuiControllerCommand::Overlay(TuiOverlayCommand::ConfirmDecision {
        request_id: confirmation.request_id.clone(),
        intent: confirmation.intent.clone(),
        confirmed,
    }))
}

pub(super) fn question_command(
    state: &TuiState,
    request_id: &str,
    answers: Vec<V3UserInputAnswer>,
) -> Result<TuiControllerCommand, String> {
    let mut overlay = state
        .tasks
        .pending_questions
        .iter()
        .find(|overlay| overlay.request_id == request_id)
        .cloned()
        .ok_or_else(|| "user-input request is not pending".to_string())?;
    if state.session.session_id.as_deref() != Some(overlay.session_id.as_str()) {
        return Err("user-input request does not belong to the current session".to_string());
    }
    if answers.is_empty() {
        return Ok(TuiControllerCommand::Overlay(TuiOverlayCommand::QuestionRejected(overlay)));
    }
    if answers.len() != overlay.prompts.len() {
        return Err("user-input answer shape does not match the pending request".to_string());
    }

    let mut ordered = vec![None; overlay.prompts.len()];
    for answer in answers {
        let index = answer
            .question_id
            .parse::<usize>()
            .map_err(|_| "user-input question id is not a bridge index".to_string())?;
        let slot = ordered
            .get_mut(index)
            .ok_or_else(|| "user-input question id is out of range".to_string())?;
        if slot.replace(answer.values).is_some() {
            return Err("user-input question was answered more than once".to_string());
        }
    }
    overlay.answers = ordered
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "user-input request has an unanswered question".to_string())?;
    Ok(TuiControllerCommand::Overlay(TuiOverlayCommand::QuestionSubmitted(overlay)))
}

pub(super) fn project_message_delta(
    before: &[UiMessage],
    after: &[UiMessage],
    turn_id: &str,
    events: &mut Vec<V3Event>,
) {
    for (index, message) in after.iter().enumerate() {
        let previous = before.get(index);
        if previous == Some(message) {
            continue;
        }
        match (previous, message) {
            (Some(UiMessage::Assistant(previous)), UiMessage::Assistant(current))
                if current.text.starts_with(previous.text.as_str()) =>
            {
                let delta = current.text[previous.text.len()..].to_string();
                if !delta.is_empty() {
                    events.push(V3Event::TextDelta { turn_id: turn_id.to_string(), delta });
                }
            }
            (Some(UiMessage::Thinking(previous)), UiMessage::Thinking(current))
                if current.content.starts_with(previous.content.as_str()) =>
            {
                let delta = current.content[previous.content.len()..].to_string();
                if !delta.is_empty() {
                    events.push(V3Event::ThinkingDelta { turn_id: turn_id.to_string(), delta });
                }
            }
            (_, UiMessage::Assistant(message)) if !message.text.is_empty() => {
                events.push(V3Event::TextDelta {
                    turn_id: turn_id.to_string(),
                    delta: message.text.clone(),
                });
            }
            (_, UiMessage::Thinking(message)) if !message.content.is_empty() => {
                events.push(V3Event::ThinkingDelta {
                    turn_id: turn_id.to_string(),
                    delta: message.content.clone(),
                });
            }
            (_, UiMessage::ToolCall(message)) => events.push(V3Event::ToolStarted {
                turn_id: turn_id.to_string(),
                call_id: message.call_id.clone().unwrap_or_else(|| "gateway-tool".to_string()),
                tool: message.tool_name.clone(),
                command: message
                    .arguments
                    .clone()
                    .or_else(|| message.summary.clone())
                    .unwrap_or_default(),
            }),
            (_, UiMessage::ToolResult(message)) if message.is_error => {
                events.push(V3Event::ToolFailed {
                    turn_id: turn_id.to_string(),
                    call_id: message.call_id.clone().unwrap_or_else(|| "gateway-tool".to_string()),
                    message: message.content.clone(),
                    duration_ms: 0,
                });
            }
            (_, UiMessage::ToolResult(message)) => events.push(V3Event::ToolCompleted {
                turn_id: turn_id.to_string(),
                call_id: message.call_id.clone().unwrap_or_else(|| "gateway-tool".to_string()),
                output: message.content.clone(),
                exit_code: 0,
                duration_ms: 0,
            }),
            _ => {}
        }
    }
}

pub(super) fn project_non_message_event(event: &UiRuntimeEvent, turn_id: &str, events: &mut Vec<V3Event>) {
    let UiRuntimeEvent::Unknown { event_type } = event else {
        return;
    };
    events.push(V3Event::Unknown {
        type_name: "unknown_gateway_event".to_string(),
        detail: format!(
            "{turn_id}: {}",
            event_type.clone().unwrap_or_else(|| "unknown".to_string())
        ),
    });
}

pub(super) fn v3_terminal_event(turn_id: String, terminal: UiRuntimeTerminalEvent) -> V3Event {
    match terminal {
        UiRuntimeTerminalEvent::Done { .. } => V3Event::TurnCompleted { turn_id },
        UiRuntimeTerminalEvent::Cancelled { reason, .. } => V3Event::TurnCancelled {
            turn_id,
            reason: reason.unwrap_or_else(|| "cancelled".to_string()),
        },
        UiRuntimeTerminalEvent::TimedOut { message, .. }
        | UiRuntimeTerminalEvent::Error(message) => V3Event::TurnFailed { turn_id, message },
    }
}
