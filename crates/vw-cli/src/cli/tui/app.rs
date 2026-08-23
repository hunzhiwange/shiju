//! 将 Gateway 会话运行时、TUI 状态与活动聊天界面连接起来的组装根。
//!
//! 本模块负责终端生命周期、会话恢复、流式请求启动和事件循环；
//! Gateway 事件到界面事件的语义转换留在 `bridge`，不在主循环中重复。

use crate::app::agent::config::Config;
use crate::cli::setup::CliSetup;
use anyhow::{Result, anyhow};
use crossterm::event::{self, Event, KeyEventKind};
use dialoguer::Editor;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::backend::Backend;
use std::io;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use vw_tui::{
    ChatWidget, ChatWidgetConfig, CustomTerminal, TerminalMode, TranscriptPager,
    TranscriptReflowState, V3Action, V3Attachment, V3DisplayEvent, V3Event, V3StepStatus,
    insert_history_lines, run_terminal,
};
use vw_gateway_client::{
    GatewayChatStreamEvent, GatewayChatStreamRequest, GatewayStreamAttemptResult,
    PendingPermissionReplyDto,
};
use serde_json::json;
use vw_gateway_client::vw_api_types::id::SessionId;
use vw_shared::session::ui_types::{ChatMessage, ChatRole, ChatSession, ChatSessionMeta};

use super::bridge::GatewayV3Bridge;
use super::controller::{TuiController, TuiControllerCommand, TuiOverlayCommand};
use super::model::{
    PermissionMode, PromptSubmission, ReasoningEffort, UiMessage, UiOverlay, UiPermissionOverlay,
    UiStepState, UiTodoOverlay,
};
use super::model::permission::{PermissionDecision, PermissionQueue, PermissionRequest};
use super::runtime::gateway::{GatewayUiRuntime, normalize_optional_str_ref};
use super::runtime::interaction::InteractionSnapshot;
use super::state::{TuiAction, TuiState, reduce_tui_state};

type V3Terminal = CustomTerminal<CrosstermBackend<io::Stdout>>;

enum StreamMessage {
    Event(GatewayChatStreamEvent),
    Finished(Result<GatewayStreamAttemptResult, String>),
}

struct ActiveStream {
    request_id: String,
    receiver: Receiver<StreamMessage>,
    cancel: Arc<AtomicBool>,
}

/// 外部编辑器调用边界；输入当前草稿，返回替换文本或用户取消。
pub(crate) type EditorAction = dyn Fn(&str) -> Result<Option<String>, String> + Send + Sync;

enum HostMessage {
    EditorFinished(Result<Option<String>, String>),
    StopFinished { request_id: String, result: Result<bool, String> },
    InteractionsRefreshed(Result<InteractionSnapshot, String>),
    PermissionReplyFinished { request_id: String, result: Result<(), String> },
    QuestionReplyFinished { request_id: String, result: Result<(), String> },
}

/// 事件循环执行一次操作后的继续或退出指示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerControl {
    Continue,
    Quit,
}

/// 在渲染 crate 之外统一管理活跃流、transcript 和本地副作生命周期。
pub(crate) struct ActiveRunner {
    active_stream: Option<ActiveStream>,
    transcript_pager: Option<TranscriptPager>,
    editor_in_flight: bool,
    interaction_in_flight: bool,
    editor: Arc<EditorAction>,
    host_sender: mpsc::Sender<HostMessage>,
    host_receiver: Receiver<HostMessage>,
}

impl ActiveRunner {
    fn new() -> Self {
        Self::new_with_editor(Arc::new(|draft| {
            Editor::new().edit(draft).map_err(|error| error.to_string())
        }))
    }

    /// 用可注入的编辑器动作创建 runner，便于将外部进程与事件循环解耦。
    pub(crate) fn new_with_editor(editor: Arc<EditorAction>) -> Self {
        let (host_sender, host_receiver) = mpsc::channel();
        Self {
            active_stream: None,
            transcript_pager: None,
            editor_in_flight: false,
            interaction_in_flight: false,
            editor,
            host_sender,
            host_receiver,
        }
    }

    /// 将界面操作转为宿主命令并执行，错误会投影为界面消息而非中止循环。
    pub(crate) fn execute_action(
        &mut self,
        app: &mut V3GatewayApp,
        action: V3Action,
    ) -> RunnerControl {
        let command = match app.handle_action(action) {
            Ok(command) => command,
            Err(error) => {
                push_runner_error(app, error);
                app.show_next_interaction();
                return RunnerControl::Continue;
            }
        };
        self.execute_command(app, command)
    }

    fn execute_command(
        &mut self,
        app: &mut V3GatewayApp,
        command: TuiControllerCommand,
    ) -> RunnerControl {
        match command {
            TuiControllerCommand::Continue => RunnerControl::Continue,
            TuiControllerCommand::Quit => RunnerControl::Quit,
            TuiControllerCommand::CancelActiveSubmission => {
                if let Some(stream) = self.active_stream.as_ref() {
                    stream.cancel.store(true, Ordering::Release);
                    app.accept_gateway_event(GatewayChatStreamEvent::Error(
                        "cancelled by user".to_string(),
                    ));
                    spawn_stop(
                        app.runtime().clone(),
                        stream.request_id.clone(),
                        self.host_sender.clone(),
                    );
                    RunnerControl::Continue
                } else {
                    RunnerControl::Quit
                }
            }
            TuiControllerCommand::QueuePrompt(text) => {
                app.widget_mut().queue_input(text, Vec::new());
                self.start_next_queued_prompt(app);
                RunnerControl::Continue
            }
            TuiControllerCommand::OpenTranscript => {
                let width = crossterm::terminal::size().map(|(width, _)| width).unwrap_or(80);
                self.transcript_pager = Some(app.transcript_pager(width));
                RunnerControl::Continue
            }
            TuiControllerCommand::OpenExternalEditor => {
                self.launch_editor(app.widget().draft().to_string());
                RunnerControl::Continue
            }
            TuiControllerCommand::Overlay(command) => {
                self.execute_overlay_command(app, command);
                RunnerControl::Continue
            }
            command @ TuiControllerCommand::SubmitPrompt(_) => {
                if self.active_stream.is_some() {
                    push_runner_error(
                        app,
                        "a gateway stream is already active; queue the follow-up instead",
                    );
                    return RunnerControl::Continue;
                }
                let Some(request) = app.request_for_command(&command) else {
                    push_runner_error(app, "TUI submission did not produce a request");
                    return RunnerControl::Continue;
                };
                let request_id = next_stream_request_id();
                self.active_stream = Some(spawn_stream(
                    app.runtime().clone(),
                    request,
                    request_id,
                ));
                RunnerControl::Continue
            }
        }
    }

    fn execute_overlay_command(&mut self, app: &mut V3GatewayApp, command: TuiOverlayCommand) {
        if self.interaction_in_flight {
            app.record_interaction_error("an interaction request is already in flight");
            app.show_next_interaction();
            return;
        }
        self.interaction_in_flight = true;
        match command {
            TuiOverlayCommand::PermissionDecision { request_id, decision } => {
                let (reply, message) = match decision {
                    PermissionDecision::Once => (PendingPermissionReplyDto::Once, None),
                    PermissionDecision::Always => (PendingPermissionReplyDto::Always, None),
                    PermissionDecision::Reject { reason } => {
                        (PendingPermissionReplyDto::Reject, reason)
                    }
                };
                spawn_permission_reply(
                    app.runtime().clone(),
                    request_id,
                    reply,
                    message,
                    self.host_sender.clone(),
                );
            }
            TuiOverlayCommand::QuestionSubmitted(question) => {
                spawn_question_reply(
                    app.runtime().clone(),
                    question.request_id,
                    Some(question.answers),
                    self.host_sender.clone(),
                );
            }
            TuiOverlayCommand::QuestionRejected(question) => {
                spawn_question_reply(
                    app.runtime().clone(),
                    question.request_id,
                    None,
                    self.host_sender.clone(),
                );
            }
        }
    }

    fn launch_editor(&mut self, draft: String) {
        if self.editor_in_flight {
            return;
        }
        self.editor_in_flight = true;
        let editor = Arc::clone(&self.editor);
        let sender = self.host_sender.clone();
        std::thread::spawn(move || {
            let result = editor(draft.as_str());
            let _ = sender.send(HostMessage::EditorFinished(result));
        });
    }

    /// 非阻塞汇集本地副作与 Gateway 流消息，并在流结束后尝试启动排队输入。
    pub(crate) fn drain(&mut self, app: &mut V3GatewayApp) {
        if app.take_interaction_refresh_requested() {
            self.request_interaction_refresh(app);
        }
        self.drain_host_messages(app);
        self.drain_active_stream(app);
        if app.take_interaction_refresh_requested() {
            self.request_interaction_refresh(app);
        }
    }

    /// 在后台加载当前会话交互；并发事件会保留刷新标记供本轮结束后重试。
    pub(crate) fn request_interaction_refresh(&mut self, app: &mut V3GatewayApp) {
        if self.interaction_in_flight {
            app.request_interaction_refresh();
            return;
        }
        let Some(session_id) = app.state().session.session_id.clone() else {
            app.record_interaction_error("cannot refresh interactions without a session id");
            return;
        };
        self.interaction_in_flight = true;
        spawn_interaction_refresh(
            app.runtime().clone(),
            session_id,
            self.host_sender.clone(),
        );
    }

    fn drain_host_messages(&mut self, app: &mut V3GatewayApp) {
        loop {
            match self.host_receiver.try_recv() {
                Ok(HostMessage::EditorFinished(result)) => {
                    self.editor_in_flight = false;
                    match result {
                        Ok(Some(text)) => app.widget_mut().replace_draft(text.as_str()),
                        Ok(None) => {}
                        Err(error) => {
                            push_runner_error(app, format!("external editor failed: {error}"));
                        }
                    }
                }
                Ok(HostMessage::StopFinished { request_id, result }) => match result {
                    Ok(true) => {}
                    Ok(false) => push_runner_error(
                        app,
                        format!("gateway did not stop active stream {request_id}"),
                    ),
                    Err(error) => push_runner_error(
                        app,
                        format!("failed to stop active stream {request_id}: {error}"),
                    ),
                },
                Ok(HostMessage::InteractionsRefreshed(result)) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(snapshot) => app.apply_interaction_snapshot(snapshot),
                        Err(error) => app.record_interaction_error(format!(
                            "failed to refresh pending interactions: {error}"
                        )),
                    }
                }
                Ok(HostMessage::PermissionReplyFinished { request_id, result }) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(()) => app.complete_permission(request_id.as_str()),
                        Err(error) => {
                            app.record_interaction_error(format!(
                                "failed to reply to permission {request_id}: {error}"
                            ));
                            app.show_next_interaction();
                        }
                    }
                }
                Ok(HostMessage::QuestionReplyFinished { request_id, result }) => {
                    self.interaction_in_flight = false;
                    match result {
                        Ok(()) => app.complete_question(request_id.as_str()),
                        Err(error) => {
                            app.record_interaction_error(format!(
                                "failed to reply to question {request_id}: {error}"
                            ));
                            app.show_next_interaction();
                        }
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
    }

    fn drain_active_stream(&mut self, app: &mut V3GatewayApp) {
        let Some(stream) = self.active_stream.as_ref() else {
            self.start_next_queued_prompt(app);
            return;
        };
        let mut terminal = None;

        loop {
            match stream.receiver.try_recv() {
                Ok(StreamMessage::Event(event)) => {
                    app.accept_gateway_event(event);
                }
                Ok(StreamMessage::Finished(result)) => {
                    terminal = Some(result);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    terminal = Some(Err("gateway stream worker disconnected".to_string()));
                    break;
                }
            }
        }

        let Some(result) = terminal else {
            return;
        };
        match result {
            Ok(GatewayStreamAttemptResult::Terminated | GatewayStreamAttemptResult::CallerStopped) => {}
            Ok(GatewayStreamAttemptResult::Incomplete) => {
                push_runner_error(app, "gateway stream ended before a terminal event");
            }
            Err(error) => push_runner_error(app, format!("gateway stream failed: {error}")),
        }
        if let Err(error) = app.persist_session_snapshot() {
            push_runner_error(app, format!("session save failed: {error}"));
        }
        self.active_stream = None;
        self.start_next_queued_prompt(app);
    }

    fn start_next_queued_prompt(&mut self, app: &mut V3GatewayApp) {
        if self.active_stream.is_some() {
            return;
        }
        let Some((text, attachments)) = app.widget_mut().pop_queued_input() else {
            return;
        };
        let _ = self.execute_action(app, V3Action::Submit { text, attachments });
    }

    fn refresh_pager(&mut self, app: &V3GatewayApp, width: u16) {
        if let Some(pager) = self.transcript_pager.as_mut() {
            pager.replace_committed(app.transcript_lines(width, true));
        }
    }

    fn handle_pager_key(&mut self, key: crossterm::event::KeyEvent) {
        let Some(pager) = self.transcript_pager.as_mut() else {
            return;
        };
        pager.handle_key_event(key);
        if pager.is_done() {
            self.transcript_pager = None;
        }
    }

    #[cfg(test)]
    /// 返回当前是否有未结束的 Gateway 流。
    pub(crate) fn has_active_stream(&self) -> bool {
        self.active_stream.is_some()
    }

    #[cfg(test)]
    /// 返回活跃流的请求 ID，供停止与诊断路径关联。
    pub(crate) fn active_request_id(&self) -> Option<&str> {
        self.active_stream.as_ref().map(|stream| stream.request_id.as_str())
    }

    /// 返回完整 transcript 阅读器是否已打开。
    pub(crate) fn has_transcript_pager(&self) -> bool {
        self.transcript_pager.is_some()
    }

    /// 返回外部编辑器动作是否仍在执行。
    pub(crate) fn editor_in_flight(&self) -> bool {
        self.editor_in_flight
    }

    /// 返回交互刷新或回复工作线程是否尚未结束。
    pub(crate) fn interaction_in_flight(&self) -> bool {
        self.interaction_in_flight
    }
}

fn push_runner_error(app: &mut V3GatewayApp, message: impl Into<String>) {
    app.widget_mut().push_history(V3DisplayEvent::Error { message: message.into() });
}

/// Gateway 驱动的 TUI 组装根，持有运行时、状态、控制器与渲染部件。
pub(crate) struct V3GatewayApp {
    runtime: GatewayUiRuntime,
    state: TuiState,
    controller: TuiController,
    widget: ChatWidget,
    bridge: GatewayV3Bridge,
    terminal_mode: TerminalMode,
    transcript_reflow: TranscriptReflowState,
    interaction_refresh_requested: bool,
}

impl V3GatewayApp {
    /// 以内联终端模式创建应用，并用会话配置补齐缺失的模型名。
    pub(crate) fn new(runtime: GatewayUiRuntime, mut state: TuiState) -> Self {
        if state.status.model_name.is_none() {
            state.status.model_name = state.session_options.model.clone();
        }
        Self::new_with_terminal_mode(runtime, state, TerminalMode::Inline)
    }

    /// 按指定终端模式创建应用，同时回放已恢复的历史消息。
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
            interaction_refresh_requested: false,
        }
    }

    /// 接收一个 Gateway 流事件，原子地更新状态、聊天部件与会话头。
    pub(crate) fn accept_gateway_event(&mut self, event: GatewayChatStreamEvent) -> Vec<V3Event> {
        let previous_display = GatewayV3Bridge::session_display_data(&self.runtime, &self.state);
        let events = self.bridge.adapt_gateway_event(&mut self.state, event);
        if self.bridge.take_interaction_refresh_requested() {
            self.interaction_refresh_requested = true;
        }
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

    fn request_interaction_refresh(&mut self) {
        self.interaction_refresh_requested = true;
    }

    fn take_interaction_refresh_requested(&mut self) -> bool {
        std::mem::take(&mut self.interaction_refresh_requested)
    }

    fn apply_interaction_snapshot(&mut self, snapshot: InteractionSnapshot) {
        if self.state.session.session_id.as_deref() != Some(snapshot.session_id.as_str()) {
            return;
        }
        let mut queue = PermissionQueue::default();
        queue.replace(snapshot.permissions.into_iter().map(PermissionRequest::from_dto).collect());
        self.state
            .overlays
            .stack
            .retain(|overlay| !matches!(overlay, UiOverlay::Permissions(_)));
        if queue.len() > 0 {
            self.state.overlays.push(UiOverlay::Permissions(UiPermissionOverlay {
                queue,
                notice: None,
            }));
        }
        reduce_tui_state(
            &mut self.state,
            TuiAction::QuestionsReplaced(
                snapshot
                    .questions
                    .iter()
                    .map(super::model::UiQuestionOverlay::from_request)
                    .collect(),
            ),
        );
        reduce_tui_state(
            &mut self.state,
            TuiAction::TodoOverlayReplaced(Some(UiTodoOverlay::from_todos(
                Some(snapshot.session_id.as_str()),
                snapshot.todos.as_slice(),
            ))),
        );
        reduce_tui_state(&mut self.state, TuiAction::TaskSyncErrorSet(None));
        self.show_next_interaction();
    }

    fn complete_permission(&mut self, request_id: &str) {
        for overlay in &mut self.state.overlays.stack {
            let UiOverlay::Permissions(permission) = overlay else {
                continue;
            };
            if permission.queue.current().is_some_and(|request| request.id == request_id) {
                permission.queue.pop_current();
            }
        }
        self.state.overlays.stack.retain(|overlay| {
            !matches!(overlay, UiOverlay::Permissions(permission) if permission.queue.len() == 0)
        });
        reduce_tui_state(&mut self.state, TuiAction::TaskSyncErrorSet(None));
        self.show_next_interaction();
    }

    fn complete_question(&mut self, request_id: &str) {
        self.state
            .tasks
            .pending_questions
            .retain(|question| question.request_id != request_id);
        reduce_tui_state(&mut self.state, TuiAction::TaskSyncErrorSet(None));
        self.show_next_interaction();
    }

    fn record_interaction_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        reduce_tui_state(
            &mut self.state,
            TuiAction::TaskSyncErrorSet(Some(message.clone())),
        );
        push_runner_error(self, message);
    }

    /// 以权限优先、提问次之的顺序重建安全 overlay；没有请求时才清除。
    pub(crate) fn show_next_interaction(&mut self) {
        let permission = self.state.overlays.stack.iter().rev().find_map(|overlay| {
            let UiOverlay::Permissions(permission) = overlay else {
                return None;
            };
            permission.current_request()
        });
        if let Some(permission) = permission {
            self.widget.show_approval(permission);
        } else if let Some(question) = self.state.tasks.pending_questions.first() {
            self.widget.show_user_input(question.to_v3_request());
        } else {
            self.widget.clear_overlay();
        }
    }

    /// 将当前视图状态转换为会话快照并同步保存。
    pub(crate) fn persist_session_snapshot(&mut self) -> Result<(), String> {
        self.sync_runtime_session_seed();
        self.runtime.session_ui_save_blocking(&self.state.to_chat_session())
    }

    /// 将界面操作路由为控制器命令；提交成功时先追加本地用户历史。
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

    /// 仅为提交命令构造 Gateway 流式请求。
    pub(crate) fn request_for_command(
        &self,
        command: &TuiControllerCommand,
    ) -> Option<GatewayChatStreamRequest> {
        GatewayV3Bridge::request_for_command(&self.state, command)
    }

    /// 借用 Gateway 运行时，不转移其所有权。
    pub(crate) fn runtime(&self) -> &GatewayUiRuntime {
        &self.runtime
    }

    /// 借用当前 TUI 状态快照。
    pub(crate) fn state(&self) -> &TuiState {
        &self.state
    }

    /// 可变借用 TUI 状态；调用方需继续维护 reducer 不变量。
    pub(crate) fn state_mut(&mut self) -> &mut TuiState {
        &mut self.state
    }

    /// 借用聊天渲染部件。
    pub(crate) fn widget(&self) -> &ChatWidget {
        &self.widget
    }

    /// 可变借用聊天渲染部件，供宿主同步草稿与历史。
    pub(crate) fn widget_mut(&mut self) -> &mut ChatWidget {
        &mut self.widget
    }

    /// 返回创建应用时选定的终端模式。
    pub(crate) fn terminal_mode(&self) -> TerminalMode {
        self.terminal_mode
    }

    /// 使用聊天视图同一批已提交与活跃单元构造完整 transcript 阅读器。
    pub(crate) fn transcript_pager(&self, width: u16) -> TranscriptPager {
        TranscriptPager::new(self.transcript_lines(width, true))
    }

    /// 按宽度生成 transcript 行，供内联历史插入和窗口缩放回放共用。
    pub(crate) fn transcript_lines(&self, width: u16, include_active: bool) -> Vec<Line<'static>> {
        self.widget.transcript_lines(width, include_active)
    }

    /// 仅将已提交单元写入终端原生回滚区，不泄漏尚未完成的流式内容。
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

    /// 仅在宽度确实变化时返回可用于回滚重放的源数据行。
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

/// 作为 CLI 交互入口启动 Gateway、恢复会话并运行终端循环。
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

/// 优先恢复显式或最近会话；无可用会话时向 Gateway 创建新会话。
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

/// 根据当前状态和提交快照构造 Gateway 请求，并在此收紧工具与权限选项。
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
    let mut runner = ActiveRunner::new();
    runner.request_interaction_refresh(&mut app);
    let mut editor_terminal_suspended = false;

    loop {
        runner.drain(&mut app);
        if editor_terminal_suspended && !runner.editor_in_flight() {
            crossterm::terminal::enable_raw_mode()?;
            terminal.clear()?;
            editor_terminal_suspended = false;
        }
        if runner.editor_in_flight() {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        let width = crossterm::terminal::size()?.0;
        runner.refresh_pager(&app, width);
        draw(&app, runner.transcript_pager.as_mut(), &mut terminal)?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        if runner.has_transcript_pager() {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    runner.handle_pager_key(key);
                }
                Event::Resize(width, _) => runner.refresh_pager(&app, width),
                _ => {}
            }
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

        let opens_editor = matches!(action, V3Action::OpenExternalEditor);
        if opens_editor {
            crossterm::terminal::disable_raw_mode()?;
        }
        let control = runner.execute_action(&mut app, action);
        if opens_editor {
            if runner.editor_in_flight() {
                editor_terminal_suspended = true;
            } else {
                crossterm::terminal::enable_raw_mode()?;
            }
        }
        if control == RunnerControl::Quit {
            return Ok(());
        }
    }
}

fn spawn_stream(
    runtime: GatewayUiRuntime,
    request: GatewayChatStreamRequest,
    request_id: String,
) -> ActiveStream {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let stream_cancel = Arc::clone(&cancel);
    let worker_request_id = request_id.clone();
    std::thread::spawn(move || {
        let result = runtime.stream_chat_resumable_blocking(
            &request,
            worker_request_id.as_str(),
            |event| {
                !stream_cancel.load(Ordering::Acquire)
                    && sender.send(StreamMessage::Event(event)).is_ok()
            },
        );
        let result = if stream_cancel.load(Ordering::Acquire) {
            Ok(GatewayStreamAttemptResult::CallerStopped)
        } else {
            result
        };
        let _ = sender.send(StreamMessage::Finished(result));
    });
    ActiveStream { request_id, receiver, cancel }
}

fn spawn_stop(runtime: GatewayUiRuntime, request_id: String, sender: mpsc::Sender<HostMessage>) {
    std::thread::spawn(move || {
        let result = runtime.chat_stream_stop_blocking(request_id.as_str());
        let _ = sender.send(HostMessage::StopFinished { request_id, result });
    });
}

fn spawn_interaction_refresh(
    runtime: GatewayUiRuntime,
    session_id: String,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = runtime.interaction_snapshot_blocking(Some(session_id.as_str()));
        let _ = sender.send(HostMessage::InteractionsRefreshed(result));
    });
}

fn spawn_permission_reply(
    runtime: GatewayUiRuntime,
    request_id: String,
    reply: PendingPermissionReplyDto,
    message: Option<String>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = runtime.permission_reply_blocking(request_id.as_str(), reply, message);
        let _ = sender.send(HostMessage::PermissionReplyFinished { request_id, result });
    });
}

fn spawn_question_reply(
    runtime: GatewayUiRuntime,
    request_id: String,
    answers: Option<Vec<Vec<String>>>,
    sender: mpsc::Sender<HostMessage>,
) {
    std::thread::spawn(move || {
        let result = match answers {
            Some(answers) => runtime.question_reply_blocking(request_id.as_str(), answers),
            None => runtime.question_reject_blocking(request_id.as_str()),
        };
        let _ = sender.send(HostMessage::QuestionReplyFinished { request_id, result });
    });
}

fn next_stream_request_id() -> String {
    static NEXT_SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default();
    let sequence = NEXT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("tui-{now_ms}-{sequence}")
}

fn draw(
    app: &V3GatewayApp,
    pager: Option<&mut TranscriptPager>,
    terminal: &mut V3Terminal,
) -> io::Result<()> {
    let (width, height) = crossterm::terminal::size()?;
    let y = match app.terminal_mode() {
        TerminalMode::Inline => terminal.viewport_area.y.min(height.saturating_sub(1)),
        TerminalMode::Alternate => 0,
    };
    terminal.set_viewport_area(Rect::new(0, y, width, height.saturating_sub(y)));
    terminal.draw(|frame| {
        let area = frame.area();
        if let Some(pager) = pager {
            pager.render(area, frame.buffer_mut());
        } else {
            app.widget().render(area, frame.buffer_mut());
            frame.set_cursor_style(app.widget().cursor_style(area));
            if let Some(position) = app.widget().cursor_pos(area) {
                frame.set_cursor_position(position);
            }
        }
    })
}
