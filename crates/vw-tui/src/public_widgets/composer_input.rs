//! Public wrapper around the internal ChatComposer for simple, reusable text input.
//! 面向外部调用者封装内部 ChatComposer，提供简单可复用的文本输入。
//!
//! This exposes a minimal interface suitable for other crates (e.g.,
//! codex-cloud-tasks) to reuse the mature composer behavior: multi-line input,
//! paste heuristics, Enter-to-submit, and Shift+Enter for newline.
//! 该最小接口供其他 crate 复用成熟的编辑器行为，包括多行输入、粘贴启发式、Enter 提交
//! 与 Shift+Enter 换行。

use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use std::time::Duration;

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::bottom_pane::ChatComposer;
use crate::bottom_pane::InputResult;
use crate::bottom_pane::QueuedInputAction;
use crate::render::renderable::Renderable;

/// Action returned from feeding a key event into the ComposerInput.
/// 向 ComposerInput 输入按键事件后返回的高层动作。
pub enum ComposerAction {
    /// The user submitted the current text (typically via Enter). Contains the submitted text.
    /// 用户通常通过 Enter 提交当前文本；变体内携带提交内容。
    Submitted(String),
    /// No submission occurred; UI may need to redraw if `needs_redraw()` returned true.
    /// 未发生提交；若底层状态变化，UI 仍可能需要重绘。
    None,
}

/// A minimal, public wrapper for the internal `ChatComposer` that behaves as a
/// reusable text input field with submit semantics.
/// 内部 `ChatComposer` 的最小公开封装，表现为带提交语义的可复用文本输入框。
pub struct ComposerInput {
    inner: ChatComposer,
    _tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    rx: tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
}

impl ComposerInput {
    /// Create a new composer input with a neutral placeholder.
    /// 使用中性占位文案创建新的输入框。
    pub fn new() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let sender = AppEventSender::new(tx.clone());
        // `enhanced_keys_supported=true` enables Shift+Enter newline hint/behavior.
        // `enhanced_keys_supported=true` 用于启用 Shift+Enter 换行提示与行为。
        // 以下位置参数分别保持输入焦点、增强键支持和粘贴突发检测处于公共控件所需状态。
        let inner = ChatComposer::new(
            /*has_input_focus*/ true,
            sender,
            /*enhanced_keys_supported*/ true,
            "Compose new task".to_string(),
            /*disable_paste_burst*/ false,
        );
        Self { inner, _tx: tx, rx }
    }

    /// Returns true if the input is empty.
    /// 返回输入内容是否为空。
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Clear the input text.
    /// 清空输入文本及其关联元素。
    pub fn clear(&mut self) {
        self.inner
            .set_text_content(String::new(), Vec::new(), Vec::new());
    }

    /// Feed a key event into the composer and return a high-level action.
    /// 将按键事件交给编辑器，并返回提交或无提交的高层动作。
    pub fn input(&mut self, key: KeyEvent) -> ComposerAction {
        let action = match self.inner.handle_key_event(key).0 {
            InputResult::Submitted { text, .. } => ComposerAction::Submitted(text),
            InputResult::Queued {
                text,
                text_elements,
                action: QueuedInputAction::Literal,
                pending_pastes,
            } => ComposerAction::Submitted(
                ChatComposer::expand_pending_pastes(&text, text_elements, &pending_pastes).0,
            ),
            _ => ComposerAction::None,
        };
        self.drain_app_events();
        action
    }

    pub fn handle_paste(&mut self, pasted: String) -> bool {
        // 粘贴后同步清空内部通知事件，公共包装层不向调用者暴露应用事件通道。
        let handled = self.inner.handle_paste(pasted);
        self.drain_app_events();
        handled
    }

    /// Override the footer hint items displayed under the composer.
    /// Each tuple is rendered as "<key> <label>", with keys styled.
    /// 覆盖编辑器底部展示的提示项。
    /// 每个二元组按“按键 标签”渲染，其中按键使用专门样式。
    pub fn set_hint_items(&mut self, items: Vec<(impl Into<String>, impl Into<String>)>) {
        let mapped: Vec<(String, String)> = items
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self.inner.set_footer_hint_override(Some(mapped));
    }

    /// Clear any previously set custom hint items and restore the default hints.
    /// 清除自定义提示项并恢复默认提示。
    pub fn clear_hint_items(&mut self) {
        // items 参数为空表示撤销覆盖并恢复默认提示。
        self.inner.set_footer_hint_override(/*items*/ None);
    }

    /// Desired height (in rows) for a given width.
    /// 返回给定宽度下期望占用的行数。
    pub fn desired_height(&self, width: u16) -> u16 {
        self.inner.desired_height(width)
    }

    /// Compute the on-screen cursor position for the given area.
    /// 计算给定区域内的屏幕光标位置。
    pub fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.inner.cursor_pos(area)
    }

    /// Render the input into the provided buffer at `area`.
    /// 将输入框渲染到 `area` 对应的缓冲区区域。
    pub fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.inner.render(area, buf);
    }

    /// Return true if a paste-burst detection is currently active.
    /// 返回粘贴突发检测当前是否处于活动状态。
    pub fn is_in_paste_burst(&self) -> bool {
        self.inner.is_in_paste_burst()
    }

    /// Flush a pending paste-burst if the inter-key timeout has elapsed.
    /// Returns true if text changed and a redraw is warranted.
    /// 若按键间隔已超时则刷新待处理的粘贴突发。
    /// 文本发生变化且值得重绘时返回 true。
    pub fn flush_paste_burst_if_due(&mut self) -> bool {
        let flushed = self.inner.flush_paste_burst_if_due();
        self.drain_app_events();
        flushed
    }

    /// Recommended delay to schedule the next micro-flush frame while a
    /// paste-burst is active.
    /// 粘贴突发活动期间，调度下一次微刷新帧的建议延迟。
    pub fn recommended_flush_delay() -> Duration {
        crate::bottom_pane::ChatComposer::recommended_paste_flush_delay()
    }

    fn drain_app_events(&mut self) {
        // 公共控件只暴露文本输入契约，丢弃内部编辑器为完整 TUI 生成的旁路事件。
        while self.rx.try_recv().is_ok() {}
    }
}

impl Default for ComposerInput {
    fn default() -> Self {
        Self::new()
    }
}
