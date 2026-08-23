//! Explicit adapter between the existing Gateway/v2 pipeline and the copied v3 UI contract.

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use vw_gateway_client::{
    GatewayChatStreamEvent, GatewayChatStreamRequest, GatewayChatToolEvent,
    GatewayChatToolEventKind, GatewayChatToolStatus, normalize_chat_tool_event,
};
use vw_tui_v3::{
    V3Action, V3ApprovalDecision, V3Event, V3SessionDisplayData, V3TokenUsage, V3UserInputAnswer,
};

use super::super::tui_v2::app::gateway_stream_request_from_state;
use super::super::tui_v2::controller::{
    TuiController, TuiControllerCommand, TuiControllerEvent, TuiOverlayCommand,
};
use super::super::tui_v2::model::permission::PermissionDecision;
use super::super::tui_v2::model::{PermissionMode, UiMessage, UiOverlay};
use super::super::tui_v2::runtime::gateway::GatewayUiRuntime;
use super::super::tui_v2::runtime::stream_adapter::{
    UiRuntimeEvent, UiRuntimeTerminalEvent, adapt_gateway_stream_event,
};
use super::super::tui_v2::state::selectors::select_status_summary;
use super::super::tui_v2::state::{TuiAction, TuiState, apply_runtime_event, reduce_tui_state};

/// Stateful turn-id allocator for the product-neutral v3 event vocabulary.
#[derive(Debug, Default)]
pub(crate) struct GatewayV3Bridge {
    next_turn: u64,
    active_turn_id: Option<String>,
    tool_started_ms: std::collections::HashMap<String, u64>,
    tool_output: std::collections::HashMap<String, String>,
}

impl GatewayV3Bridge {
    /// Projects local runtime state into the read-only welcome/session/status contract.
    pub(crate) fn session_display_data(
        runtime: &GatewayUiRuntime,
        state: &TuiState,
    ) -> V3SessionDisplayData {
        let summary = select_status_summary(state);
        let model = summary
            .model_name
            .or_else(|| state.session_options.model.clone())
            .unwrap_or_else(|| "default".to_string());
        let permissions = permission_display(state);
        let session_title = summary.title.trim();
        let session_title = (!session_title.is_empty()).then(|| session_title.to_string());
        let token_usage = summary.token_usage;

        V3SessionDisplayData {
            version: env!("CARGO_PKG_VERSION").to_string(),
            cwd: runtime.directory().to_path_buf(),
            gateway_endpoint: runtime.endpoint().describe(),
            model,
            permissions,
            session_id: summary.session_id,
            session_title,
            token_usage: V3TokenUsage {
                input: non_negative_tokens(token_usage.input_tokens),
                output: non_negative_tokens(token_usage.output_tokens),
                cached: non_negative_tokens(token_usage.cached_tokens),
                reasoning: non_negative_tokens(token_usage.reasoning_tokens),
            },
            resumed: state.session.created_ms != 0 || !state.messages.is_empty(),
        }
    }

    /// Runs the real Gateway adapter and v2 runtime pipeline, then projects the resulting UI delta.
    pub(crate) fn adapt_gateway_event(
        &mut self,
        state: &mut TuiState,
        event: GatewayChatStreamEvent,
    ) -> Vec<V3Event> {
        let tool_event = match &event {
            GatewayChatStreamEvent::Other(value) => normalize_chat_tool_event(value),
            _ => None,
        };
        let runtime_event = adapt_gateway_stream_event(event);
        let terminal = match &runtime_event {
            UiRuntimeEvent::Terminal(terminal) => Some(terminal.clone()),
            _ => None,
        };
        let before = state.messages.clone();
        let starts_turn = self.active_turn_id.is_none();
        let turn_id = self.active_turn_id();
        apply_runtime_event(state, runtime_event.clone());

        let mut events = Vec::new();
        if starts_turn {
            events.push(V3Event::TurnStarted { turn_id: turn_id.clone() });
        }
        if let Some(tool_event) = tool_event {
            self.project_tool_event(tool_event, &turn_id, &mut events);
        } else {
            project_message_delta(&before, &state.messages, &turn_id, &mut events);
        }
        project_non_message_event(&runtime_event, &turn_id, &mut events);

        if let Some(terminal) = terminal {
            events.push(v3_terminal_event(turn_id, terminal));
            self.active_turn_id = None;
            self.tool_started_ms.clear();
            self.tool_output.clear();
        }
        events
    }

    /// Routes a v3 UI action through existing controller commands and submission construction.
    pub(crate) fn controller_command(
        &mut self,
        state: &mut TuiState,
        controller: &mut TuiController,
        action: V3Action,
    ) -> Result<TuiControllerCommand, String> {
        match action {
            V3Action::None
            | V3Action::Select { .. }
            | V3Action::Attach(_)
            | V3Action::RemoveAttachment { .. }
            | V3Action::ToggleShortcuts => Ok(TuiControllerCommand::Continue),
            V3Action::Submit { text, attachments } | V3Action::Queue { text, attachments } => {
                if !attachments.is_empty() {
                    return Err(
                        "v3 attachments cannot be submitted until the existing request path supports them"
                            .to_string(),
                    );
                }
                reduce_tui_state(state, TuiAction::PromptValueSet(text));
                let command = controller.handle_event(
                    state,
                    TuiControllerEvent::Terminal(CrosstermEvent::Key(KeyEvent::new(
                        KeyCode::Enter,
                        KeyModifiers::NONE,
                    ))),
                );
                if let TuiControllerCommand::SubmitPrompt(submission) = &command {
                    reduce_tui_state(
                        state,
                        TuiAction::PromptSubmissionStarted(submission.clone()),
                    );
                }
                Ok(command)
            }
            V3Action::Interrupt | V3Action::Cancel => {
                Ok(TuiControllerCommand::CancelActiveSubmission)
            }
            V3Action::HistoryPrevious => {
                Ok(controller_key(controller, state, KeyCode::Up, KeyModifiers::NONE))
            }
            V3Action::HistoryNext => {
                Ok(controller_key(controller, state, KeyCode::Down, KeyModifiers::NONE))
            }
            V3Action::InsertNewline => {
                Ok(controller_key(controller, state, KeyCode::Enter, KeyModifiers::SHIFT))
            }
            V3Action::ShowTranscript => {
                Ok(controller_key(controller, state, KeyCode::Char('o'), KeyModifiers::CONTROL))
            }
            V3Action::OpenExternalEditor => {
                Err("v3 external-editor dispatch is not wired to the existing controller"
                    .to_string())
            }
            V3Action::ResolveApproval { request_id, decision } => {
                permission_command(state, request_id.as_str(), decision)
            }
            V3Action::AnswerUserInput { request_id, answers } => {
                question_command(state, request_id.as_str(), answers)
            }
        }
    }

    /// Delegates request construction to the existing v2 request builder.
    pub(crate) fn request_for_command(
        state: &TuiState,
        command: &TuiControllerCommand,
    ) -> Option<GatewayChatStreamRequest> {
        match command {
            TuiControllerCommand::SubmitPrompt(submission) => {
                Some(gateway_stream_request_from_state(state, submission))
            }
            _ => None,
        }
    }

    fn active_turn_id(&mut self) -> String {
        if let Some(turn_id) = self.active_turn_id.as_ref() {
            return turn_id.clone();
        }
        self.next_turn = self.next_turn.saturating_add(1);
        let turn_id = format!("gateway-turn-{}", self.next_turn);
        self.active_turn_id = Some(turn_id.clone());
        turn_id
    }

    fn project_tool_event(
        &mut self,
        event: GatewayChatToolEvent,
        turn_id: &str,
        events: &mut Vec<V3Event>,
    ) {
        let call_id = event.call_id.clone();
        match event.kind {
            GatewayChatToolEventKind::Start => {
                self.tool_started_ms.entry(call_id.clone()).or_insert(event.occurred_ms);
                self.tool_output.remove(call_id.as_str());
                events.push(V3Event::ToolStarted {
                    turn_id: turn_id.to_string(),
                    call_id,
                    tool: event.name,
                    command: tool_command(&event.payload),
                });
            }
            GatewayChatToolEventKind::Update => {
                if let Some(output) = tool_payload_text(&event.payload, "output") {
                    let previous = self.tool_output.entry(call_id.clone()).or_default();
                    let delta = output.strip_prefix(previous.as_str()).unwrap_or(output.as_str());
                    if !delta.is_empty() {
                        events.push(V3Event::ToolOutputDelta {
                            turn_id: turn_id.to_string(),
                            call_id,
                            delta: delta.to_string(),
                        });
                    }
                    *previous = output;
                }
            }
            GatewayChatToolEventKind::Complete => {
                let duration_ms = self
                    .tool_started_ms
                    .remove(call_id.as_str())
                    .map(|started_ms| event.occurred_ms.saturating_sub(started_ms))
                    .unwrap_or_default();
                self.tool_output.remove(call_id.as_str());
                let output = tool_payload_text(&event.payload, "output").unwrap_or_default();
                match event.status {
                    GatewayChatToolStatus::Completed => events.push(V3Event::ToolCompleted {
                        turn_id: turn_id.to_string(),
                        call_id,
                        output,
                        exit_code: tool_exit_code(&event.payload).unwrap_or_default(),
                        duration_ms,
                    }),
                    GatewayChatToolStatus::Cancelled => events.push(V3Event::ToolCancelled {
                        turn_id: turn_id.to_string(),
                        call_id,
                        reason: tool_error(&event.payload, output, "tool cancelled"),
                        duration_ms,
                    }),
                    GatewayChatToolStatus::Failed | GatewayChatToolStatus::Denied => {
                        events.push(V3Event::ToolFailed {
                            turn_id: turn_id.to_string(),
                            call_id,
                            message: tool_error(&event.payload, output, "tool failed"),
                            duration_ms,
                        });
                    }
                    GatewayChatToolStatus::Running
                    | GatewayChatToolStatus::Waiting
                    | GatewayChatToolStatus::Other(_) => events.push(V3Event::ToolFailed {
                        turn_id: turn_id.to_string(),
                        call_id,
                        message: tool_error(&event.payload, output, "tool ended without terminal status"),
                        duration_ms,
                    }),
                }
            }
        }
    }
}

fn tool_command(payload: &serde_json::Value) -> String {
    tool_payload_text(payload, "input")
        .or_else(|| tool_payload_text(payload, "title"))
        .unwrap_or_default()
}

fn tool_payload_text(payload: &serde_json::Value, key: &str) -> Option<String> {
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

fn tool_exit_code(payload: &serde_json::Value) -> Option<i32> {
    payload
        .get("exit_code")
        .or_else(|| payload.get("exitCode"))
        .and_then(serde_json::Value::as_i64)
        .and_then(|code| i32::try_from(code).ok())
}

fn tool_error(payload: &serde_json::Value, output: String, fallback: &str) -> String {
    tool_payload_text(payload, "error")
        .or_else(|| (!output.is_empty()).then_some(output))
        .unwrap_or_else(|| fallback.to_string())
}

fn non_negative_tokens(tokens: i64) -> u64 {
    u64::try_from(tokens).unwrap_or_default()
}

fn permission_display(state: &TuiState) -> String {
    let mode = match state.session_options.permission_mode {
        PermissionMode::Default => "Default",
        PermissionMode::Plan => "Plan",
        PermissionMode::AcceptEdits => "Accept edits",
        PermissionMode::Bypass => "Bypass",
    };
    let access = if state.session_options.full_access_enabled() {
        "full access"
    } else {
        "workspace write"
    };
    format!("{mode} · {access}")
}

fn controller_key(
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

fn permission_command(
    state: &TuiState,
    request_id: &str,
    decision: V3ApprovalDecision,
) -> Result<TuiControllerCommand, String> {
    let Some(UiOverlay::Permissions(overlay)) = state.overlays.active() else {
        return Err("no permission approval is active".to_string());
    };
    let request =
        overlay.queue.current().ok_or_else(|| "no pending permission request".to_string())?;
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
    Ok(TuiControllerCommand::Overlay(TuiOverlayCommand::PermissionDecision(decision)))
}

fn question_command(
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

fn project_message_delta(
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

fn project_non_message_event(event: &UiRuntimeEvent, turn_id: &str, events: &mut Vec<V3Event>) {
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

fn v3_terminal_event(turn_id: String, terminal: UiRuntimeTerminalEvent) -> V3Event {
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
