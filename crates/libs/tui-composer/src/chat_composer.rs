//! 本地输入组合器，返回 [`V3Action`]，不持有请求发送器或运行时。
//! 普通 Enter 提交非空草稿或附件；任务运行时 Tab 将其加入队列。
//! 带 Shift、Control 或 Alt 的 Enter 交给 TextArea 编辑并返回 InsertNewline；
//! 实际换行遵循 TextArea 的按键映射与 Vim 模式。历史导航、附件、光标处
//! `@token` 替换及失败提交恢复均保留在本地。
//!
//! 快速键流复用 PasteBurst 缓冲；粘贴窗口中的 Enter 插入换行。
//! 调用者须在空闲时调用 flush_paste_burst_if_due 刷新悬挂输入。

use std::cell::RefCell;
use std::ops::Range;
use std::time::Instant;
use tui_input::paste_burst::{CharDecision, FlushResult, PasteBurst};

use crate::chat_composer_history::{
    ChatComposerHistory, HistoryEntry, HistorySearchDirection, HistorySearchResult,
};
use crate::footer::{self, FooterMode, FooterProps};
use crate::footer_state::FooterState;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, StatefulWidgetRef, Widget};
use tui_contracts::action::{V3Action, V3FooterData};
use tui_contracts::event::V3Attachment;
use tui_input::textarea::{TextArea, TextAreaState};
use tui_render::renderable::Renderable;

const LARGE_PASTE_CHAR_THRESHOLD: usize = 1000;

const PROMPT_PREFIX: &str = "› ";

struct HistorySearchSession {
    original: HistoryEntry,
    query: String,
    matched: bool,
}

/// 一个没有请求发送器或运行时依赖的本地输入编辑器。
/// A local composer with no request sender or runtime dependency.
pub struct ChatComposer {
    textarea: TextArea,
    textarea_state: RefCell<TextAreaState>,
    history: ChatComposerHistory,
    history_search: Option<HistorySearchSession>,
    footer: FooterState,
    footer_data: V3FooterData,
    placeholder: String,
    attachments: Vec<V3Attachment>,
    queued_count: usize,
    paste_burst: PasteBurst,
}

impl ChatComposer {
    pub fn new() -> Self {
        Self::with_data("Compose a ShiJu task".to_string(), V3FooterData::default())
    }

    pub fn with_data(placeholder: String, footer_data: V3FooterData) -> Self {
        Self {
            textarea: TextArea::new(),
            textarea_state: RefCell::new(TextAreaState::default()),
            history: ChatComposerHistory::new(),
            history_search: None,
            footer: FooterState::new(),
            footer_data,
            placeholder,
            attachments: Vec::new(),
            queued_count: 0,
            paste_burst: PasteBurst::default(),
        }
    }

    pub fn set_footer_data(&mut self, footer_data: V3FooterData) {
        self.footer_data = footer_data;
    }

    pub fn footer_data(&self) -> &V3FooterData {
        &self.footer_data
    }

    /// Vim is a local editor preference; changing it preserves the draft.
    pub fn set_vim_enabled(&mut self, enabled: bool) {
        if self.textarea.is_vim_enabled() != enabled {
            self.flush_pending_input();
            self.paste_burst.clear_after_explicit_paste();
            self.textarea.set_vim_enabled(enabled);
        }
    }

    pub fn is_vim_enabled(&self) -> bool {
        self.textarea.is_vim_enabled()
    }

    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = placeholder.into();
    }

    /// Visible editor text, including folded paste labels.
    pub fn text(&self) -> &str {
        self.textarea.text()
    }

    /// Original draft for external editors and other consumers of user content.
    pub fn expanded_text(&self) -> String {
        self.textarea.expanded_text()
    }

    pub fn replace_text(&mut self, text: &str) {
        self.history_search = None;
        self.history.reset_navigation();
        self.paste_burst.clear_after_explicit_paste();
        self.textarea.set_text_clearing_elements(text);
        self.textarea.set_cursor(text.len());
        self.footer.set_mode(if text.is_empty() {
            FooterMode::ComposerEmpty
        } else {
            FooterMode::ComposerHasDraft
        });
    }

    /// Returns the editable file-search query immediately before the cursor.
    pub fn active_file_search_query(&self) -> Option<&str> {
        if self.history_search.is_some() {
            return None;
        }
        let range = self.file_search_token_range()?;
        self.textarea.text().get(range.start + '@'.len_utf8()..range.end)
    }

    /// Replaces the active `@token` without touching text after the cursor.
    pub fn replace_file_search_token(&mut self, path: &str) -> bool {
        let Some(range) = self.file_search_token_range() else {
            return false;
        };
        let normalized_path = path.replace('\\', "/");
        self.textarea.replace_range(range, &format!("@{normalized_path} "));
        self.footer.set_mode(FooterMode::ComposerHasDraft);
        true
    }

    fn file_search_token_range(&self) -> Option<Range<usize>> {
        let cursor = self.textarea.cursor();
        let before_cursor = self.textarea.text().get(..cursor)?;
        let token_start = before_cursor
            .char_indices()
            .filter_map(|(index, character)| {
                character.is_whitespace().then_some(index + character.len_utf8())
            })
            .next_back()
            .unwrap_or_default();
        let token = &before_cursor[token_start..];
        let at_offset = token.rfind('@')?;
        let query = &token[at_offset + '@'.len_utf8()..];
        query.chars().all(is_file_search_char).then_some(token_start + at_offset..cursor)
    }

    pub fn restore_submission(&mut self, text: &str, attachments: Vec<V3Attachment>) {
        // Queue edits and failed sends carry expanded wire text. Reuse the local
        // snapshot only when both content and ordered attachments still match.
        let entry = self
            .history
            .entries()
            .iter()
            .rev()
            .find(|entry| entry.text == text && entry.attachments == attachments)
            .cloned();
        self.history.reset_navigation();
        if let Some(entry) = entry {
            self.restore_history_entry(entry);
        } else {
            self.replace_text(text);
            self.attachments = attachments;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.textarea.is_empty()
    }

    pub fn attachments(&self) -> &[V3Attachment] {
        &self.attachments
    }

    pub fn set_task_running(&mut self, running: bool) {
        self.footer_data.is_task_running = running;
    }

    pub fn set_queued_count(&mut self, queued_count: usize) {
        self.queued_count = queued_count;
    }

    pub fn add_attachment(&mut self, attachment: V3Attachment) -> V3Action {
        self.attachments.push(attachment.clone());
        V3Action::Attach(attachment)
    }

    pub fn remove_attachment(&mut self, index: usize) -> V3Action {
        if index < self.attachments.len() {
            self.attachments.remove(index);
            V3Action::RemoveAttachment { index }
        } else {
            V3Action::None
        }
    }

    pub fn clear(&mut self) {
        self.history_search = None;
        self.history.reset_navigation();
        self.paste_burst.clear_after_explicit_paste();
        self.textarea.set_text_clearing_elements("");
        self.attachments.clear();
        self.footer.set_mode(FooterMode::ComposerEmpty);
        self.footer.set_shortcuts_visible(false);
    }

    /// 本地处理粘贴。粘贴的内容没有业务影响。
    /// Handle a paste locally.  Pasted text does not have a business side effect.
    pub fn handle_paste(&mut self, pasted: String) -> V3Action {
        self.flush_pending_input();
        self.paste_burst.clear_after_explicit_paste();
        if let Some(search) = &mut self.history_search {
            search.query.push_str(&pasted);
            self.update_history_search(true);
        } else {
            self.insert_pasted_text(&pasted);
        }
        V3Action::None
    }

    fn insert_pasted_text(&mut self, pasted: &str) {
        if !pasted.is_empty() {
            let chars = pasted.chars().count();
            if chars > LARGE_PASTE_CHAR_THRESHOLD {
                self.textarea
                    .insert_element_with_payload(&format!("[Pasted text: {chars} chars]"), pasted);
            } else {
                self.textarea.insert_str(pasted);
            }
            self.footer.set_mode(FooterMode::ComposerHasDraft);
        }
    }

    /// Deadline for the host event loop to wake without another keypress.
    pub fn next_paste_flush_deadline(&self) -> Option<Instant> {
        self.paste_burst.next_flush_deadline()
    }

    /// Flush a pending key or paste without resetting the Enter protection window.
    pub fn flush_paste_burst_if_due(&mut self, now: Instant) -> bool {
        match self.paste_burst.flush_if_due(now) {
            FlushResult::Typed(ch) => {
                self.insert_pasted_text(&ch.to_string());
                true
            }
            FlushResult::Paste(text) => {
                self.insert_pasted_text(&text);
                true
            }
            FlushResult::None => false,
        }
    }

    fn flush_pending_input(&mut self) {
        if let Some(text) = self.paste_burst.flush_before_modified_input() {
            self.insert_pasted_text(&text);
        }
    }

    fn submit_or_queue(&mut self, queue: bool) -> V3Action {
        if self.textarea.is_empty() && self.attachments.is_empty() {
            return V3Action::None;
        }
        let text = self.textarea.expanded_text();
        if text.trim().is_empty() && self.attachments.is_empty() {
            return V3Action::None;
        }
        self.history.record_local_submission(self.history_entry());
        let attachments = std::mem::take(&mut self.attachments);
        self.textarea.set_text_clearing_elements("");
        self.footer.set_mode(FooterMode::ComposerEmpty);
        if queue {
            V3Action::Queue { text, attachments }
        } else {
            V3Action::Submit { text, attachments }
        }
    }

    fn history_entry(&self) -> HistoryEntry {
        let mut entry = HistoryEntry::new(self.textarea.expanded_text());
        entry.attachments = self.attachments.clone();
        entry.draft = Some(self.textarea.draft());
        entry
    }

    fn restore_history_entry(&mut self, entry: HistoryEntry) {
        self.paste_burst.clear_after_explicit_paste();
        if let Some(draft) = entry.draft {
            self.textarea.restore_draft(&draft);
        } else {
            self.textarea.set_text_clearing_elements(&entry.text);
            self.textarea.set_cursor(self.textarea.text().len());
        }
        self.attachments = entry.attachments;
        self.footer.set_mode(if self.is_empty() && self.attachments.is_empty() {
            FooterMode::ComposerEmpty
        } else {
            FooterMode::ComposerHasDraft
        });
    }

    fn restore_history_previous(&mut self) -> bool {
        let Some(entry) = self.history.navigate_up_with_draft(self.history_entry()) else {
            return false;
        };
        self.restore_history_entry(entry);
        true
    }

    fn restore_history_next(&mut self) -> bool {
        let Some(entry) = self.history.navigate_down() else {
            return false;
        };
        self.restore_history_entry(entry);
        true
    }

    fn update_history_search(&mut self, restart: bool) {
        let query = self.history_search.as_ref().unwrap().query.clone();
        let result = self.history.search(&query, HistorySearchDirection::Older, restart);
        match result {
            HistorySearchResult::Found(entry) => {
                self.restore_history_entry(entry);
                self.history_search.as_mut().unwrap().matched = true;
            }
            HistorySearchResult::NotFound | HistorySearchResult::AtBoundary => {
                // Keep the last preview and the query even when traversal runs out.
                self.history_search.as_mut().unwrap().matched = false;
            }
        }
    }

    fn handle_history_search_key(&mut self, event: KeyEvent) -> V3Action {
        match (event.code, event.modifiers) {
            (KeyCode::Esc, _) => {
                let search = self.history_search.take().unwrap();
                self.restore_history_entry(search.original);
                self.history.reset_navigation();
            }
            (KeyCode::Enter, _) => {
                self.history_search = None;
                self.history.reset_navigation();
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => self.update_history_search(false),
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                self.history_search.as_mut().unwrap().query.pop();
                self.update_history_search(true);
            }
            (KeyCode::Char(ch), modifiers)
                if !modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER
                        | KeyModifiers::HYPER | KeyModifiers::META,
                ) => {
                self.history_search.as_mut().unwrap().query.push(ch);
                self.update_history_search(true);
            }
            _ => {}
        }
        V3Action::None
    }

    /// 将终端的一个键转换为本地编辑或显式的 V3 操作。
    /// Turn one terminal key into a local edit or an explicit V3 action.
    pub fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        self.handle_key_event_at(event, Instant::now())
    }

    /// Handle input at a supplied monotonic time, shared with idle flushing.
    pub fn handle_key_event_at(&mut self, event: KeyEvent, now: Instant) -> V3Action {
        if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return V3Action::None;
        }

        self.flush_paste_burst_if_due(now);

        if self.history_search.is_some()
            && !(event.code == KeyCode::Char('c') && event.modifiers == KeyModifiers::CONTROL)
        {
            return self.handle_history_search_key(event);
        }
        if event.code == KeyCode::Char('r') && event.modifiers == KeyModifiers::CONTROL {
            self.flush_pending_input();
            self.paste_burst.clear_after_explicit_paste();
            self.history.reset_navigation();
            self.history_search = Some(HistorySearchSession {
                original: self.history_entry(),
                query: String::new(),
                matched: false,
            });
            self.update_history_search(true);
            return V3Action::None;
        }

        if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(search) = self.history_search.take() {
                self.restore_history_entry(search.original);
                self.history.reset_navigation();
            }
            return if self.footer_data.is_task_running {
                self.flush_pending_input();
                self.paste_burst.clear_after_explicit_paste();
                V3Action::Interrupt
            } else {
                self.clear();
                V3Action::Cancel
            };
        }
        if event.code == KeyCode::Esc && self.textarea.is_vim_enabled() {
            self.flush_pending_input();
            self.paste_burst.clear_after_explicit_paste();
            self.textarea.input(event);
            return V3Action::None;
        }
        // Normal-mode commands must never be buffered as pasted text or
        // interpreted as history/help shortcuts. Enter retains submit policy.
        if self.textarea.is_vim_normal_mode()
            && event.code != KeyCode::Enter
            && !(event.code == KeyCode::Tab && self.footer_data.is_task_running)
        {
            self.flush_pending_input();
            self.paste_burst.clear_after_explicit_paste();
            self.textarea.input(event);
            return V3Action::None;
        }
        if event.code == KeyCode::Esc {
            return if self.footer_data.is_task_running {
                self.flush_pending_input();
                self.paste_burst.clear_after_explicit_paste();
                V3Action::Interrupt
            } else {
                self.clear();
                V3Action::Cancel
            };
        }
        let text_modifiers = !event.modifiers.intersects(
            KeyModifiers::CONTROL
                | KeyModifiers::ALT
                | KeyModifiers::SUPER
                | KeyModifiers::HYPER
                | KeyModifiers::META,
        );
        if event.code == KeyCode::Enter
            && text_modifiers
            && self.paste_burst.newline_should_insert_instead_of_submit(now)
        {
            self.flush_pending_input();
            self.insert_pasted_text("\n");
            self.paste_burst.extend_window(now);
            return V3Action::InsertNewline;
        }
        if let KeyCode::Char(ch) = event.code {
            // Keep the empty-draft help shortcut immediate.
            if text_modifiers && !(ch == '?' && self.is_empty() && !self.paste_burst.is_active()) {
                let decision = if ch.is_ascii() {
                    Some(self.paste_burst.on_plain_char(ch, now))
                } else {
                    if self.paste_burst.try_append_char_if_active(ch, now) {
                        return V3Action::None;
                    }
                    self.flush_pending_input();
                    self.paste_burst.on_plain_char_no_hold(now)
                };
                match decision {
                    Some(CharDecision::RetainFirstChar) => return V3Action::None,
                    Some(CharDecision::BufferAppend | CharDecision::BeginBufferFromPending) => {
                        self.paste_burst.append_char_to_buffer(ch, now);
                        return V3Action::None;
                    }
                    Some(CharDecision::BeginBuffer { retro_chars }) => {
                        let cursor = self.textarea.cursor();
                        if let Some(grab) = self.paste_burst.decide_begin_buffer(
                            now,
                            &self.textarea.text()[..cursor],
                            usize::from(retro_chars),
                        ) {
                            if self
                                .textarea
                                .text_element_ranges()
                                .any(|range| range.start < cursor && range.end > grab.start_byte)
                            {
                                self.paste_burst.clear_after_explicit_paste();
                            } else {
                                self.textarea.replace_range(grab.start_byte..cursor, "");
                                self.paste_burst.append_char_to_buffer(ch, now);
                                return V3Action::None;
                            }
                        }
                    }
                    None => {}
                }
                self.textarea.input(event);
                self.footer.set_mode(FooterMode::ComposerHasDraft);
                return V3Action::None;
            }
        }
        self.flush_pending_input();
        self.paste_burst.clear_window_after_non_char();
        if event.code == KeyCode::Enter {
            if event
                .modifiers
                .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                if self.textarea.is_vim_normal_mode() {
                    self.textarea.insert_str("\n");
                } else {
                    self.textarea.input(event);
                }
                self.footer.set_mode(FooterMode::ComposerHasDraft);
                return V3Action::InsertNewline;
            }
            return self.submit_or_queue(false);
        }
        if event.code == KeyCode::Tab && self.footer_data.is_task_running {
            return self.submit_or_queue(true);
        }
        if event.code == KeyCode::Up
            && self.textarea.cursor() == 0
            && self.restore_history_previous()
        {
            self.footer.set_mode(FooterMode::ComposerHasDraft);
            return V3Action::HistoryPrevious;
        }
        if event.code == KeyCode::Down
            && self.textarea.cursor() == self.textarea.text().len()
            && self.restore_history_next()
        {
            self.footer.set_mode(if self.is_empty() {
                FooterMode::ComposerEmpty
            } else {
                FooterMode::ComposerHasDraft
            });
            return V3Action::HistoryNext;
        }
        if event.code == KeyCode::Char('?')
            && !event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && self.is_empty()
        {
            self.footer.set_shortcuts_visible(!self.footer.shortcuts_visible());
            self.footer.set_mode(if self.footer.shortcuts_visible() {
                FooterMode::ShortcutOverlay
            } else {
                FooterMode::ComposerEmpty
            });
            return V3Action::ToggleShortcuts;
        }
        if event.code == KeyCode::Char('g') && event.modifiers.contains(KeyModifiers::CONTROL) {
            return V3Action::OpenExternalEditor;
        }
        if event.code == KeyCode::Char('t') && event.modifiers.contains(KeyModifiers::CONTROL) {
            return V3Action::ShowTranscript;
        }

        self.textarea.input(event);
        if !self.is_empty() {
            self.footer.set_mode(FooterMode::ComposerHasDraft);
        } else if !self.footer.shortcuts_visible() {
            self.footer.set_mode(FooterMode::ComposerEmpty);
        }
        V3Action::None
    }

    fn footer_props(&self) -> FooterProps {
        FooterProps {
            mode: if self.history_search.is_some() {
                FooterMode::HistorySearch
            } else if self.footer_data.is_task_running && self.is_empty() {
                FooterMode::Working
            } else {
                self.footer.mode()
            },
            data: self.footer_data.clone(),
            queued_count: self.queued_count,
            attachment_count: self.attachments.len(),
        }
    }

    fn layout_areas(&self, area: Rect) -> (Rect, Rect) {
        let footer_height = footer::footer_height(&self.footer_props(), area.width);
        let areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(footer_height)])
            .split(area);
        (areas[0], areas[1])
    }
}

fn is_file_search_char(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(character, '/' | '\\' | '.' | '_' | '-' | ':' | '#')
}

impl Renderable for ChatComposer {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let (body, footer_area) = self.layout_areas(area);
        let text_area = Rect::new(
            body.x + 2.min(body.width),
            body.y,
            body.width.saturating_sub(2),
            body.height,
        );
        if self.is_empty() {
            Paragraph::new(Line::from(vec![
                Span::from(PROMPT_PREFIX).magenta(),
                Span::from(self.placeholder.clone()).dim(),
            ]))
            .render(body, buf);
        } else if !text_area.is_empty() {
            let mut state = self.textarea_state.borrow_mut();
            StatefulWidgetRef::render_ref(&&self.textarea, text_area, buf, &mut state);
            Paragraph::new(Span::from("›").magenta()).render(Rect::new(body.x, body.y, 1, 1), buf);
        }
        footer::render_footer(&self.footer_props(), footer_area, buf);
        if let Some(search) = &self.history_search {
            let status = if search.matched { "reverse search" } else { "no further match" };
            Paragraph::new(format!("  {status}: {}", search.query)).render(
                Rect::new(footer_area.x, footer_area.y + 1, footer_area.width,
                    footer_area.height.saturating_sub(1)), buf,
            );
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        let text_width = width.saturating_sub(2).max(1);
        let body_height =
            if self.is_empty() { 1 } else { self.textarea.desired_height(text_width).max(1) };
        body_height + footer::footer_height(&self.footer_props(), width)
    }

    fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        if area.is_empty() {
            return None;
        }
        let (body, _) = self.layout_areas(area);
        let text_area = Rect::new(
            body.x + 2.min(body.width),
            body.y,
            body.width.saturating_sub(2),
            body.height,
        );
        self.textarea.cursor_pos_with_state(text_area, *self.textarea_state.borrow())
    }
}

#[cfg(test)]
#[path = "chat_composer_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "terminal_req_01_tests.rs"]
mod terminal_req_01_tests;

#[cfg(test)]
#[path = "terminal_req_04_tests.rs"]
mod terminal_req_04_tests;

#[cfg(test)]
#[path = "terminal_req_05_tests.rs"]
mod terminal_req_05_tests;

#[cfg(test)]
#[path = "terminal_req_08_tests.rs"]
mod terminal_req_08_tests;
