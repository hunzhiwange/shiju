//! 复制自 Codex 主聊天界面的 VibeWindow 适配层。
//!
//! 本模块刻意只持有展示状态：已提交历史、活动单元和流式尾部、复制的底部面板、
//! 状态文本、本地排队输入及覆盖层。运行时请求和协议生命周期留在本 crate 之外。
//! VibeWindow adaptation of the copied Codex main chat surface.
//!
//! This module deliberately owns presentation state only: committed history,
//! the active cell and live tail, the copied bottom pane, status text, queued
//! local input, and overlays. Runtime requests and protocol lifecycle handling
//! stay outside this crate.

use std::cell::Cell;

use crate::bottom_pane::approval_overlay::ApprovalOverlay;
use crate::bottom_pane::request_user_input::RequestUserInputOverlay;
use crate::bottom_pane::{BottomPane, BottomPaneView};
use crate::history_cell::HistoryCell;
use crate::render::renderable::Renderable;
use crate::vibe::action::{
    V3Action, V3ApprovalRequest, V3FileSearchMatch, V3FooterData, V3SelectionItem,
    V3UserInputRequest,
};
use crate::vibe::event::{V3DisplayEvent, V3FileDiff};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::text::Line;

mod constructor;
mod command_lifecycle;
mod exec_state;
mod input_queue;
mod input_submission;
mod interrupts;
mod rendering;
mod selection_overlay;
mod status_state;
mod status_surfaces;
mod streaming;
mod tool_lifecycle;
mod transcript;
mod turn_lifecycle;
mod user_messages;

#[cfg(test)]
#[path = "chatwidget/tests/exec_flow.rs"]
mod exec_flow;

#[cfg(test)]
#[path = "chatwidget/tests/status_and_layout.rs"]
mod status_and_layout;

#[cfg(test)]
#[path = "chatwidget/selection_overlay_tests.rs"]
mod selection_overlay_tests;

pub use constructor::ChatWidgetConfig;
pub use status_state::V3ChatStatus;

use input_queue::InputQueueState;
use status_state::StatusState;
use transcript::{TranscriptMouseAction, TranscriptState};
use user_messages::UserMessage;

enum ChatOverlay {
    Approval(Box<ApprovalOverlay>),
    UserInput(Box<RequestUserInputOverlay>),
    Selection(Box<selection_overlay::SelectionOverlay>),
}

impl ChatOverlay {
    fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        match self {
            Self::Approval(overlay) => overlay.handle_key_event(event),
            Self::UserInput(overlay) => overlay.handle_key_event(event),
            Self::Selection(overlay) => overlay.handle_key_event(event),
        }
    }

}

impl Renderable for ChatOverlay {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        match self {
            Self::Approval(overlay) => overlay.render(area, buf),
            Self::UserInput(overlay) => overlay.render(area, buf),
            Self::Selection(overlay) => overlay.render(area, buf),
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        match self {
            Self::Approval(overlay) => overlay.desired_height(width),
            Self::UserInput(overlay) => overlay.desired_height(width),
            Self::Selection(overlay) => overlay.desired_height(width),
        }
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        match self {
            Self::Approval(overlay) => overlay.cursor_pos(area),
            Self::UserInput(overlay) => overlay.cursor_pos(area),
            Self::Selection(overlay) => overlay.cursor_pos(area),
        }
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        match self {
            Self::Approval(overlay) => overlay.cursor_style(area),
            Self::UserInput(overlay) => overlay.cursor_style(area),
            Self::Selection(overlay) => overlay.cursor_style(area),
        }
    }
}

/// 复制主聊天界面的纯渲染外壳，不解释运行时业务语义。
/// Pure rendering shell for the copied main chat surface.
pub struct ChatWidget {
    bottom_pane: BottomPane,
    transcript: TranscriptState,
    input_queue: InputQueueState,
    status_state: StatusState,
    streaming: streaming::StreamingState,
    turn_lifecycle: turn_lifecycle::TurnLifecycleState,
    exec_state: exec_state::ExecState,
    pending_file_diffs: Vec<V3FileDiff>,
    interrupt_state: interrupts::InterruptState,
    overlay: Option<ChatOverlay>,
    clipboard_lease: Option<crate::clipboard_copy::ClipboardLease>,
    last_rendered_width: Cell<Option<u16>>,
}

impl ChatWidget {
    /// 将完成的展示事件加入已提交记录。
    /// Adds a completed display event to the committed transcript.
    pub fn push_history(&mut self, event: V3DisplayEvent) {
        self.transcript.push_history(event);
    }

    /// 添加外层桥接提供的带边框会话标题。
    /// Adds the copied bordered session header supplied by the outer bridge.
    pub fn push_session_header(&mut self, data: crate::V3SessionDisplayData) {
        self.transcript
            .push_cell(Box::new(crate::history_cell::SessionHistoryCell::header(data)));
    }

    /// 桥接层拥有的元数据变化后替换现有会话标题。
    /// Replaces the existing copied session header after bridge-owned metadata changes.
    pub fn set_session_header(&mut self, data: crate::V3SessionDisplayData) {
        self.transcript.set_session_header(data);
    }

    /// 替换已提交历史下方展示的活动单元。
    /// Replaces the active cell shown below committed history.
    pub fn set_active_event(&mut self, event: Option<V3DisplayEvent>) {
        self.transcript.set_active_event(event);
    }

    /// 替换活动单元之后展示的流式尾部。
    /// Replaces the live tail shown after the active cell.
    pub fn set_live_tail_event(&mut self, event: Option<V3DisplayEvent>) {
        self.transcript.set_live_tail_event(event);
    }

    /// 提交当前活动单元，但不解释其运行时含义。
    /// Commits the current active cell without interpreting its runtime meaning.
    pub fn commit_active_cell(&mut self) {
        self.transcript.commit_active_cell();
    }

    /// 替换外层桥接提供的页脚和状态展示数据。
    /// Replaces footer/status display data supplied by the outer bridge.
    pub fn set_footer_data(&mut self, footer_data: V3FooterData) {
        self.status_state.set_task_running(footer_data.is_task_running);
        self.bottom_pane.composer_mut().set_footer_data(footer_data);
    }

    /// 显示或隐藏复制的运行状态界面。
    /// Shows or hides the copied running-status surface.
    pub fn set_status(&mut self, status: V3ChatStatus) {
        let running = status.is_running();
        self.status_state.set(status);
        self.bottom_pane.composer_mut().set_task_running(running);
    }

    /// 使用只读 v3 请求数据展示安全审批覆盖层。
    /// Shows a security approval overlay using read-only v3 request data.
    pub fn show_approval(&mut self, request: V3ApprovalRequest) {
        self.overlay = Some(ChatOverlay::Approval(Box::new(ApprovalOverlay::new(request))));
    }

    /// 使用只读 v3 请求数据展示用户问题覆盖层。
    /// Shows a user-question overlay using read-only v3 request data.
    pub fn show_user_input(&mut self, request: V3UserInputRequest) {
        self.overlay =
            Some(ChatOverlay::UserInput(Box::new(RequestUserInputOverlay::new(request))));
    }

    /// 展示仅由稳定项目 ID 和展示数据支持的可搜索选择界面。
    /// Shows a searchable selection surface backed only by stable item IDs and display data.
    pub fn show_selection_overlay(
        &mut self,
        surface: impl Into<String>,
        title: impl Into<String>,
        items: Vec<V3SelectionItem>,
    ) {
        self.overlay = Some(ChatOverlay::Selection(Box::new(
            selection_overlay::SelectionOverlay::new(surface, title, items),
        )));
    }

    /// 替换底部面板斜杠命令面板使用的数据。
    /// Replaces the data used by the slash-command palette in the bottom pane.
    pub fn set_command_palette_items(&mut self, items: Vec<V3SelectionItem>) {
        self.bottom_pane.set_command_palette_items(items);
    }

    /// 返回斜杠输入当前是否显示命令面板。
    /// Returns whether slash input currently exposes the command palette.
    #[must_use]
    pub fn command_palette_visible(&self) -> bool {
        self.bottom_pane.command_palette_visible()
    }

    /// Supplies successful file matches for the still-active query.
    /// 为仍处于活动状态的查询注入成功的文件匹配结果。
    pub fn set_file_search_matches(
        &mut self,
        query: &str,
        matches: Vec<V3FileSearchMatch>,
    ) {
        self.bottom_pane.set_file_search_matches(query, matches);
    }

    /// Supplies an unavailable state for the still-active query.
    /// 为仍处于活动状态的查询注入明确的不可用状态。
    pub fn set_file_search_error(&mut self, query: &str, message: impl Into<String>) {
        self.bottom_pane.set_file_search_error(query, message.into());
    }

    /// 关闭当前覆盖层，但不伪造业务决策。
    /// Dismisses the current overlay without manufacturing a business decision.
    pub fn clear_overlay(&mut self) {
        self.overlay = None;
    }

    /// 返回当前是否保留任一交互覆盖层，包括审批、用户输入或选择界面。
    /// Returns whether an approval or user-input overlay is currently retained.
    #[must_use]
    pub fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }

    /// 将一个按键路由到可见覆盖层或复制的输入框。
    /// Routes one key through the visible overlay or copied composer.
    pub fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        if let Some(overlay) = self.overlay.as_mut() {
            let is_selection = matches!(overlay, ChatOverlay::Selection(_));
            let rejected_question_id = match overlay {
                ChatOverlay::UserInput(overlay) => Some(overlay.request().id.clone()),
                ChatOverlay::Approval(_) | ChatOverlay::Selection(_) => None,
            };
            let action = overlay.handle_key_event(event);
            if matches!(action, V3Action::Cancel)
                && let Some(request_id) = rejected_question_id
            {
                return V3Action::AnswerUserInput { request_id, answers: Vec::new() };
            }
            if is_selection && matches!(action, V3Action::Select { .. } | V3Action::Cancel) {
                self.overlay = None;
                return if matches!(action, V3Action::Cancel) { V3Action::None } else { action };
            }
            return action;
        }

        if event.kind == KeyEventKind::Press {
            match (event.code, event.modifiers) {
                (KeyCode::PageUp, _) => {
                    self.transcript.scroll_page(-1);
                    return V3Action::None;
                }
                (KeyCode::PageDown, _) => {
                    self.transcript.scroll_page(1);
                    return V3Action::None;
                }
                (KeyCode::Home, modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.transcript.scroll_to_top();
                    return V3Action::None;
                }
                (KeyCode::End, modifiers) if modifiers.contains(KeyModifiers::CONTROL) => {
                    self.transcript.scroll_to_bottom();
                    return V3Action::None;
                }
                _ => {}
            }
        }

        self.bottom_pane.handle_key_event(event)
    }

    /// 使用终端鼠标滚轮滚动复制的记录。
    /// Scrolls the copied transcript with the terminal mouse wheel.
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> V3Action {
        if let TranscriptMouseAction::Copy(text) = self.transcript.handle_mouse_event(event) {
            match crate::clipboard_copy::copy_to_clipboard(text.as_str()) {
                Ok(lease) => self.clipboard_lease = lease,
                Err(error) => self
                    .transcript
                    .push_history(V3DisplayEvent::Error { message: format!("复制失败：{error}") }),
            }
        }
        V3Action::None
    }

    /// 仅把粘贴内容应用到本地输入界面；选择覆盖层可将其作为搜索文本处理。
    /// Applies a paste to the local composer only.
    pub fn handle_paste(&mut self, text: String) -> V3Action {
        if let Some(ChatOverlay::Selection(overlay)) = self.overlay.as_mut() {
            overlay.handle_paste(text.as_str());
            return V3Action::None;
        }
        self.bottom_pane.handle_paste(text)
    }

    /// 返回本地排队的后续消息数量。
    /// Returns the number of locally queued follow-up messages.
    #[must_use]
    pub fn queued_input_count(&self) -> usize {
        self.input_queue.len()
    }

    /// 向宿主拥有的 FIFO 及其可见预览添加一条后续消息。
    /// Adds one follow-up to the host-owned FIFO and its visible preview.
    pub fn queue_input(&mut self, text: String, attachments: Vec<crate::V3Attachment>) {
        self.bottom_pane.push_queued_input(text.clone());
        self.input_queue.push(UserMessage::new(text, attachments));
    }

    /// 为外层桥接移除最早排队的本地后续消息。
    /// Removes the oldest locally queued follow-up for the outer bridge.
    pub fn pop_queued_input(&mut self) -> Option<(String, Vec<crate::V3Attachment>)> {
        let message = self.input_queue.pop()?;
        self.bottom_pane.pop_queued_input();
        Some((message.text, message.attachments))
    }

    /// 返回当前输入草稿，供宿主拥有的编辑器集成使用。
    /// Returns the current composer draft for host-owned editor integration.
    #[must_use]
    pub fn draft(&self) -> &str {
        self.bottom_pane.draft()
    }

    /// 使用外部编辑器返回的文本替换输入草稿。
    /// Replaces the composer draft with text returned by an external editor.
    pub fn replace_draft(&mut self, text: &str) {
        self.bottom_pane.replace_draft(text);
    }

    /// 提交未能启动时原子恢复文本与附件。
    pub fn restore_submission(
        &mut self,
        text: &str,
        attachments: Vec<crate::V3Attachment>,
    ) {
        self.bottom_pane.restore_submission(text, attachments);
    }

    /// 向本地 composer 添加一个由宿主验证的附件候选。
    pub fn add_attachment(&mut self, attachment: crate::V3Attachment) -> V3Action {
        self.bottom_pane.add_attachment(attachment)
    }

    /// 返回 composer 中尚未提交的附件。
    #[must_use]
    pub fn attachments(&self) -> &[crate::V3Attachment] {
        self.bottom_pane.attachments()
    }

    /// 当前由外壳持有的已提交历史单元数量。
    /// Number of committed history cells currently owned by the shell.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.transcript.history_len()
    }

    /// 返回按宽度渲染的已提交历史及可选活动/流式尾部；内联历史插入、尺寸重放和完整分页器共用该来源。
    /// Returns width-aware committed history plus the optional active/live tail.
    ///
    /// The same source is used by inline history insertion, resize replay and the full pager.
    #[must_use]
    pub fn transcript_lines(&self, width: u16, include_active: bool) -> Vec<Line<'static>> {
        self.transcript.transcript_lines(width, include_active)
    }

    /// 将复制的主聊天布局绘制到 Ratatui 缓冲区。
    /// Paints the copied main chat layout into a Ratatui buffer.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        Renderable::render(self, area, buf);
    }

    /// 在给定宽度下测量复制的主聊天布局。
    /// Measures the copied main chat layout at the supplied width.
    #[must_use]
    pub fn desired_height(&self, width: u16) -> u16 {
        Renderable::desired_height(self, width)
    }

    /// 返回输入框或覆盖层的光标位置。
    /// Returns the composer or overlay cursor position.
    #[must_use]
    pub fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        Renderable::cursor_pos(self, area)
    }

    /// 返回活动输入框或覆盖层的光标样式。
    /// Returns the active composer or overlay cursor style.
    #[must_use]
    pub fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        Renderable::cursor_style(self, area)
    }
}
