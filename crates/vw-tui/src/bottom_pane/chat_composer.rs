//! 本模块集中实现 `chat_composer` 在底部面板中的职责。
//! 关键声明 `PROMPT_PREFIX`、`ChatComposer`、`new`、`with_data` 共同界定本文件的对外交互面。
//! 状态读取与更新均沿用既有类型，不在渲染路径中引入额外业务副作用。
//! 交互分支通过明确枚举或返回值向上层传递选择结果。
//! 空数据、越界索引与不可用选项按现有分支保持可预期的回退。
//! 布局计算以当前可用区域为边界，避免向小终端尺寸外绘制。


//! VibeWindow 输入编辑器：通过显式行为边界复制了文本域的行为。编辑、Unicode 换行、Vim 模式、粘贴突发和本地草稿历史均保留在 UI 中。Enter、Tab、Esc、中断、历史记录和附件操作返回外部桥接的 [`V3Action`]；该模块从未发送应用事件或查找运行时状态。
//! The VibeWindow composer: copied textarea behavior with an explicit action boundary.
//!
//! Text editing, Unicode wrapping, Vim mode, paste bursts, and local draft history remain local
//! to the UI.  Enter, Tab, Esc, interrupt, history, and attachment operations return
//! [`V3Action`] for the outer bridge; this module never sends an app event or looks up runtime
//! state.

use std::cell::RefCell;

use crate::bottom_pane::chat_composer_history::{ChatComposerHistory, HistoryEntry};
use crate::bottom_pane::footer::{self, FooterMode, FooterProps};
use crate::bottom_pane::footer_state::FooterState;
use crate::bottom_pane::textarea::{TextArea, TextAreaState};
use crate::render::renderable::Renderable;
use crate::vibe::action::{V3Action, V3FooterData};
use crate::vibe::event::V3Attachment;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, StatefulWidgetRef, Widget};

const PROMPT_PREFIX: &str = "› ";

/// 一个没有请求发送器或运行时依赖的本地输入编辑器。
/// A local composer with no request sender or runtime dependency.
pub(crate) struct ChatComposer {
    textarea: TextArea,
    textarea_state: RefCell<TextAreaState>,
    history: ChatComposerHistory,
    footer: FooterState,
    footer_data: V3FooterData,
    placeholder: String,
    attachments: Vec<V3Attachment>,
    queued_count: usize,
}

impl ChatComposer {
    pub(crate) fn new() -> Self {
        Self::with_data("Compose a VibeWindow task".to_string(), V3FooterData::default())
    }

    pub(crate) fn with_data(placeholder: String, footer_data: V3FooterData) -> Self {
        Self {
            textarea: TextArea::new(),
            textarea_state: RefCell::new(TextAreaState::default()),
            history: ChatComposerHistory::new(),
            footer: FooterState::new(),
            footer_data,
            placeholder,
            attachments: Vec::new(),
            queued_count: 0,
        }
    }

    pub(crate) fn set_footer_data(&mut self, footer_data: V3FooterData) {
        self.footer_data = footer_data;
    }

    pub(crate) fn footer_data(&self) -> &V3FooterData {
        &self.footer_data
    }

    pub(crate) fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = placeholder.into();
    }

    pub(crate) fn text(&self) -> &str {
        self.textarea.text()
    }

    pub(crate) fn replace_text(&mut self, text: &str) {
        self.textarea.set_text_clearing_elements(text);
        self.textarea.set_cursor(text.len());
        self.footer.mode = if text.is_empty() {
            FooterMode::ComposerEmpty
        } else {
            FooterMode::ComposerHasDraft
        };
    }

    pub(crate) fn restore_submission(
        &mut self,
        text: &str,
        attachments: Vec<V3Attachment>,
    ) {
        self.replace_text(text);
        self.attachments = attachments;
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.textarea.is_empty()
    }

    pub(crate) fn attachments(&self) -> &[V3Attachment] {
        &self.attachments
    }

    pub(crate) fn set_task_running(&mut self, running: bool) {
        self.footer_data.is_task_running = running;
    }

    pub(crate) fn set_queued_count(&mut self, queued_count: usize) {
        self.queued_count = queued_count;
    }

    pub(crate) fn add_attachment(&mut self, attachment: V3Attachment) -> V3Action {
        self.attachments.push(attachment.clone());
        V3Action::Attach(attachment)
    }

    pub(crate) fn remove_attachment(&mut self, index: usize) -> V3Action {
        if index < self.attachments.len() {
            self.attachments.remove(index);
            V3Action::RemoveAttachment { index }
        } else {
            V3Action::None
        }
    }

    pub(crate) fn clear(&mut self) {
        self.textarea.set_text_clearing_elements("");
        self.attachments.clear();
        self.footer.mode = FooterMode::ComposerEmpty;
        self.footer.shortcuts_visible = false;
    }

    /// 本地处理粘贴。粘贴的内容没有业务影响。
    /// Handle a paste locally.  Pasted text does not have a business side effect.
    pub(crate) fn handle_paste(&mut self, pasted: String) -> V3Action {
        if !pasted.is_empty() {
            self.textarea.insert_str(&pasted);
            self.footer.mode = FooterMode::ComposerHasDraft;
        }
        V3Action::None
    }

    fn submit_or_queue(&mut self, queue: bool) -> V3Action {
        if self.textarea.is_empty() && self.attachments.is_empty() {
            return V3Action::None;
        }
        let text = self.textarea.text().trim().to_string();
        if text.is_empty() && self.attachments.is_empty() {
            return V3Action::None;
        }
        self.history.record_local_submission(HistoryEntry::new(text.clone()));
        let attachments = std::mem::take(&mut self.attachments);
        self.textarea.set_text_clearing_elements("");
        self.footer.mode = FooterMode::ComposerEmpty;
        if queue {
            V3Action::Queue { text, attachments }
        } else {
            V3Action::Submit { text, attachments }
        }
    }

    fn restore_history_previous(&mut self) -> bool {
        let Some(entry) = self.history.navigate_up(self.textarea.text()) else {
            return false;
        };
        self.textarea.set_text_clearing_elements(&entry.text);
        self.textarea.set_cursor(self.textarea.text().len());
        true
    }

    fn restore_history_next(&mut self) -> bool {
        let Some(entry) = self.history.navigate_down() else {
            return false;
        };
        self.textarea.set_text_clearing_elements(&entry.text);
        self.textarea.set_cursor(self.textarea.text().len());
        true
    }

    /// 将终端的一个键转换为本地编辑或显式的 V3 操作。
    /// Turn one terminal key into a local edit or an explicit V3 action.
    pub(crate) fn handle_key_event(&mut self, event: KeyEvent) -> V3Action {
        if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return V3Action::None;
        }

        if event.code == KeyCode::Char('c') && event.modifiers.contains(KeyModifiers::CONTROL) {
            return if self.footer_data.is_task_running {
                V3Action::Interrupt
            } else {
                self.clear();
                V3Action::Cancel
            };
        }
        if event.code == KeyCode::Esc {
            return if self.footer_data.is_task_running {
                V3Action::Interrupt
            } else {
                self.clear();
                V3Action::Cancel
            };
        }
        if event.code == KeyCode::Enter {
            if event
                .modifiers
                .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                self.textarea.input(event);
                self.footer.mode = FooterMode::ComposerHasDraft;
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
            self.footer.mode = FooterMode::ComposerHasDraft;
            return V3Action::HistoryPrevious;
        }
        if event.code == KeyCode::Down
            && self.textarea.cursor() == self.textarea.text().len()
            && self.restore_history_next()
        {
            self.footer.mode = if self.is_empty() {
                FooterMode::ComposerEmpty
            } else {
                FooterMode::ComposerHasDraft
            };
            return V3Action::HistoryNext;
        }
        if event.code == KeyCode::Char('?')
            && !event.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && self.is_empty()
        {
            self.footer.shortcuts_visible = !self.footer.shortcuts_visible;
            self.footer.mode = if self.footer.shortcuts_visible {
                FooterMode::ShortcutOverlay
            } else {
                FooterMode::ComposerEmpty
            };
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
            self.footer.mode = FooterMode::ComposerHasDraft;
        } else if !self.footer.shortcuts_visible {
            self.footer.mode = FooterMode::ComposerEmpty;
        }
        V3Action::None
    }

    fn footer_props(&self) -> FooterProps {
        FooterProps {
            mode: if self.footer_data.is_task_running && self.is_empty() {
                FooterMode::Working
            } else {
                self.footer.mode
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
mod tests {
    use super::*;
    use crate::vibe::action::V3Action;
    use insta::assert_snapshot;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn render(composer: &ChatComposer, width: u16) -> String {
        let height = composer.desired_height(width).max(1);
        let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
        composer.render(Rect::new(0, 0, width, height), &mut buf);
        format!("{buf:?}")
    }

    // 验证 `v3_composer_emits_actions_without_owning_runtime` 场景的触发条件与预期结果。
    #[test]
    fn v3_composer_emits_actions_without_owning_runtime() {
        let mut composer = ChatComposer::new();
        composer.handle_key_event(key(KeyCode::Char('h')));
        composer.handle_key_event(key(KeyCode::Char('i')));
        assert_eq!(
            composer.handle_key_event(key(KeyCode::Enter)),
            V3Action::Submit { text: "hi".into(), attachments: Vec::new() }
        );

        composer.set_task_running(true);
        assert_eq!(composer.handle_key_event(key(KeyCode::Char('c'))), V3Action::None);
        assert_eq!(
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            V3Action::Interrupt
        );
    }

    // 验证 `composer_footer_snapshots` 场景的触发条件与预期结果。
    #[test]
    fn composer_footer_snapshots() {
        let empty = ChatComposer::new();
        assert_snapshot!("empty_40", render(&empty, 40));

        let mut multiline = ChatComposer::new();
        multiline.handle_paste("one\ntwo\nthree".into());
        assert_snapshot!("multiline_80", render(&multiline, 80));

        let mut working = ChatComposer::new();
        working.set_task_running(true);
        assert_snapshot!("working_120", render(&working, 120));

        let mut queued = ChatComposer::new();
        queued.set_task_running(true);
        queued.handle_paste("queued".into());
        assert_snapshot!("queued_80", render(&queued, 80));

        assert_snapshot!("narrow_40", render(&empty, 12));
    }
}
