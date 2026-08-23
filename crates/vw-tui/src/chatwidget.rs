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
use crate::vibe::action::{V3Action, V3ApprovalRequest, V3FooterData, V3UserInputRequest};
use crate::vibe::event::V3DisplayEvent;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
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

pub use constructor::ChatWidgetConfig;
pub use status_state::V3ChatStatus;

use input_queue::InputQueueState;
use status_state::StatusState;
use transcript::TranscriptState;
use user_messages::UserMessage;

enum ChatOverlay {
    Approval(Box<ApprovalOverlay>),
    UserInput(Box<RequestUserInputOverlay>),
}

impl ChatOverlay {
    fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        match self {
            Self::Approval(overlay) => overlay.handle_key_event(event),
            Self::UserInput(overlay) => overlay.handle_key_event(event),
        }
    }

}

impl Renderable for ChatOverlay {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        match self {
            Self::Approval(overlay) => overlay.render(area, buf),
            Self::UserInput(overlay) => overlay.render(area, buf),
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        match self {
            Self::Approval(overlay) => overlay.desired_height(width),
            Self::UserInput(overlay) => overlay.desired_height(width),
        }
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        match self {
            Self::Approval(overlay) => overlay.cursor_pos(area),
            Self::UserInput(overlay) => overlay.cursor_pos(area),
        }
    }

    fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        match self {
            Self::Approval(overlay) => overlay.cursor_style(area),
            Self::UserInput(overlay) => overlay.cursor_style(area),
        }
    }
}

/// Pure rendering shell for the copied main chat surface.
pub struct ChatWidget {
    bottom_pane: BottomPane,
    transcript: TranscriptState,
    input_queue: InputQueueState,
    status_state: StatusState,
    streaming: streaming::StreamingState,
    turn_lifecycle: turn_lifecycle::TurnLifecycleState,
    exec_state: exec_state::ExecState,
    interrupt_state: interrupts::InterruptState,
    overlay: Option<ChatOverlay>,
    last_rendered_width: Cell<Option<u16>>,
}

impl ChatWidget {
    /// Adds a completed display event to the committed transcript.
    pub fn push_history(&mut self, event: V3DisplayEvent) {
        self.transcript.push_history(event);
    }

    /// Adds the copied bordered session header supplied by the outer bridge.
    pub fn push_session_header(&mut self, data: crate::V3SessionDisplayData) {
        self.transcript
            .push_cell(Box::new(crate::history_cell::SessionHistoryCell::header(data)));
    }

    /// Replaces the existing copied session header after bridge-owned metadata changes.
    pub fn set_session_header(&mut self, data: crate::V3SessionDisplayData) {
        self.transcript.set_session_header(data);
    }

    /// Replaces the active cell shown below committed history.
    pub fn set_active_event(&mut self, event: Option<V3DisplayEvent>) {
        self.transcript.set_active_event(event);
    }

    /// Replaces the live tail shown after the active cell.
    pub fn set_live_tail_event(&mut self, event: Option<V3DisplayEvent>) {
        self.transcript.set_live_tail_event(event);
    }

    /// Commits the current active cell without interpreting its runtime meaning.
    pub fn commit_active_cell(&mut self) {
        self.transcript.commit_active_cell();
    }

    /// Replaces footer/status display data supplied by the outer bridge.
    pub fn set_footer_data(&mut self, footer_data: V3FooterData) {
        self.status_state.set_task_running(footer_data.is_task_running);
        self.bottom_pane.composer_mut().set_footer_data(footer_data);
    }

    /// Shows or hides the copied running-status surface.
    pub fn set_status(&mut self, status: V3ChatStatus) {
        let running = status.is_running();
        self.status_state.set(status);
        self.bottom_pane.composer_mut().set_task_running(running);
    }

    /// Shows a security approval overlay using read-only v3 request data.
    pub fn show_approval(&mut self, request: V3ApprovalRequest) {
        self.overlay = Some(ChatOverlay::Approval(Box::new(ApprovalOverlay::new(request))));
    }

    /// Shows a user-question overlay using read-only v3 request data.
    pub fn show_user_input(&mut self, request: V3UserInputRequest) {
        self.overlay =
            Some(ChatOverlay::UserInput(Box::new(RequestUserInputOverlay::new(request))));
    }

    /// Dismisses the current overlay without manufacturing a business decision.
    pub fn clear_overlay(&mut self) {
        self.overlay = None;
    }

    /// Returns whether an approval or user-input overlay is currently retained.
    #[must_use]
    pub fn has_overlay(&self) -> bool {
        self.overlay.is_some()
    }

    /// Routes one key through the visible overlay or copied composer.
    pub fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        if let Some(overlay) = self.overlay.as_mut() {
            let rejected_question_id = match overlay {
                ChatOverlay::UserInput(overlay) => Some(overlay.request().id.clone()),
                ChatOverlay::Approval(_) => None,
            };
            let action = overlay.handle_key_event(event);
            if matches!(action, V3Action::Cancel)
                && let Some(request_id) = rejected_question_id
            {
                return V3Action::AnswerUserInput { request_id, answers: Vec::new() };
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

    /// Scrolls the copied transcript with the terminal mouse wheel.
    pub fn handle_mouse_event(&mut self, event: MouseEvent) -> V3Action {
        match event.kind {
            MouseEventKind::ScrollUp => self.transcript.scroll_lines(-3),
            MouseEventKind::ScrollDown => self.transcript.scroll_lines(3),
            _ => {}
        }
        V3Action::None
    }

    /// Applies a paste to the local composer only.
    pub fn handle_paste(&mut self, text: String) -> V3Action {
        self.bottom_pane.handle_paste(text)
    }

    /// Returns the number of locally queued follow-up messages.
    #[must_use]
    pub fn queued_input_count(&self) -> usize {
        self.input_queue.len()
    }

    /// Adds one follow-up to the host-owned FIFO and its visible preview.
    pub fn queue_input(&mut self, text: String, attachments: Vec<crate::V3Attachment>) {
        self.bottom_pane.push_queued_input(text.clone());
        self.input_queue.push(UserMessage::new(text, attachments));
    }

    /// Removes the oldest locally queued follow-up for the outer bridge.
    pub fn pop_queued_input(&mut self) -> Option<(String, Vec<crate::V3Attachment>)> {
        let message = self.input_queue.pop()?;
        self.bottom_pane.pop_queued_input();
        Some((message.text, message.attachments))
    }

    /// Returns the current composer draft for host-owned editor integration.
    #[must_use]
    pub fn draft(&self) -> &str {
        self.bottom_pane.draft()
    }

    /// Replaces the composer draft with text returned by an external editor.
    pub fn replace_draft(&mut self, text: &str) {
        self.bottom_pane.replace_draft(text);
    }

    /// Number of committed history cells currently owned by the shell.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.transcript.history_len()
    }

    /// Returns width-aware committed history plus the optional active/live tail.
    ///
    /// The same source is used by inline history insertion, resize replay and the full pager.
    #[must_use]
    pub fn transcript_lines(&self, width: u16, include_active: bool) -> Vec<Line<'static>> {
        self.transcript.transcript_lines(width, include_active)
    }

    /// Paints the copied main chat layout into a Ratatui buffer.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        Renderable::render(self, area, buf);
    }

    /// Measures the copied main chat layout at the supplied width.
    #[must_use]
    pub fn desired_height(&self, width: u16) -> u16 {
        Renderable::desired_height(self, width)
    }

    /// Returns the composer or overlay cursor position.
    #[must_use]
    pub fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        Renderable::cursor_pos(self, area)
    }

    /// Returns the active composer or overlay cursor style.
    #[must_use]
    pub fn cursor_style(&self, area: Rect) -> SetCursorStyle {
        Renderable::cursor_style(self, area)
    }
}
