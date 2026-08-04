//! tui_v2 controller 骨架。
//!
//! controller 负责 terminal 事件轮询、全局按键与状态动作边界；具体 prompt、scroll
//! 和 overlay 键位归一化分别委托给 `keymap` 与 `overlay_controller`。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::event::{
    self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent,
    MouseEventKind,
};
use vw_config_types::skills::SkillsDirectoryProvider;

use super::history_search::handle_history_search_key;
use super::input::TuiSlashCommandInvocation;
use super::editor::selected_text;
use super::keybindings::{is_prompt_selection_copy, is_reserved_exit};
use super::keymap::{
    KeymapLayerResult, handle_prompt_layer_key, handle_scroll_layer_key, scroll_relative,
    scroll_to_tail, seek_main_scrollbar,
};
use super::message_action_controller::handle_message_action_key;
use super::model::settings_panel::SettingsPanel;
use super::model::integration::IntegrationPanel;
use super::model::{
    MessageActionIntent, PermissionDecision, PromptSubmission, PromptSubmissionStatus,
    TranscriptEffectIntent, UiConfirmOverlay, UiQuestionOverlay, UiTodoOverlay,
};
use super::overlay_controller::handle_overlay_layer_key;
use super::paste::PromptEditorIntent;
use super::paste::apply_terminal_paste;
use super::render::layout::FullscreenLayoutSlots;
use super::render::{TuiScrollbarFeedback, TuiScrollbarView};
use super::state::{
    TuiAction, TuiState, reduce_tui_state, select_visible_grouped_transcript_window,
};
use super::transcript_controller::{
    handle_transcript_mode_key, resize_transcript_mode, scroll_transcript_mode,
    seek_transcript_scrollbar,
};
use super::vim::handle_vim_key;

const MOUSE_SCROLL_STEP: isize = 3;

/// controller 轮询得到的归一化事件。
#[derive(Debug)]
pub(crate) enum TuiControllerEvent {
    Tick,
    Terminal(CrosstermEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiOverlayCommand {
    OpenSearchOverlay,
    OpenPendingQuestions,
    OpenTodoPanel,
    OpenTaskPanel,
    OpenMcpPanel,
    OpenMemoryPanel,
    ConfirmExit,
    ConfirmAccepted(UiConfirmOverlay),
    QuestionSubmitted(UiQuestionOverlay),
    QuestionRejected(UiQuestionOverlay),
    TodoRefresh(UiTodoOverlay),
    TodoSave(UiTodoOverlay),
    SettingsSave(Box<SettingsPanel>),
    PermissionDecision(PermissionDecision),
    IntegrationRefresh(IntegrationPanel),
    IntegrationSkillProviderSelected(IntegrationPanel, SkillsDirectoryProvider),
    IntegrationToggle(IntegrationPanel),
    IntegrationDeleteRequested(IntegrationPanel),
}

/// controller 每次处理事件后给 app 的控制指令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiControllerCommand {
    Continue,
    Quit,
    CancelActiveSubmission,
    Overlay(TuiOverlayCommand),
    SubmitPrompt(PromptSubmission),
    ExecuteSlashCommand(TuiSlashCommandInvocation),
    TranscriptEffect(TranscriptEffectIntent),
    MessageAction(MessageActionIntent),
    PromptExternalEditor(PromptEditorIntent),
    CopyPromptSelection(String),
}

/// fullscreen skeleton 的 terminal controller。
#[derive(Debug, Clone)]
pub(crate) struct TuiController {
    tick_rate: Duration,
    spinner_frame: usize,
    scrollbar: Option<TuiScrollbarFeedback>,
    scrollbar_drag: Option<TuiScrollbarDrag>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TuiScrollbarDrag {
    view: TuiScrollbarView,
    grab_offset: usize,
}

impl Default for TuiController {
    fn default() -> Self {
        Self::new(Duration::from_millis(180))
    }
}

impl TuiController {
    /// 基于给定 tick 速率创建 controller。
    pub(crate) fn new(tick_rate: Duration) -> Self {
        Self { tick_rate, spinner_frame: 0, scrollbar: None, scrollbar_drag: None }
    }

    /// 返回当前动画帧索引，供 renderer 在 header 中绘制轻量状态指示。
    pub(crate) fn spinner_frame(&self) -> usize {
        self.spinner_frame
    }

    /// 轮询下一个终端事件；超时后返回 `Tick`。
    pub(crate) fn next_event(&mut self) -> Result<TuiControllerEvent> {
        self.next_event_with_timeout(self.tick_rate)
    }

    /// 使用显式超时轮询一个终端事件；超时后返回 `Tick`。
    pub(crate) fn next_event_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<TuiControllerEvent> {
        if event::poll(timeout)? {
            Ok(TuiControllerEvent::Terminal(event::read()?))
        } else {
            self.spinner_frame = (self.spinner_frame + 1) % 4;
            Ok(TuiControllerEvent::Tick)
        }
    }

    /// 将 renderer 反馈的 viewport 能力同步回状态层。
    pub(crate) fn sync_layout(&mut self, state: &mut TuiState, slots: &impl TuiRenderFeedbackLike) {
        let layout = slots.layout();
        let scrollbar = slots.scrollbar().copied();
        if self.scrollbar_drag.is_some_and(|drag| {
            scrollbar.is_none_or(|feedback| feedback.view != drag.view)
        }) {
            self.scrollbar_drag = None;
        }
        self.scrollbar = scrollbar;
        let next_viewport_height = layout.scrollable_body.height;
        let next_viewport_width = layout.scrollable_body.width;
        if state.scroll.viewport_height == next_viewport_height
            && state.scroll.viewport_width == next_viewport_width
        {
            return;
        }

        let mut scroll = state.scroll.clone();
        scroll.sync_viewport(next_viewport_height, next_viewport_width);
        reduce_tui_state(state, TuiAction::ScrollSet(scroll));
        state.refresh_transcript_layout_for_current_width();
    }

    /// 处理一个 controller 事件。
    pub(crate) fn handle_event(
        &mut self,
        state: &mut TuiState,
        event: TuiControllerEvent,
    ) -> TuiControllerCommand {
        match event {
            TuiControllerEvent::Tick => TuiControllerCommand::Continue,
            TuiControllerEvent::Terminal(event) => self.handle_terminal_event(state, event),
        }
    }

    fn handle_terminal_event(
        &mut self,
        state: &mut TuiState,
        event: CrosstermEvent,
    ) -> TuiControllerCommand {
        match event {
            CrosstermEvent::Key(key)
                if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
            {
                self.handle_key_event(state, key)
            }
            CrosstermEvent::Resize(_, height) => {
                self.clear_scrollbar_interaction();
                resize_transcript_mode(state, height.saturating_sub(2));
                // 新尺寸由下一帧 renderer 反馈；先丢弃旧 viewport/wrap 几何，避免误用。
                let mut scroll = state.scroll.clone();
                scroll.sync_viewport(0, 0);
                reduce_tui_state(state, TuiAction::ScrollSet(scroll));
                state.transcript_layout.clear();
                TuiControllerCommand::Continue
            }
            CrosstermEvent::FocusGained => {
                // app 会在每个归一化事件后完整 draw，FocusGained 同时覆盖 resume 重绘。
                TuiControllerCommand::Continue
            }
            CrosstermEvent::FocusLost => {
                self.scrollbar_drag = None;
                TuiControllerCommand::Continue
            }
            CrosstermEvent::Paste(text) => {
                apply_terminal_paste(&mut state.prompt, text.as_str());
                TuiControllerCommand::Continue
            }
            CrosstermEvent::Mouse(mouse) => self.handle_mouse_event(state, mouse),
            CrosstermEvent::Key(_) => TuiControllerCommand::Continue,
        }
    }

    fn handle_mouse_event(
        &mut self,
        state: &mut TuiState,
        mouse: MouseEvent,
    ) -> TuiControllerCommand {
        match mouse.kind {
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let delta = if matches!(mouse.kind, MouseEventKind::ScrollUp) {
                    -MOUSE_SCROLL_STEP
                } else {
                    MOUSE_SCROLL_STEP
                };
                if state.transcript_mode.active {
                    scroll_transcript_mode(state, delta);
                } else {
                    scroll_relative(state, delta);
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.begin_scrollbar_drag(state, mouse.column, mouse.row);
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                self.continue_scrollbar_drag(state, mouse.row);
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.scrollbar_drag = None;
            }
            _ => {}
        }
        TuiControllerCommand::Continue
    }

    fn begin_scrollbar_drag(&mut self, state: &mut TuiState, column: u16, row: u16) {
        self.scrollbar_drag = None;
        if !state.transcript_mode.active && state.overlays.active().is_some() {
            return;
        }
        let Some(scrollbar) = self.active_scrollbar(state) else {
            return;
        };
        if !rect_contains(scrollbar.track, column, row) {
            return;
        }

        let grab_offset = if rect_contains(scrollbar.thumb, column, row) {
            usize::from(row.saturating_sub(scrollbar.thumb.y))
        } else {
            usize::from(scrollbar.thumb.height / 2)
        };
        self.seek_scrollbar(state, scrollbar, row, grab_offset);
        self.scrollbar_drag = Some(TuiScrollbarDrag { view: scrollbar.view, grab_offset });
    }

    fn continue_scrollbar_drag(&mut self, state: &mut TuiState, row: u16) {
        if !state.transcript_mode.active && state.overlays.active().is_some() {
            self.scrollbar_drag = None;
            return;
        }
        let Some(drag) = self.scrollbar_drag else {
            return;
        };
        let Some(scrollbar) = self.active_scrollbar(state) else {
            self.scrollbar_drag = None;
            return;
        };
        if scrollbar.view != drag.view {
            self.scrollbar_drag = None;
            return;
        }
        self.seek_scrollbar(state, scrollbar, row, drag.grab_offset);
    }

    fn active_scrollbar(&self, state: &TuiState) -> Option<TuiScrollbarFeedback> {
        let expected_view = if state.transcript_mode.active {
            TuiScrollbarView::Transcript
        } else {
            TuiScrollbarView::Main
        };
        self.scrollbar.filter(|scrollbar| scrollbar.view == expected_view)
    }

    fn seek_scrollbar(
        &self,
        state: &mut TuiState,
        scrollbar: TuiScrollbarFeedback,
        row: u16,
        grab_offset: usize,
    ) {
        let max_position = scrollbar.content_length.saturating_sub(scrollbar.viewport_length);
        let max_thumb_start =
            usize::from(scrollbar.track.height.saturating_sub(scrollbar.thumb.height));
        if max_position == 0 || max_thumb_start == 0 {
            return;
        }

        let row_offset = i32::from(row) - i32::from(scrollbar.track.y);
        let desired_thumb_start =
            (row_offset - i32::try_from(grab_offset).unwrap_or(i32::MAX))
                .clamp(0, i32::try_from(max_thumb_start).unwrap_or(i32::MAX))
                .cast_unsigned() as usize;
        let current_thumb_start =
            usize::from(scrollbar.thumb.y.saturating_sub(scrollbar.track.y));
        let position = if desired_thumb_start == current_thumb_start {
            scrollbar.position
        } else {
            desired_thumb_start
                .saturating_mul(max_position)
                .saturating_add(max_thumb_start / 2)
                / max_thumb_start
        };
        match scrollbar.view {
            TuiScrollbarView::Main => seek_main_scrollbar(state, position, max_position),
            TuiScrollbarView::Transcript => seek_transcript_scrollbar(state, position),
        }
    }

    fn clear_scrollbar_interaction(&mut self) {
        self.scrollbar = None;
        self.scrollbar_drag = None;
    }

    fn handle_key_event(&mut self, state: &mut TuiState, key: KeyEvent) -> TuiControllerCommand {
        if let Some(command) = handle_transcript_mode_key(state, key) {
            return command;
        }

        if state.overlays.active().is_none()
            && is_prompt_selection_copy(key)
            && let Some(text) = selected_text(&state.prompt)
        {
            return TuiControllerCommand::CopyPromptSelection(text);
        }

        if is_reserved_exit(key) {
            // 若会话中已有消息，弹二次确认；无消息则直接退出
            if state.messages.is_empty() && state.session.session_id.is_none() {
                return TuiControllerCommand::Quit;
            }
            return TuiControllerCommand::Overlay(TuiOverlayCommand::ConfirmExit);
        }

        if let KeymapLayerResult::Handled(command) = handle_overlay_layer_key(state, key) {
            return command;
        }

        if let Some(command) = handle_vim_key(state, key) {
            return command;
        }
        if let Some(command) = handle_history_search_key(state, key) {
            return command;
        }
        if let Some(command) = handle_message_action_key(state, key) {
            return command;
        }

        if key.code == KeyCode::Enter
            && state.overlays.active().is_none()
            && state.prompt.value.is_empty()
            && select_visible_grouped_transcript_window(state).unseen_range().is_some()
        {
            scroll_to_tail(state);
            return TuiControllerCommand::Continue;
        }

        if let KeymapLayerResult::Handled(command) = handle_prompt_layer_key(state, key) {
            return command;
        }

        if let KeymapLayerResult::Handled(command) = handle_scroll_layer_key(state, key) {
            return command;
        }

        TuiControllerCommand::Continue
    }

    fn handle_escape(&mut self, state: &mut TuiState) -> TuiControllerCommand {
        if state.overlays.active().is_some() {
            reduce_tui_state(state, TuiAction::OverlayPopped);
            return TuiControllerCommand::Continue;
        }

        if !state.prompt.value.is_empty() {
            reduce_tui_state(state, TuiAction::PromptValueSet(String::new()));
            return TuiControllerCommand::Continue;
        }

        TuiControllerCommand::Quit
    }
}

/// 为了避免 controller 依赖 renderer 具体实现，这里只约定同步布局时所需的最小视图。
pub(crate) trait TuiRenderFeedbackLike {
    fn layout(&self) -> &FullscreenLayoutSlots;

    fn scrollbar(&self) -> Option<&TuiScrollbarFeedback> {
        None
    }
}

impl TuiRenderFeedbackLike for super::render::TuiRenderFeedback {
    fn layout(&self) -> &FullscreenLayoutSlots {
        &self.layout
    }

    fn scrollbar(&self) -> Option<&TuiScrollbarFeedback> {
        self.scrollbar.as_ref()
    }
}

fn rect_contains(rect: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    column >= rect.x && column < rect.right() && row >= rect.y && row < rect.bottom()
}

pub(crate) fn build_prompt_submission(state: &TuiState, text: &str) -> Option<PromptSubmission> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return None;
    }

    let mut submission = PromptSubmission::new(text.clone())
        .with_stream_id(now_ms())
        .with_history_len(state.prompt.history.entries.len());
    submission.execution = Box::new(state.session_options.execution_snapshot());

    if let Some(session_id) = state.session.session_id.as_ref() {
        submission = submission.with_session_id(session_id.clone());
    }

    if let Some(root) = state.project.workspace_root.as_ref() {
        submission = submission.with_root(root.display().to_string());
    }

    if let Some(model) = state.status.model_name.as_ref() {
        submission = submission.with_model(model.clone());
    }

    Some(submission)
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

#[allow(dead_code)]
fn _assert_prompt_status(_: PromptSubmissionStatus) {}
