//! Gateway 事件管线与活动 UI 契约之间的显式适配器。
//!
//! 本模块是两套事件语汇的唯一转换边界：先用 Gateway 事件更新 `TuiState`，
//! 再投影为产品中立的 `V3Event`。未支持的附件或失配审批会显式失败。

use self::projection::{v3_terminal_event, tool_payload_text};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use vw_api_tools::{ToolResultContentDto, ToolResultDto};
use vw_gateway_client::{
    GatewayChatStreamEvent, GatewayChatStreamRequest, GatewayChatToolEvent,
    GatewayChatToolEventKind, GatewayChatToolStatus, normalize_chat_tool_event,
};
use vw_tui::{
    V3Action, V3ApprovalDecision, V3Event, V3FileChange, V3FileDiff, V3FileDiffEntry,
    V3SessionDisplayData, V3TokenUsage, V3UserInputAnswer,
};

use super::request::gateway_stream_request_from_state;
use super::commands::{route_selection, route_slash_submission};
use super::controller::{
    TuiController, TuiControllerCommand, TuiControllerEvent, TuiOverlayCommand,
    build_attachment_submission,
};
use cli_interaction_model::attachment::prepare_prompt_attachments;
use cli_interaction_model::permission::PermissionDecision;
use cli_interaction_model::{PermissionMode, UiMessage, UiOverlay};
use cli_gateway_transport::gateway::GatewayUiRuntime;
use cli_interaction_model::{UiRuntimeEvent, UiRuntimeTerminalEvent};
use cli_gateway_transport::stream_adapter::adapt_gateway_stream_event;
use cli_interaction_state::selectors::select_status_summary;
use cli_interaction_state::{TuiAction, TuiState, apply_runtime_event, reduce_tui_state};

/// 为产品中立事件语汇分配 turn ID，并跟踪工具输出增量的有状态适配器。
#[derive(Debug, Default)]
pub(crate) struct GatewayV3Bridge {
    next_turn: u64,
    active_turn_id: Option<String>,
    tool_started_ms: std::collections::HashMap<String, u64>,
    tool_output: std::collections::HashMap<String, String>,
    interaction_refresh_requested: bool,
}

impl GatewayV3Bridge {
    /// 将本地运行时状态投影为只读的欢迎页、会话与状态契约。
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
        let session_title =
            (!session_title.is_empty()).then(|| session_title.to_string()).or_else(|| {
                state
                    .session
                    .first_response_preview
                    .as_deref()
                    .and_then(session_text::initial_title_from_content)
                    .or_else(|| {
                        state.messages.iter().find_map(|message| match message {
                            UiMessage::User(user) => {
                                session_text::initial_title_from_content(
                                    &user.text,
                                )
                            }
                            _ => None,
                        })
                    })
                    .map(str::to_owned)
            });
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

    /// 运行 Gateway 适配与状态管线，再仅投影本次产生的 UI 增量。
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
        self.adapt_runtime_event_inner(state, runtime_event, tool_event)
    }

    /// 将已标准化的 plan-run 事件送入同一 reducer 与活动 transcript 生命周期。
    pub(crate) fn adapt_runtime_event(
        &mut self,
        state: &mut TuiState,
        event: UiRuntimeEvent,
    ) -> Vec<V3Event> {
        self.adapt_runtime_event_inner(state, event, None)
    }

    fn adapt_runtime_event_inner(
        &mut self,
        state: &mut TuiState,
        runtime_event: UiRuntimeEvent,
        tool_event: Option<GatewayChatToolEvent>,
    ) -> Vec<V3Event> {
        let terminal = match &runtime_event {
            UiRuntimeEvent::Terminal(terminal) => Some(terminal.clone()),
            _ => None,
        };
        let before = state.messages.clone();
        let starts_turn = self.active_turn_id.is_none();
        let turn_id = self.active_turn_id();
        if apply_runtime_event(state, runtime_event.clone()) {
            self.interaction_refresh_requested = true;
        }

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
            // turn 终态后必须清空工具跟踪，避免下一 turn 复用 call ID 时混入旧输出。
            self.active_turn_id = None;
            self.tool_started_ms.clear();
            self.tool_output.clear();
        }
        events
    }

    /// 取出一次由当前会话 task 事件触发的交互刷新请求。
    pub(crate) fn take_interaction_refresh_requested(&mut self) -> bool {
        std::mem::take(&mut self.interaction_refresh_requested)
    }

    /// 将 UI 操作路由为控制器命令、对话框结果或提交快照。
    pub(crate) fn controller_command(
        &mut self,
        state: &mut TuiState,
        controller: &mut TuiController,
        action: V3Action,
    ) -> Result<TuiControllerCommand, String> {
        match action {
            V3Action::None
            | V3Action::Attach(_)
            | V3Action::RemoveAttachment { .. }
            | V3Action::ToggleShortcuts => Ok(TuiControllerCommand::Continue),
            V3Action::FileSearchQueryChanged { query } => {
                Ok(TuiControllerCommand::FileSearchQueryChanged { query })
            }
            V3Action::Select { surface, item_id } => {
                route_selection(surface.as_str(), item_id.as_str())
                    .map(TuiControllerCommand::Command)
            }
            V3Action::Submit { text, attachments } => {
                if attachments.is_empty()
                    && let Some(route) = route_slash_submission(text.as_str())?
                {
                    return Ok(TuiControllerCommand::Command(route));
                }
                if text.trim().is_empty() && attachments.is_empty() {
                    return Ok(TuiControllerCommand::Continue);
                }
                let prepared = prepare_prompt_attachments(
                    state.project.workspace_root.as_deref(),
                    &attachments,
                )?;
                reduce_tui_state(state, TuiAction::PromptValueSet(text));
                let submission = build_attachment_submission(state, state.prompt.value.as_str())
                    .with_attachments(prepared);
                reduce_tui_state(state, TuiAction::PromptSubmissionStarted(submission.clone()));
                Ok(TuiControllerCommand::SubmitPrompt(submission))
            }
            V3Action::Queue { text, attachments } => {
                prepare_prompt_attachments(state.project.workspace_root.as_deref(), &attachments)?;
                Ok(TuiControllerCommand::QueuePrompt { text, attachments })
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
            V3Action::ShowTranscript => Ok(TuiControllerCommand::OpenTranscript),
            V3Action::OpenExternalEditor => Ok(TuiControllerCommand::OpenExternalEditor),
            V3Action::ResolveApproval { request_id, decision } => {
                if matches!(state.overlays.active(), Some(UiOverlay::Confirm(_))) {
                    confirmation_command(state, request_id.as_str(), decision)
                } else {
                    permission_command(state, request_id.as_str(), decision)
                }
            }
            V3Action::AnswerUserInput { request_id, answers } => {
                question_command(state, request_id.as_str(), answers)
            }
        }
    }

    /// 仅对提交命令委托共享构造器生成 Gateway 请求。
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
                self.ensure_tool_started(&event, turn_id, events);
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
                self.ensure_tool_started(&event, turn_id, events);
                let duration_ms = self
                    .tool_started_ms
                    .remove(call_id.as_str())
                    .map(|started_ms| event.occurred_ms.saturating_sub(started_ms))
                    .unwrap_or_default();
                self.tool_output.remove(call_id.as_str());
                let output = tool_payload_text(&event.payload, "output").unwrap_or_default();
                let file_diff = matches!(event.status, GatewayChatToolStatus::Completed)
                    .then(|| structured_patch_diff(&event.payload))
                    .flatten();
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
                        message: tool_error(
                            &event.payload,
                            output,
                            "tool ended without terminal status",
                        ),
                        duration_ms,
                    }),
                }
                if let Some(diff) = file_diff {
                    events.push(V3Event::FileDiff { turn_id: turn_id.to_string(), diff });
                }
            }
        }
    }

    fn ensure_tool_started(
        &mut self,
        event: &GatewayChatToolEvent,
        turn_id: &str,
        events: &mut Vec<V3Event>,
    ) {
        match self.tool_started_ms.entry(event.call_id.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => return,
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(event.occurred_ms);
            }
        }
        events.push(V3Event::ToolStarted {
            turn_id: turn_id.to_string(),
            call_id: event.call_id.clone(),
            tool: event.name.clone(),
            command: tool_command(&event.payload),
        });
    }
}

pub(super) use projection::structured_patch_diff;
use projection::tool_command;
use projection::tool_exit_code;
use projection::tool_error;
use projection::non_negative_tokens;
use projection::permission_display;
use projection::controller_key;
use projection::permission_command;
use projection::confirmation_command;
use projection::question_command;
use projection::project_message_delta;
use projection::project_non_message_event;

mod projection;
