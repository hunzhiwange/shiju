//! 底部输入面板装配：组合器、输入预览与命令/文件搜索弹层。

use tui_render::renderable::Renderable;
use tui_contracts::action::{V3Action, V3FileSearchMatch, V3FooterData, V3SelectionItem};
use tui_composer::ChatComposer;
use tui_composer::pending_input_preview::PendingInputPreview;
use tui_popups::{command_popup, file_search_popup};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::WidgetRef;

#[cfg(test)]
mod file_search_integration_tests;

pub struct BottomPaneParams {
    pub footer_data: V3FooterData,
    pub placeholder: String,
}

pub struct BottomPane {
    composer: ChatComposer,
    pending_input_preview: PendingInputPreview,
    command_items: Vec<V3SelectionItem>,
    command_popup: Option<command_popup::CommandPopup>,
    file_search_query: Option<String>,
    file_search_popup: Option<file_search_popup::FileSearchPopup>,
}

impl BottomPane {
    pub fn new(params: BottomPaneParams) -> Self {
        Self {
            composer: ChatComposer::with_data(params.placeholder, params.footer_data),
            pending_input_preview: PendingInputPreview::new(),
            command_items: Vec::new(),
            command_popup: None,
            file_search_query: None,
            file_search_popup: None,
        }
    }

    pub fn composer(&self) -> &ChatComposer {
        &self.composer
    }

    pub fn composer_mut(&mut self) -> &mut ChatComposer {
        &mut self.composer
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        if let Some(popup) = self.file_search_popup.as_mut() {
            match event.code {
                KeyCode::Esc => {
                    self.close_file_search();
                    return V3Action::FileSearchQueryChanged { query: None };
                }
                KeyCode::Up | KeyCode::Down => return popup.handle_key_event(event),
                KeyCode::Enter => {
                    if let Some(path) = popup.selected_match().map(|item| item.path.clone()) {
                        self.composer.replace_file_search_token(path.as_str());
                        self.close_file_search();
                        return V3Action::FileSearchQueryChanged { query: None };
                    }
                }
                _ => {}
            }
        }

        if let Some(popup) = self.command_popup.as_mut() {
            match event.code {
                KeyCode::Esc => {
                    self.command_popup = None;
                    return V3Action::None;
                }
                KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                    return popup.handle_key_event(event);
                }
                KeyCode::Enter if popup.selected_item().is_some() => {
                    let action = popup.handle_key_event(event);
                    if matches!(action, V3Action::Select { .. }) {
                        self.composer.clear();
                        self.command_popup = None;
                    }
                    return action;
                }
                _ => {}
            }
        }

        let action = self.composer.handle_key_event(event);
        let query_action = self.sync_popups();
        if matches!(action, V3Action::None) { query_action.unwrap_or(action) } else { action }
    }

    /// Refresh popups after idle input becomes visible in the composer.
    pub fn flush_paste_burst_if_due(&mut self, now: std::time::Instant) -> bool {
        let changed = self.composer.flush_paste_burst_if_due(now);
        if changed {
            self.sync_popups();
        }
        changed
    }

    pub fn handle_paste(&mut self, text: String) -> V3Action {
        let action = self.composer.handle_paste(text);
        self.sync_popups().unwrap_or(action)
    }

    pub fn draft(&self) -> &str {
        self.composer.text()
    }

    pub fn replace_draft(&mut self, text: &str) {
        self.composer.replace_text(text);
        self.sync_popups();
    }

    pub fn restore_submission(&mut self, text: &str, attachments: Vec<tui_contracts::event::V3Attachment>) {
        self.composer.restore_submission(text, attachments);
        self.sync_popups();
    }

    pub fn add_attachment(&mut self, attachment: tui_contracts::event::V3Attachment) -> V3Action {
        self.composer.add_attachment(attachment)
    }

    pub fn attachments(&self) -> &[tui_contracts::event::V3Attachment] {
        self.composer.attachments()
    }

    pub fn push_queued_input(&mut self, text: String) {
        self.pending_input_preview.queued_messages.push(text);
        self.composer.set_queued_count(self.pending_input_preview.queued_messages.len());
    }

    pub fn pop_queued_input(&mut self) {
        if !self.pending_input_preview.queued_messages.is_empty() {
            self.pending_input_preview.queued_messages.remove(0);
        }
        self.composer.set_queued_count(self.pending_input_preview.queued_messages.len());
    }

    pub fn pop_last_queued_input(&mut self) {
        self.pending_input_preview.queued_messages.pop();
        self.composer.set_queued_count(self.pending_input_preview.queued_messages.len());
    }

    pub fn set_command_palette_items(&mut self, items: Vec<V3SelectionItem>) {
        self.command_items = items;
        self.sync_popups();
    }

    pub fn set_file_search_matches(&mut self, query: &str, matches: Vec<V3FileSearchMatch>) {
        if let Some(popup) = self.file_search_popup.as_mut() {
            popup.set_matches(query, matches);
        }
    }

    pub fn set_file_search_error(&mut self, query: &str, message: String) {
        if let Some(popup) = self.file_search_popup.as_mut() {
            popup.set_error(query, message);
        }
    }

    pub fn file_search_visible(&self) -> bool {
        self.file_search_popup.is_some()
    }

    pub fn command_palette_visible(&self) -> bool {
        self.command_popup.is_some()
    }

    fn sync_command_popup(&mut self) {
        let text = self.composer.text();
        let first_line = text.lines().next().unwrap_or_default();
        let editing_command = first_line.starts_with('/')
            && !first_line.strip_prefix('/').unwrap_or_default().chars().any(char::is_whitespace);
        if !editing_command || self.command_items.is_empty() {
            self.command_popup = None;
            return;
        }

        let mut popup = command_popup::CommandPopup::new(self.command_items.clone());
        popup.on_composer_text_change(first_line.to_string());
        self.command_popup = Some(popup);
    }

    fn sync_popups(&mut self) -> Option<V3Action> {
        let query = self.composer.active_file_search_query().map(str::to_owned);
        if query == self.file_search_query {
            if query.is_some() {
                self.command_popup = None;
            } else {
                self.sync_command_popup();
            }
            return None;
        }

        self.file_search_query.clone_from(&query);
        match query.as_deref() {
            Some(query) => {
                self.command_popup = None;
                let popup = self
                    .file_search_popup
                    .get_or_insert_with(file_search_popup::FileSearchPopup::new);
                popup.set_query(query);
            }
            None => {
                self.file_search_popup = None;
                self.sync_command_popup();
            }
        }
        Some(V3Action::FileSearchQueryChanged { query })
    }

    fn close_file_search(&mut self) {
        self.file_search_query = None;
        self.file_search_popup = None;
    }

    fn layout_areas(&self, area: Rect) -> [Rect; 3] {
        let preview_height = self.pending_input_preview.desired_height(area.width);
        let popup_height = self
            .command_popup
            .as_ref()
            .map(|popup| popup.calculate_required_height(area.width))
            .or_else(|| {
                self.file_search_popup
                    .as_ref()
                    .map(file_search_popup::FileSearchPopup::calculate_required_height)
            })
            .unwrap_or_default();
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(preview_height),
                Constraint::Length(popup_height),
                Constraint::Min(1),
            ])
            .split(area);
        [areas[0], areas[1], areas[2]]
    }
}

impl Renderable for BottomPane {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let [preview_area, popup_area, composer_area] = self.layout_areas(area);
        if preview_area.height > 0 {
            self.pending_input_preview.render(preview_area, buf);
        }
        if let Some(popup) = self.command_popup.as_ref()
            && popup_area.height > 0
        {
            WidgetRef::render_ref(&popup, popup_area, buf);
        } else if let Some(popup) = self.file_search_popup.as_ref()
            && popup_area.height > 0
        {
            WidgetRef::render_ref(&popup, popup_area, buf);
        }
        self.composer.render(composer_area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.pending_input_preview.desired_height(width)
            + self
                .command_popup
                .as_ref()
                .map(|popup| popup.calculate_required_height(width))
                .or_else(|| {
                    self.file_search_popup
                        .as_ref()
                        .map(file_search_popup::FileSearchPopup::calculate_required_height)
                })
                .unwrap_or_default()
            + self.composer.desired_height(width)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        let [_preview_area, _popup_area, composer_area] = self.layout_areas(area);
        self.composer.cursor_pos(composer_area)
    }
}
