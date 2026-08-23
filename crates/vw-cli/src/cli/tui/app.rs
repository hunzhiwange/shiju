//! Default host that wires the Gateway runtime to the active chat surface.

use crate::app::agent::config::Config;
use crate::cli::setup::CliSetup;
use anyhow::{Result, anyhow};
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use vw_gateway_client::{GatewayChatStreamEvent, GatewayChatStreamRequest};
use ratatui::text::Line;
use ratatui::backend::Backend;
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;
use vw_tui::{
    ChatWidget, ChatWidgetConfig, CustomTerminal, TerminalMode, TranscriptPager,
    TranscriptReflowState, V3Action, V3Attachment, V3DisplayEvent, V3Event, V3StepStatus,
    insert_history_lines, run_terminal,
};
use vw_gateway_client::GatewayClient;
use serde_json::json;
use vw_gateway_client::vw_api_types::id::SessionId;
use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession, ChatSessionMeta};

use super::bridge::GatewayV3Bridge;
use super::controller::{TuiController, TuiControllerCommand};
use super::model::{PermissionMode, PromptSubmission, ReasoningEffort, UiMessage, UiStepState};
use super::runtime::gateway::{GatewayUiRuntime, normalize_optional_str_ref};
use super::state::TuiState;

type V3Terminal = CustomTerminal<CrosstermBackend<io::Stdout>>;

enum StreamMessage {
    Event(GatewayChatStreamEvent),
    Finished(Result<(), String>),
}

struct ActiveStream {
    receiver: Receiver<StreamMessage>,
    cancel: Arc<AtomicBool>,
}

/// Gateway-backed TUI composition root.
pub(crate) struct V3GatewayApp {
    runtime: GatewayUiRuntime,
    state: TuiState,
    controller: TuiController,
    widget: ChatWidget,
    bridge: GatewayV3Bridge,
    terminal_mode: TerminalMode,
    transcript_reflow: TranscriptReflowState,
}

impl V3GatewayApp {
    pub(crate) fn new(runtime: GatewayUiRuntime, mut state: TuiState) -> Self {
        if state.status.model_name.is_none() {
            state.status.model_name = state.session_options.model.clone();
        }
        Self::new_with_terminal_mode(runtime, state, TerminalMode::Inline)
    }

    pub(crate) fn new_with_terminal_mode(
        runtime: GatewayUiRuntime,
        state: TuiState,
        terminal_mode: TerminalMode,
    ) -> Self {
        let display_data = GatewayV3Bridge::session_display_data(&runtime, &state);
        let mut widget = ChatWidget::new(ChatWidgetConfig::default());
        widget.push_session_header(display_data);
        replay_session_history(&mut widget, &state);
        Self {
            runtime,
            state,
            controller: TuiController,
            widget,
            bridge: GatewayV3Bridge::default(),
            terminal_mode,
            transcript_reflow: TranscriptReflowState::default(),
        }
    }

    pub(crate) fn accept_gateway_event(&mut self, event: GatewayChatStreamEvent) -> Vec<V3Event> {
        let previous_display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        let events = self.bridge.adapt_gateway_event(&mut self.state, event);
        for event in events.iter().cloned() {
            self.widget.handle_v3_event(event);
        }
        self.sync_runtime_session_seed();
        let display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        if display != previous_display {
            self.widget.set_session_header(display);
        }
        events
    }

    pub(crate) fn persist_session_snapshot(&mut self) -> Result<(), String> {
        self.sync_runtime_session_seed();
        self.runtime.session_ui_save_blocking(&self.state.to_chat_session())
    }

    pub(crate) fn handle_action(
        &mut self,
        action: V3Action,
    ) -> Result<TuiControllerCommand, String> {
        let command = self.bridge.controller_command(&mut self.state, &mut self.controller, action)?;
        if let TuiControllerCommand::SubmitPrompt(submission) = &command {
            self.widget.push_history(vw_tui::V3DisplayEvent::User {
                text: submission.text.clone(),
                attachments: Vec::new(),
            });
        }
        Ok(command)
    }

    pub(crate) fn request_for_command(
        &self,
        command: &TuiControllerCommand,
    ) -> Option<GatewayChatStreamRequest> {
        GatewayV3Bridge::request_for_command(&self.state, command)
    }

    pub(crate) fn runtime(&self) -> &GatewayUiRuntime {
        &self.runtime
    }

    pub(crate) fn state(&self) -> &TuiState {
        &self.state
    }

    pub(crate) fn state_mut(&mut self) -> &mut TuiState {
        &mut self.state
    }

    pub(crate) fn widget(&self) -> &ChatWidget {
        &self.widget
    }

    pub(crate) fn widget_mut(&mut self) -> &mut ChatWidget {
        &mut self.widget
    }

    pub(crate) fn terminal_mode(&self) -> TerminalMode {
        self.terminal_mode
    }

    /// Builds the full-record pager from the same committed/active cells shown in the chat view.
    pub(crate) fn transcript_pager(&self, width: u16) -> TranscriptPager {
        TranscriptPager::new(self.transcript_lines(width, true))
    }

    /// Supplies width-aware rows for inline history insertion and resize replay.
    pub(crate) fn transcript_lines(&self, width: u16, include_active: bool) -> Vec<Line<'static>> {
        self.widget.transcript_lines(width, include_active)
    }

    /// Inserts only committed cells into native terminal scrollback.
    pub(crate) fn insert_committed_history<B>(
        &self,
        terminal: &mut CustomTerminal<B>,
        width: u16,
    ) -> io::Result<()>
    where
        B: Backend<Error = io::Error> + Write,
    {
        insert_history_lines(terminal, self.transcript_lines(width, false))
    }

    /// Returns source-backed transcript rows when a width change requires scrollback replay.
    pub(crate) fn resize_reflow_lines(&mut self, width: u16) -> Option<Vec<Line<'static>>> {
        let change = self.transcript_reflow.note_width(width);
        if change.changed {
            self.transcript_reflow.mark_reflowed_width(width);
            Some(self.transcript_lines(width, true))
        } else {
            None
        }
    }

    fn sync_runtime_session_seed(&mut self) {
        let title = (!self.state.session.title.trim().is_empty())
            .then(|| self.state.session.title.clone());
        self.runtime.bind_session_seed(
            self.state.session.session_id.clone(),
            self.state.session.scope.clone(),
            title,
        );
    }
}

/// Runs the active TUI surface as the CLI interactive entry.
pub(crate) fn run_tui(config: &Config, setup: &CliSetup) -> Result<()> {
    let mut runtime = GatewayUiRuntime::for_workspace(config).map_err(|error| anyhow!(error))?;
    runtime.ensure_local_gateway_ready_blocking().map_err(|error| anyhow!(error))?;

    let mut state = restore_or_create_session_state(&mut runtime).map_err(|error| anyhow!(error))?;
    state.session_options.model = Some(setup.model_name.clone());
    state.status.model_name = Some(setup.model_name.clone());
    let app = V3GatewayApp::new(runtime, state);
    let terminal_mode = app.terminal_mode();

    run_terminal(terminal_mode, || run_event_loop(app)).map_err(Into::into)
}

pub(crate) fn restore_or_create_session_state(
    runtime: &mut GatewayUiRuntime,
) -> Result<TuiState, String> {
    let previews = runtime.session_ui_previews_blocking()?;
    let scope = runtime
        .scope()
        .map(ToOwned::to_owned)
        .or(runtime.session_scope_get_blocking()?);
    let snapshot = match select_restore_session_id(runtime.session_id(), &previews) {
        Some(session_id) => runtime
            .session_ui_load_any_blocking(Some(session_id.as_str()))?
            .or_else(|| {
                previews
                    .iter()
                    .find(|preview| preview.id == session_id)
                    .map(snapshot_from_preview)
            })
            .unwrap_or_else(|| empty_session_snapshot(session_id.as_str())),
        None => {
            let title = default_session_title(runtime.directory());
            let created = runtime.session_create_blocking(Some(title.as_str()))?;
            let mut snapshot = empty_session_snapshot(created.id.as_str());
            snapshot.title = created.title;
            snapshot
        }
    };

    let mut state = TuiState::from_chat_session(&snapshot);
    state.session.scope = scope;
    state.project.workspace_root = Some(runtime.directory().to_path_buf());
    let title = (!state.session.title.trim().is_empty()).then(|| state.session.title.clone());
    runtime.bind_session_seed(state.session.session_id.clone(), state.session.scope.clone(), title);
    Ok(state)
}

fn latest_session_preview(previews: &[ChatSessionMeta]) -> Option<&ChatSessionMeta> {
    previews.iter().max_by(|left, right| {
        left.updated_ms
            .cmp(&right.updated_ms)
            .then(left.message_count.cmp(&right.message_count))
            .then(left.call_count.cmp(&right.call_count))
            .then(left.id.cmp(&right.id))
    })
}

fn snapshot_from_preview(preview: &ChatSessionMeta) -> ChatSession {
    ChatSession {
        id: preview.id.clone(),
        title: preview.title.clone(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: preview.updated_ms,
        updated_ms: preview.updated_ms,
    }
}

fn empty_session_snapshot(session_id: &str) -> ChatSession {
    ChatSession {
        id: session_id.to_string(),
        title: String::new(),
        messages: Vec::new(),
        message_ids: Vec::new(),
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 0,
        updated_ms: 0,
    }
}

fn select_restore_session_id(
    explicit_session_id: Option<&str>,
    previews: &[ChatSessionMeta],
) -> Option<String> {
    normalize_optional_str_ref(explicit_session_id)
        .map(ToOwned::to_owned)
        .or_else(|| latest_session_preview(previews).map(|preview| preview.id.clone()))
}

fn default_session_title(directory: &Path) -> String {
    directory
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(|name| format!("TUI {}", name.trim()))
        .unwrap_or_else(|| "TUI 会话".to_string())
}

pub(crate) fn gateway_stream_request_from_state(
    state: &TuiState,
    submission: &PromptSubmission,
) -> GatewayChatStreamRequest {
    let snapshot = state.to_chat_session();
    let acp_agent = submission.execution.acp_backend.clone();
    let full_access_enabled = submission.execution.full_access_enabled;
    let subagent_mode_enabled = submission.execution.subagent_mode_enabled;
    let mut allowed_tools = (!full_access_enabled
        && state.session_options.permission_mode == PermissionMode::Plan)
        .then(|| vec!["todowrite".to_string()]);
    if subagent_mode_enabled {
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
            let mut options = json!({
                "acp_test": true,
                "acp_agent": acp_agent,
                "cwd": cwd,
                "acp_force_new_session": submission.execution.acp_force_new_session,
                "acp_permission_mode": if full_access_enabled { "approve-all" } else { "approve-reads" },
            });
            if full_access_enabled {
                options["full_access"] = json!(true);
            }
            options
        })
        .or_else(|| full_access_enabled.then(|| json!({"full_access": true})));
    if subagent_mode_enabled {
        let options = options.get_or_insert_with(|| json!({}));
        options["allowed_tools"] = json!(allowed_tools);
    }
    let system = subagent_mode_enabled.then(|| vec![SUBAGENT_MODE_SYSTEM_PROMPT.to_string()]);
    let acp_allowed_tools = acp_agent.is_some().then(|| allowed_tools.clone()).flatten();

    GatewayChatStreamRequest {
        session_id: normalize_optional_str_ref(submission.session_id.as_deref())
            .map(SessionId::from),
        messages: snapshot.messages.iter().map(gateway_message_from_chat_message).collect(),
        system,
        model: submission.model.clone(),
        reasoning_effort: Some(match state.session_options.effort {
            ReasoningEffort::Minimal => vw_gateway_client::vw_api_types::ReasoningEffortDto::None,
            ReasoningEffort::Low => vw_gateway_client::vw_api_types::ReasoningEffortDto::Low,
            ReasoningEffort::Medium => vw_gateway_client::vw_api_types::ReasoningEffortDto::Medium,
            ReasoningEffort::High => vw_gateway_client::vw_api_types::ReasoningEffortDto::High,
            ReasoningEffort::XHigh => vw_gateway_client::vw_api_types::ReasoningEffortDto::Xhigh,
        }),
        agent: submission.execution.delegate_agent.clone(),
        allowed_tools,
        acp_agent,
        acp_allowed_tools,
        options,
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
        ChatRole::Tool => "tool",
    }
}

fn replay_session_history(widget: &mut ChatWidget, state: &TuiState) {
    for message in &state.messages {
        widget.push_history(display_event_from_message(message));
    }
}

fn display_event_from_message(message: &UiMessage) -> V3DisplayEvent {
    match message {
        UiMessage::User(message) => {
            V3DisplayEvent::User { text: message.text.clone(), attachments: Vec::new() }
        }
        UiMessage::Assistant(message) => {
            V3DisplayEvent::Assistant { markdown: message.text.clone() }
        }
        UiMessage::ToolCall(message) => V3DisplayEvent::ToolCall {
            call_id: message
                .call_id
                .clone()
                .unwrap_or_else(|| message.base.id.as_str().to_string()),
            tool: message.tool_name.clone(),
            command: message
                .arguments
                .clone()
                .or_else(|| message.summary.clone())
                .unwrap_or_default(),
        },
        UiMessage::ToolResult(message) => V3DisplayEvent::ToolResult {
            call_id: message
                .call_id
                .clone()
                .unwrap_or_else(|| message.base.id.as_str().to_string()),
            output: message.content.clone(),
            exit_code: i32::from(message.is_error),
            duration_ms: 0,
        },
        UiMessage::Thinking(message) => {
            V3DisplayEvent::Thinking { text: message.content.clone() }
        }
        UiMessage::Step(message) => V3DisplayEvent::Step {
            title: message
                .finish_reason
                .clone()
                .unwrap_or_else(|| format!("step {}", message.step_index)),
            status: match message.state {
                UiStepState::Pending => V3StepStatus::Pending,
                UiStepState::Running => V3StepStatus::InProgress,
                UiStepState::Complete => V3StepStatus::Completed,
                UiStepState::Cancelled | UiStepState::Failed => V3StepStatus::Failed,
            },
        },
        UiMessage::System(message) => V3DisplayEvent::System { text: message.text.clone() },
        UiMessage::Error(message) => V3DisplayEvent::Error { message: message.message.clone() },
        UiMessage::Attachment(message) => V3DisplayEvent::Attachment {
            attachment: V3Attachment {
                label: message.name.clone(),
                path: (!message.uri.trim().is_empty()).then(|| message.uri.clone()),
            },
        },
        UiMessage::Progress(message) => V3DisplayEvent::Progress {
            label: message.label.clone(),
            detail: match (message.current, message.total) {
                (Some(current), Some(total)) => Some(format!("{current}/{total}")),
                (Some(current), None) => Some(current.to_string()),
                _ => None,
            },
        },
        UiMessage::CompactBoundary(message) => {
            V3DisplayEvent::CompactBoundary { summary: message.summary.clone() }
        }
        UiMessage::Unknown(message) => V3DisplayEvent::Unknown {
            type_name: message.external_kind.clone(),
            detail: message.summary.clone(),
        },
    }
}

fn run_event_loop(mut app: V3GatewayApp) -> io::Result<()> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = V3Terminal::with_options(backend)?;
    let mut active_stream = None;

    loop {
        drain_stream(&mut app, &mut active_stream)?;
        draw(&app, &mut terminal)?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        let action = match event::read()? {
            Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
                app.widget_mut().handle_key_event(key)
            }
            Event::Paste(text) => app.widget_mut().handle_paste(text),
            Event::Mouse(mouse) => app.widget_mut().handle_mouse_event(mouse),
            Event::Resize(width, _) => {
                let _ = app.resize_reflow_lines(width);
                continue;
            }
            _ => continue,
        };

        if matches!(action, V3Action::Cancel) && active_stream.is_none() {
            return Ok(());
        }

        let command = app.handle_action(action).map_err(io::Error::other)?;
        match command {
            TuiControllerCommand::Continue => {}
            TuiControllerCommand::Quit => return Ok(()),
            TuiControllerCommand::CancelActiveSubmission => {
                if let Some(stream) = &active_stream {
                    stream.cancel.store(true, Ordering::Release);
                } else {
                    return Ok(());
                }
            }
            TuiControllerCommand::SubmitPrompt(_) if active_stream.is_some() => {
                return Err(io::Error::other(
                    "TUI received a second submission while a gateway stream is active",
                ));
            }
            command @ TuiControllerCommand::SubmitPrompt(_) => {
                let request = app
                    .request_for_command(&command)
                    .ok_or_else(|| io::Error::other("TUI submission did not produce a request"))?;
                active_stream = Some(spawn_stream(app.runtime().clone(), request));
            }
            unsupported => {
                return Err(io::Error::other(format!(
                    "TUI runner does not support controller command {unsupported:?}"
                )));
            }
        }
    }
}

fn spawn_stream(runtime: GatewayUiRuntime, request: GatewayChatStreamRequest) -> ActiveStream {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_cancel = Arc::clone(&cancel);
    std::thread::spawn(move || {
        let directory = runtime.directory_value();
        let request = runtime.prepare_stream_request(&request);
        let result = GatewayClient::stream_chat_blocking(
            runtime.endpoint(),
            directory.as_deref(),
            &request,
            |event| {
                !stream_cancel.load(Ordering::Acquire)
                    && sender.send(StreamMessage::Event(event)).is_ok()
            },
        );
        let _ = sender.send(StreamMessage::Finished(result));
    });
    ActiveStream { receiver, cancel }
}

fn drain_stream(
    app: &mut V3GatewayApp,
    active_stream: &mut Option<ActiveStream>,
) -> io::Result<()> {
    let Some(stream) = active_stream else {
        return Ok(());
    };
    let mut finished = false;

    loop {
        match stream.receiver.try_recv() {
            Ok(StreamMessage::Event(event)) => {
                app.accept_gateway_event(event);
            }
            Ok(StreamMessage::Finished(result)) => {
                result.map_err(io::Error::other)?;
                finished = true;
                break;
            }
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                finished = true;
                break;
            }
        }
    }

    if finished {
        if let Err(error) = app.persist_session_snapshot() {
            app.widget_mut().push_history(V3DisplayEvent::Error {
                message: format!("session save failed: {error}"),
            });
        }
        *active_stream = None;
    }
    Ok(())
}

fn draw(app: &V3GatewayApp, terminal: &mut V3Terminal) -> io::Result<()> {
    let (width, height) = crossterm::terminal::size()?;
    let y = match app.terminal_mode() {
        TerminalMode::Inline => terminal.viewport_area.y.min(height.saturating_sub(1)),
        TerminalMode::Alternate => 0,
    };
    terminal.set_viewport_area(Rect::new(0, y, width, height.saturating_sub(y)));
    terminal.draw(|frame| {
        let area = frame.area();
        app.widget().render(area, frame.buffer_mut());
        frame.set_cursor_style(app.widget().cursor_style(area));
        if let Some(position) = app.widget().cursor_pos(area) {
            frame.set_cursor_position(position);
        }
    })
}
