//! VibeWindow's local composer and footer boundary.
//!
//! The copied Codex rendering core is kept below the boundary, while all business outcomes leave
//! this module as [`V3Action`].  This module intentionally owns no sender, client, runtime, or
//! request object.

use crate::render::renderable::Renderable;
use crate::vibe::action::{V3Action, V3FooterData};
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MentionBinding {
    pub(crate) sigil: char,
    pub(crate) mention: String,
    pub(crate) path: String,
}

#[path = "bottom_pane_view.rs"]
mod bottom_pane_view;
#[path = "chat_composer_history.rs"]
pub(crate) mod chat_composer_history;
#[path = "paste_burst.rs"]
pub(crate) mod paste_burst;
#[path = "pending_input_preview.rs"]
mod pending_input_preview;
#[path = "textarea.rs"]
pub(crate) mod textarea;
pub(crate) use bottom_pane_view::BottomPaneView;
#[path = "action_required_title.rs"]
pub(crate) mod action_required_title;
#[path = "approval_overlay.rs"]
pub(crate) mod approval_overlay;
#[path = "chat_composer.rs"]
mod chat_composer;
#[path = "command_popup.rs"]
pub(crate) mod command_popup;
#[path = "file_search_popup.rs"]
pub(crate) mod file_search_popup;
#[path = "footer.rs"]
mod footer;
#[path = "chat_composer/footer_state.rs"]
mod footer_state;
#[path = "list_selection_view.rs"]
pub(crate) mod list_selection_view;
#[path = "request_user_input/mod.rs"]
pub(crate) mod request_user_input;
#[path = "scroll_state.rs"]
pub(crate) mod scroll_state;
#[path = "selection_popup_common.rs"]
pub(crate) mod selection_popup_common;
#[path = "selection_row_layout.rs"]
pub(crate) mod selection_row_layout;
#[path = "selection_tabs.rs"]
pub(crate) mod selection_tabs;
#[path = "slash_commands.rs"]
pub(crate) mod slash_commands;

pub(crate) use chat_composer::ChatComposer;
pub(crate) use pending_input_preview::PendingInputPreview;

pub(crate) struct BottomPaneParams {
    pub(crate) footer_data: V3FooterData,
    pub(crate) placeholder: String,
}

pub(crate) struct BottomPane {
    composer: ChatComposer,
    pending_input_preview: PendingInputPreview,
}

impl BottomPane {
    pub(crate) fn new(params: BottomPaneParams) -> Self {
        Self {
            composer: ChatComposer::with_data(params.placeholder, params.footer_data),
            pending_input_preview: PendingInputPreview::new(),
        }
    }

    pub(crate) fn composer(&self) -> &ChatComposer {
        &self.composer
    }

    pub(crate) fn composer_mut(&mut self) -> &mut ChatComposer {
        &mut self.composer
    }

    pub(crate) fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        self.composer.handle_key_event(event)
    }

    pub(crate) fn handle_paste(&mut self, text: String) -> V3Action {
        self.composer.handle_paste(text)
    }

    pub(crate) fn draft(&self) -> &str {
        self.composer.text()
    }

    pub(crate) fn replace_draft(&mut self, text: &str) {
        self.composer.replace_text(text);
    }

    pub(crate) fn push_queued_input(&mut self, text: String) {
        self.pending_input_preview.queued_messages.push(text);
        self.composer.set_queued_count(self.pending_input_preview.queued_messages.len());
    }

    pub(crate) fn pop_queued_input(&mut self) {
        if !self.pending_input_preview.queued_messages.is_empty() {
            self.pending_input_preview.queued_messages.remove(0);
        }
        self.composer.set_queued_count(self.pending_input_preview.queued_messages.len());
    }
}

impl Renderable for BottomPane {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let preview_height = self.pending_input_preview.desired_height(area.width);
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(preview_height), Constraint::Min(1)])
            .split(area);
        if preview_height > 0 {
            self.pending_input_preview.render(areas[0], buf);
        }
        self.composer.render(areas[1], buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.pending_input_preview.desired_height(width) + self.composer.desired_height(width)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        let preview_height = self.pending_input_preview.desired_height(area.width);
        let composer_area = Rect::new(
            area.x,
            area.y + preview_height,
            area.width,
            area.height.saturating_sub(preview_height),
        );
        self.composer.cursor_pos(composer_area)
    }
}
