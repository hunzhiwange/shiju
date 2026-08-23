//! 文本域拥有可编辑的 Composer 文本、占位符元素、光标/包裹状态以及单条目死缓冲。
//! The textarea owns editable composer text, placeholder elements, cursor/wrap state, and a
//! single-entry kill buffer.
//!
//! Whole-buffer replacement APIs intentionally rebuild only the visible draft state. They clear
//! element ranges and derived cursor/wrapping caches, but they keep the kill buffer intact so a
//! caller can clear or rewrite the draft and still allow `Ctrl+Y` to restore the user's most
//! recent `Ctrl+K`. This is the contract higher-level composer flows rely on after submit,
//! slash-command dispatch, and other synthetic clears.
//!
//! This module does not implement an Emacs-style multi-entry kill ring. It keeps only the most
//! recent killed span.
//!
//! Wrapping also reserves a visible insertion point: full logical lines get continuation rows,
//! and trailing spaces wrap instead of moving the cursor outside the textarea. At soft word
//! breaks, interior separators hang off the preceding row without changing the editable text.

use crate::input_types::ByteRange;
use crate::input_types::TextElement as UserTextElement;
use crate::key_hint::KeyBindingListExt;
use crate::key_hint::is_altgr;
use crate::keymap::EditorKeymap;
use crate::keymap::KeymapContext;
use crate::keymap::RuntimeKeymap;
use crate::keymap::VimNormalKeymap;
use crate::keymap::VimOperatorKeymap;
use crate::keymap::VimTextObjectKeymap;
use crate::width::display_width;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Span;
use ratatui::widgets::StatefulWidgetRef;
use ratatui::widgets::WidgetRef;
use std::borrow::Cow;
use std::cell::Ref;
use std::cell::RefCell;
use std::ops::Range;
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

#[path = "textarea/vim.rs"]
mod vim;
#[path = "textarea/vim_commands.rs"]
mod vim_commands;
#[path = "textarea/wrapping.rs"]
mod wrapping;
use self::vim::VimMode;
use self::vim::VimMotion;
use self::vim::VimOperator;
use self::vim::VimPending;
use self::vim::VimTextObjectScope;

const WORD_SEPARATORS: &str = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";

fn is_word_separator(ch: char) -> bool {
    WORD_SEPARATORS.contains(ch)
}

fn split_word_pieces(run: &str) -> Vec<(usize, &str)> {
    let mut pieces = Vec::new();
    for (segment_start, segment) in run.split_word_bound_indices() {
        let mut piece_start = 0;
        let mut chars = segment.char_indices();
        let Some((_, first_char)) = chars.next() else {
            continue;
        };
        let mut in_separator = is_word_separator(first_char);

        for (idx, ch) in chars {
            let is_separator = is_word_separator(ch);
            if is_separator == in_separator {
                continue;
            }
            pieces.push((segment_start + piece_start, &segment[piece_start..idx]));
            piece_start = idx;
            in_separator = is_separator;
        }

        pieces.push((segment_start + piece_start, &segment[piece_start..]));
    }

    pieces
}

/// 将换行符替换为用于渲染和缩进的一列表示法。 换行符与空格均为一个字节，因此基于此文本计算的区间仍指向原始可编辑文本。
/// Replace tabs with the one-column representation used for rendering and wrapping.
///
/// A tab and a space are both one byte, so ranges computed from this text still index the original
/// editable text.
fn text_for_display(text: &str) -> Cow<'_, str> {
    if text.contains('\t') { Cow::Owned(text.replace('\t', " ")) } else { Cow::Borrowed(text) }
}

#[derive(Debug, Clone)]
struct TextElement {
    id: u64,
    range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextElementSnapshot {
    pub(crate) id: u64,
    pub(crate) range: Range<usize>,
    pub(crate) text: String,
}

/// TextArea 是 TUI 编辑器后方的可编辑缓冲区。 它负责管理原始 UTF-8 文本、占位符样式的文本元素（这些元素必须与编辑原子性移动）、渲染所需的光标/换行状态，以及用于 `Ctrl+K` / `Ctrl+Y` 风格编辑的单条目死缓冲。调用者可以通过 [`Self::set_text_clearing_elements`] 或 [`Self::set_text_with_elements`] 方法完全替换可见缓冲区而不影响死缓冲；如果它们错误地假设这些方法完全重置编辑状态，随后的回滚将显示用户视角中残留的文本。
/// `TextArea` is the editable buffer behind the TUI composer.
///
/// It owns the raw UTF-8 text, placeholder-like text elements that must move atomically with
/// edits, cursor/wrapping state for rendering, and a single-entry kill buffer for `Ctrl+K` /
/// `Ctrl+Y` style editing. Callers may replace the entire visible buffer through
/// [`Self::set_text_clearing_elements`] or [`Self::set_text_with_elements`] without disturbing the
/// kill buffer; if they incorrectly assume those methods fully reset editing state, a later yank
/// will appear to restore stale text from the user's perspective.
#[derive(Debug)]
pub(crate) struct TextArea {
    text: String,
    cursor_pos: usize,
    wrap_cache: RefCell<Option<WrapCache>>,
    preferred_col: Option<usize>,
    elements: Vec<TextElement>,
    next_element_id: u64,
    kill_buffer: String,
    kill_buffer_kind: KillBufferKind,
    vim_enabled: bool,
    vim_mode: VimMode,
    vim_pending: VimPending,
    editor_keymap: Arc<EditorKeymap>,
    vim_normal_keymap: VimNormalKeymap,
    vim_operator_keymap: VimOperatorKeymap,
    vim_text_object_keymap: VimTextObjectKeymap,
}

#[derive(Debug, Clone)]
struct WrapCache {
    width: u16,
    lines: Vec<Range<usize>>,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TextAreaState {
    /// 索引第一行可见行的第一条内容
    /// Index into wrapped lines of the first visible line.
    scroll: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillBufferKind {
    /// 在光标处以字符级方式删除和撤销粘贴内容。
    /// Characterwise kills and yanks paste at the cursor.
    Characterwise,
    /// 当光标行下方时，将整行文本以 Linewise 的方式删除并粘贴。
    /// Linewise kills and yanks paste as whole lines below the cursor line.
    Linewise,
}

impl TextArea {
    pub fn new() -> Self {
        let defaults = RuntimeKeymap::defaults();
        Self {
            text: String::new(),
            cursor_pos: 0,
            wrap_cache: RefCell::new(None),
            preferred_col: None,
            elements: Vec::new(),
            next_element_id: 1,
            kill_buffer: String::new(),
            kill_buffer_kind: KillBufferKind::Characterwise,
            vim_enabled: false,
            vim_mode: VimMode::Insert,
            vim_pending: VimPending::None,
            editor_keymap: defaults.editor,
            vim_normal_keymap: defaults.vim_normal,
            vim_operator_keymap: defaults.vim_operator,
            vim_text_object_keymap: defaults.vim_text_object,
        }
    }

    /// 替换后续文本编辑所需的编辑器与 Vim 键映射。
    /// Replace the editor and Vim keymaps used by subsequent text-editing input.
    ///
    /// This method intentionally swaps only the keymap caches. It does not
    /// reinterpret pending input, change Vim mode, move the cursor, or mutate
    /// the kill buffer, so callers can safely apply a live config update while
    /// preserving the current draft exactly as typed.
    pub fn set_keymap_bindings(&mut self, keymap: &RuntimeKeymap) {
        self.editor_keymap = Arc::clone(&keymap.editor);
        self.vim_normal_keymap = keymap.vim_normal.clone();
        self.vim_operator_keymap = keymap.vim_operator.clone();
        self.vim_text_object_keymap = keymap.vim_text_object.clone();
    }

    /// 替换可见文本区域并清除现有元素。此路径专为需要纯文本且无占位符范围的用户调用而生，它故意保留当前死缓冲，因为高级流如提交或 SlashCommandDispatch 会在此方法中清除草稿，但仍希望 Ctrl+Y 以恢复用户最近的一次死亡。
    /// Replace the visible textarea text and clear any existing text elements.
    ///
    /// This is the "fresh buffer" path for callers that want plain text with no placeholder
    /// ranges. It intentionally preserves the current kill buffer, because higher-level flows such
    /// as submit or slash-command dispatch clear the draft through this method and still want
    /// `Ctrl+Y` to recover the user's most recent kill.
    pub fn set_text_clearing_elements(&mut self, text: &str) {
        self.set_text_inner(text, /*elements*/ None);
    }

    /// 将可见的文本域内容替换并重建提供的文本元素。
    /// Replace the visible textarea text and rebuild the provided text elements.
    ///
    /// As with [`Self::set_text_clearing_elements`], this resets only state derived from the
    /// visible buffer. The kill buffer survives so callers restoring drafts or external edits do
    /// not silently discard a pending yank target.
    pub fn set_text_with_elements(&mut self, text: &str, elements: &[UserTextElement]) {
        self.set_text_inner(text, Some(elements));
    }

    fn set_text_inner(&mut self, text: &str, elements: Option<&[UserTextElement]>) {
        // 阶段一：将原始文本替换，并确保光标保持在安全字节范围内。
        // Stage 1: replace the raw text and keep the cursor in a safe byte range.
        self.text = text.to_string();
        self.cursor_pos = self.cursor_pos.clamp(0, self.text.len());
        // 阶段二：从零开始重新构建元素范围，以更新文本内容。
        // Stage 2: rebuild element ranges from scratch against the new text.
        self.elements.clear();
        if let Some(elements) = elements {
            for elem in elements {
                let mut start = elem.byte_range.start.min(self.text.len());
                let mut end = elem.byte_range.end.min(self.text.len());
                start = self.clamp_pos_to_char_boundary(start);
                end = self.clamp_pos_to_char_boundary(end);
                if start >= end {
                    continue;
                }
                let id = self.next_element_id();
                self.elements.push(TextElement { id, range: start..end });
            }
            self.elements.sort_by_key(|e| e.range.start);
        }
        // 阶段三：将光标锁定并重置与先前内容相关的推导状态。杀缓冲用于编辑历史而非可见缓冲区状态，因此故意保留完整缓冲区替换行为。
        // Stage 3: clamp the cursor and reset derived state tied to the prior content.
        // The kill buffer is editing history rather than visible-buffer state, so full-buffer
        // replacements intentionally leave it alone.
        self.cursor_pos = self.clamp_pos_to_nearest_boundary(self.cursor_pos);
        self.wrap_cache.replace(None);
        self.preferred_col = None;
    }

    /// 启用或禁用文本域模态 Vim 编辑功能。 启用时始终进入正常模式，禁用时始终返回插入语义。待处理运算符在双向清除中均被清空，因此无法将下一个按键解释为旧命令 `d` 或 `y` 的后半部分。
    /// Enable or disable modal Vim editing for the textarea.
    ///
    /// Enabling always enters normal mode and disabling always returns to
    /// insert semantics. Pending operators are cleared in both directions so a
    /// toggle cannot leave the next keypress interpreted as the second half of
    /// an old `d` or `y` command.
    pub(crate) fn set_vim_enabled(&mut self, enabled: bool) {
        self.vim_enabled = enabled;
        self.vim_pending = VimPending::None;
        self.vim_mode = if enabled { VimMode::Normal } else { VimMode::Insert };
    }

    /// 是否当前启用了模态 Vim 编辑模式。
    /// Return whether modal Vim editing is currently enabled.
    pub(crate) fn is_vim_enabled(&self) -> bool {
        self.vim_enabled
    }

    /// 判断 Vim 模式是否启用，以及当前是否在正常模式下等待。
    /// Return whether Vim mode is enabled and currently waiting in normal mode.
    ///
    /// Composer-level handlers use this to decide whether Up/Down should be
    /// offered to history navigation only after normal-mode movement reaches a
    /// text boundary.
    pub(crate) fn is_vim_normal_mode(&self) -> bool {
        self.vim_enabled && self.vim_mode == VimMode::Normal
    }

    /// 返回表示在 Vim 正常模式下最后一个可编辑项的位置。
    /// Return the cursor position that represents the last editable item in Vim normal mode.
    pub(crate) fn vim_normal_end_cursor(&self) -> usize {
        if self.text.is_empty() { 0 } else { self.prev_atomic_boundary(self.text.len()) }
    }

    /// 是否等待 Vim 操作符进行运动？ 这可以被观察到，因此输入编辑器可以避免使用` d{motion}`或` y{motion}`中的第二个键作为高级快捷键。
    /// Return whether a Vim operator is waiting for a motion.
    ///
    /// This is observable so the composer can avoid stealing the second key of
    /// `d{motion}` or `y{motion}` for higher-level shortcuts.
    pub(crate) fn is_vim_operator_pending(&self) -> bool {
        !matches!(self.vim_pending, VimPending::None)
    }

    /// 返回持有下一个编辑键的上下文键映射
    /// Return the keymap context that owns the next editing key.
    pub(crate) fn keymap_context(&self) -> KeymapContext {
        if !self.vim_enabled || self.vim_mode == VimMode::Insert {
            return KeymapContext::Editor;
        }
        match self.vim_pending {
            VimPending::None => KeymapContext::VimNormal,
            VimPending::Replace => KeymapContext::Editor,
            VimPending::Operator(_) => KeymapContext::VimOperator,
            VimPending::TextObject { .. } => KeymapContext::VimTextObject,
        }
    }

    /// 如果启用了模态编辑，则输入 Vim 插入模式。 在 Vim 未启用时调用此操作为无副作用（no-op），这允许父工作流在提交前重置模式，而不首先分支当前键映射状态。
    /// Enter Vim insert mode if modal editing is enabled.
    ///
    /// Calling this while Vim is disabled is a no-op, which lets parent
    /// workflows reset mode after submissions without first branching on the
    /// current keymap state.
    pub(crate) fn enter_vim_insert_mode(&mut self) {
        if self.vim_enabled {
            self.vim_mode = VimMode::Insert;
            self.vim_pending = VimPending::None;
        }
    }

    /// 如果启用了模态编辑，则进入正常模式。
    /// Enter Vim normal mode if modal editing is enabled.
    ///
    /// This clears any pending operator and preferred vertical column. The
    /// latter matches normal Vim navigation expectations after leaving insert
    /// mode; preserving the old column would make the next `j` or `k` jump to a
    /// stale visual target.
    pub(crate) fn enter_vim_normal_mode(&mut self) {
        if self.vim_enabled {
            self.vim_mode = VimMode::Normal;
            self.vim_pending = VimPending::None;
            self.preferred_col = None;
        }
    }

    /// 是否应将快速明文爆发视为粘贴输入。
    /// Return whether rapid plain-key bursts should be treated as paste input.
    ///
    /// Paste burst detection is disabled in Vim normal mode so a fast sequence
    /// like `dd` or `yw` remains command input instead of being converted into
    /// literal text.
    pub(crate) fn allows_paste_burst(&self) -> bool {
        !self.vim_enabled || self.vim_mode == VimMode::Insert
    }

    /// 返回是否使用插入模式光标样式渲染。
    /// Return whether rendering should use the insert-mode cursor style.
    pub(crate) fn uses_vim_insert_cursor(&self) -> bool {
        self.vim_enabled && self.vim_mode == VimMode::Insert
    }

    /// 在 Composer 级路由之前拦截是否应被转发的 Escape 键。
    /// Return whether Escape should be intercepted before composer-level routing.
    ///
    /// In Vim insert mode or while a command is pending, Escape is an editing
    /// transition rather than a popup cancel/backtrack or turn-interrupt shortcut.
    pub(crate) fn should_handle_vim_insert_escape(&self, event: KeyEvent) -> bool {
        self.vim_enabled
            && (self.vim_mode == VimMode::Insert || !matches!(self.vim_pending, VimPending::None))
            && event.code == KeyCode::Esc
            && event.modifiers == KeyModifiers::NONE
            && matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
    }

    /// 返回当前激活的 Vim 模式脚本来显示。`None` 表示编辑已禁用，因此调用者应省略模式指示器，而不应为普通非模态编辑渲染插入模式标签。
    /// Return the footer label for the active Vim mode.
    ///
    /// `None` means Vim editing is disabled, so callers should omit the mode
    /// indicator rather than rendering an insert-mode label for normal
    /// non-modal editing.
    #[cfg(test)]
    pub(crate) fn vim_mode_label(&self) -> Option<&'static str> {
        if !self.vim_enabled {
            return None;
        }
        Some(match self.vim_mode {
            VimMode::Normal => "Normal",
            VimMode::Insert => "Insert",
        })
    }

    /// 返回用于激活 Vim 编辑模式的样式脚本报示
    /// Return the styled footer indicator for the active Vim editing mode.
    pub(crate) fn vim_mode_indicator_span(&self) -> Option<Span<'static>> {
        if !self.vim_enabled {
            return None;
        }
        Some(match self.vim_mode {
            VimMode::Normal => "Vim: Normal".magenta(),
            VimMode::Insert => "Vim: Insert".green(),
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn insert_str(&mut self, text: &str) {
        self.insert_str_at(self.cursor_pos, text);
    }

    pub fn insert_str_at(&mut self, pos: usize, text: &str) {
        let pos = self.clamp_pos_for_insertion(pos);
        self.text.insert_str(pos, text);
        self.wrap_cache.replace(None);
        if pos <= self.cursor_pos {
            self.cursor_pos += text.len();
        }
        self.shift_elements(pos, /*removed*/ 0, text.len());
        self.preferred_col = None;
    }

    pub fn replace_range(&mut self, range: std::ops::Range<usize>, text: &str) {
        let range = self.expand_range_to_element_boundaries(range);
        self.replace_range_raw(range, text);
    }

    fn replace_range_raw(&mut self, range: std::ops::Range<usize>, text: &str) {
        assert!(range.start <= range.end);
        let start = range.start.clamp(0, self.text.len());
        let end = range.end.clamp(0, self.text.len());
        let removed_len = end - start;
        let inserted_len = text.len();
        if removed_len == 0 && inserted_len == 0 {
            return;
        }
        let diff = inserted_len as isize - removed_len as isize;

        self.text.replace_range(range, text);
        self.wrap_cache.replace(None);
        self.preferred_col = None;
        self.update_elements_after_replace(start, end, inserted_len);

        // 更新光标位置，以应对编辑操作。
        // Update the cursor position to account for the edit.
        self.cursor_pos = if self.cursor_pos < start {
            // 光标位于编辑范围之前，未发生偏移
            // Cursor was before the edited range – no shift.
            self.cursor_pos
        } else if self.cursor_pos <= end {
            // 光标位于被替换的范围内，将移动到新文本的末尾
            // Cursor was inside the replaced range – move to end of the new text.
            start + inserted_len
        } else {
            // 光标位于替换范围之后，通过长度差进行偏移。
            // Cursor was after the replaced range – shift by the length diff.
            ((self.cursor_pos as isize) + diff) as usize
        }
        .min(self.text.len());

        // 确保光标不在元素内部
        // Ensure cursor is not inside an element
        self.cursor_pos = self.clamp_pos_to_nearest_boundary(self.cursor_pos);
    }

    pub fn cursor(&self) -> usize {
        self.cursor_pos
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor_pos = pos.clamp(0, self.text.len());
        self.cursor_pos = self.clamp_pos_to_nearest_boundary(self.cursor_pos);
        self.preferred_col = None;
    }

    pub fn desired_height(&self, width: u16) -> u16 {
        self.wrapped_lines(width).len() as u16
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.cursor_pos_with_state(area, TextAreaState::default())
    }

    /// 在区域内返回屏幕光标位置，考虑了滚动和换行。
    /// Returns an on-screen cursor position within `area`, accounting for wrapping and scrolling.
    ///
    /// Returns `None` when the viewport has no visible cells.
    pub fn cursor_pos_with_state(&self, area: Rect, state: TextAreaState) -> Option<(u16, u16)> {
        if area.is_empty() {
            return None;
        }

        let lines = self.wrapped_lines(area.width);
        let effective_scroll = self.effective_scroll(area, &lines, state.scroll);
        let (i, col) = wrapping::cursor_position(&self.text, &lines, area.width, self.cursor_pos)?;
        let screen_row = i.saturating_sub(effective_scroll as usize).try_into().unwrap_or(0);
        Some((area.x + col as u16, area.y + screen_row))
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn current_display_col(&self) -> usize {
        let bol = self.beginning_of_current_line();
        display_width(&self.text[bol..self.cursor_pos])
    }

    fn move_to_display_col_on_line(
        &mut self,
        line_start: usize,
        line_end: usize,
        target_col: usize,
    ) {
        let mut width_so_far = 0usize;
        for (i, g) in self.text[line_start..line_end].grapheme_indices(true) {
            width_so_far += display_width(g);
            if width_so_far > target_col {
                self.cursor_pos = line_start + i;
                // 避免在元素内部着陆；将距离最近的边界四舍五入
                // Avoid landing inside an element; round to nearest boundary
                self.cursor_pos = self.clamp_pos_to_nearest_boundary(self.cursor_pos);
                return;
            }
        }
        self.cursor_pos = line_end;
        self.cursor_pos = self.clamp_pos_to_nearest_boundary(self.cursor_pos);
    }

    fn beginning_of_line(&self, pos: usize) -> usize {
        self.text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0)
    }
    fn beginning_of_current_line(&self) -> usize {
        self.beginning_of_line(self.cursor_pos)
    }

    fn first_non_blank_of_current_line(&self) -> usize {
        let bol = self.beginning_of_current_line();
        let eol = self.end_of_current_line();
        self.text[bol..eol]
            .char_indices()
            .find_map(|(offset, ch)| (!ch.is_whitespace()).then_some(bol + offset))
            .unwrap_or(eol)
    }

    fn end_of_line(&self, pos: usize) -> usize {
        self.text[pos..].find('\n').map(|i| i + pos).unwrap_or(self.text.len())
    }
    fn end_of_current_line(&self) -> usize {
        self.end_of_line(self.cursor_pos)
    }

    pub fn input(&mut self, event: KeyEvent) {
        // 仅处理按键或重复事件；忽略释放事件，防止在修饰符不再被报告时插入字符。
        // Only process key presses or repeats; ignore releases to avoid inserting
        // characters on key-up events when modifiers are no longer reported.
        if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return;
        }
        if self.vim_enabled {
            self.handle_vim_input(event);
        } else {
            let keymap = self.editor_keymap.clone();
            self.input_with_keymap(event, &keymap);
        }
    }

    pub fn input_with_keymap(&mut self, event: KeyEvent, keymap: &EditorKeymap) {
        if keymap.insert_newline.is_pressed(event) {
            self.insert_str("\n");
            return;
        }

        if keymap.delete_backward_word.is_pressed(event) {
            self.delete_backward_word();
            return;
        }

        // 当按下 Windows 键盘上的 Alt 键时，会生成 ALT|CONTROL。除非已匹配到特定的快捷键。
        // Windows AltGr generates ALT|CONTROL. Preserve typed characters for AltGr users
        // unless a specific shortcut already matched above.
        if let KeyEvent { code: KeyCode::Char(c), modifiers, .. } = event
            && is_altgr(modifiers)
        {
            self.insert_str(&c.to_string());
            return;
        }

        if keymap.delete_backward.is_pressed(event) {
            self.delete_backward(/*n*/ 1);
            return;
        }
        if keymap.delete_forward_word.is_pressed(event) {
            self.delete_forward_word();
            return;
        }
        if keymap.delete_forward.is_pressed(event) {
            self.delete_forward(/*n*/ 1);
            return;
        }
        if keymap.kill_line_start.is_pressed(event) {
            self.kill_to_beginning_of_line();
            return;
        }
        if keymap.kill_whole_line.is_pressed(event) {
            self.kill_current_line();
            return;
        }
        if keymap.kill_line_end.is_pressed(event) {
            self.kill_to_end_of_line();
            return;
        }
        if keymap.yank.is_pressed(event) {
            self.yank();
            return;
        }
        if keymap.move_word_left.is_pressed(event) {
            self.set_cursor(self.beginning_of_previous_word());
            return;
        }
        if keymap.move_word_right.is_pressed(event) {
            self.set_cursor(self.end_of_next_word());
            return;
        }
        if keymap.move_left.is_pressed(event) {
            self.move_cursor_left();
            return;
        }
        if keymap.move_right.is_pressed(event) {
            self.move_cursor_right();
            return;
        }
        if keymap.move_up.is_pressed(event) {
            self.move_cursor_up();
            return;
        }
        if keymap.move_down.is_pressed(event) {
            self.move_cursor_down();
            return;
        }
        if keymap.move_line_start.is_pressed(event) {
            let move_up_at_bol = matches!(
                event,
                KeyEvent { code: KeyCode::Char('a'), modifiers: KeyModifiers::CONTROL, .. }
            );
            self.move_cursor_to_beginning_of_line(move_up_at_bol);
            return;
        }
        if keymap.move_line_end.is_pressed(event) {
            let move_down_at_eol = matches!(
                event,
                KeyEvent { code: KeyCode::Char('e'), modifiers: KeyModifiers::CONTROL, .. }
            );
            self.move_cursor_to_end_of_line(move_down_at_eol);
            return;
        }

        if let KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE | KeyModifiers::SHIFT,
            ..
        } = event
        {
            // 插入普通字符（并带有 Shift 修饰符）。按住 Alt 键时不插入，因为许多终端将 Option/Meta 组合映射为 Alt+<字符>。
            // Insert plain characters (and Shift-modified). Do not insert when ALT is held,
            // because many terminals map Option/Meta combos to ALT+<char>.
            if c.is_ascii_control() {
                return;
            }
            self.insert_str(&c.to_string());
        }

        tracing::debug!("Unhandled key event in TextArea: {:?}", event);
    }

    fn handle_vim_input(&mut self, event: KeyEvent) {
        match self.vim_mode {
            VimMode::Insert => self.handle_vim_insert(event),
            VimMode::Normal => self.handle_vim_normal(event),
        }
    }

    fn handle_vim_insert(&mut self, event: KeyEvent) {
        if matches!(event.code, KeyCode::Esc) {
            let bol = self.beginning_of_current_line();
            if self.cursor_pos > bol {
                self.cursor_pos = self.prev_atomic_boundary(self.cursor_pos).max(bol);
            }
            self.enter_vim_normal_mode();
            return;
        }
        let keymap = self.editor_keymap.clone();
        self.input_with_keymap(event, &keymap);
    }

    fn handle_vim_normal(&mut self, event: KeyEvent) {
        let pending = std::mem::replace(&mut self.vim_pending, VimPending::None);
        match pending {
            VimPending::None => {}
            VimPending::Operator(op) => {
                self.handle_vim_operator(op, event);
                return;
            }
            VimPending::TextObject { operator, scope } => {
                self.handle_vim_text_object(operator, scope, event);
                return;
            }
            VimPending::Replace => {
                self.handle_vim_pending_command(pending, event);
                return;
            }
        }

        if self.vim_normal_keymap.enter_insert.is_pressed(event) {
            self.vim_mode = VimMode::Insert;
            return;
        }
        if self.vim_normal_keymap.append_after_cursor.is_pressed(event) {
            let next = self.next_atomic_boundary(self.cursor_pos);
            self.set_cursor(next);
            self.vim_mode = VimMode::Insert;
            return;
        }
        if self.vim_normal_keymap.append_line_end.is_pressed(event) {
            self.set_cursor(self.end_of_current_line());
            self.vim_mode = VimMode::Insert;
            return;
        }
        if self.vim_normal_keymap.insert_line_start.is_pressed(event) {
            self.set_cursor(self.first_non_blank_of_current_line());
            self.vim_mode = VimMode::Insert;
            return;
        }
        if self.vim_normal_keymap.open_line_below.is_pressed(event) {
            let eol = self.end_of_current_line();
            let old_len = self.text.len();
            let insert_at = if eol < old_len { eol + 1 } else { eol };
            self.insert_str_at(insert_at, "\n");
            let cursor = if eol < old_len { insert_at } else { insert_at + 1 };
            self.set_cursor(cursor);
            self.vim_mode = VimMode::Insert;
            return;
        }
        if self.vim_normal_keymap.open_line_above.is_pressed(event) {
            let bol = self.beginning_of_current_line();
            self.insert_str_at(bol, "\n");
            self.set_cursor(bol);
            self.vim_mode = VimMode::Insert;
            return;
        }
        if self.vim_normal_keymap.move_left.is_pressed(event) {
            self.move_cursor_left();
            return;
        }
        if self.vim_normal_keymap.move_right.is_pressed(event) {
            self.move_cursor_right();
            return;
        }
        if self.vim_normal_keymap.move_down.is_pressed(event) {
            self.move_cursor_down();
            return;
        }
        if self.vim_normal_keymap.move_up.is_pressed(event) {
            self.move_cursor_up();
            return;
        }
        if self.vim_normal_keymap.move_word_forward.is_pressed(event) {
            self.set_cursor(self.beginning_of_next_word());
            return;
        }
        if self.vim_normal_keymap.move_word_backward.is_pressed(event) {
            self.set_cursor(self.beginning_of_previous_word());
            return;
        }
        if self.vim_normal_keymap.move_word_end.is_pressed(event) {
            self.set_cursor(self.vim_word_end_cursor());
            return;
        }
        if self.vim_normal_keymap.move_line_start.is_pressed(event) {
            self.set_cursor(self.beginning_of_current_line());
            return;
        }
        if self.vim_normal_keymap.move_line_end.is_pressed(event) {
            self.set_cursor(self.vim_line_end_cursor());
            return;
        }
        if self.vim_normal_keymap.delete_char.is_pressed(event) {
            self.delete_forward_kill(/*n*/ 1);
            return;
        }
        if self.vim_normal_keymap.substitute_char.is_pressed(event) {
            if self.cursor_pos < self.end_of_current_line() {
                self.delete_forward_kill(/*n*/ 1);
            }
            self.vim_mode = VimMode::Insert;
            return;
        }
        if self.vim_normal_keymap.delete_to_line_end.is_pressed(event) {
            self.vim_kill_to_end_of_line();
            return;
        }
        if self.vim_normal_keymap.change_to_line_end.is_pressed(event) {
            self.vim_kill_to_end_of_line();
            self.vim_mode = VimMode::Insert;
            return;
        }
        if self.vim_normal_keymap.yank_line.is_pressed(event) {
            self.yank_current_line();
            return;
        }
        if self.vim_normal_keymap.paste_after.is_pressed(event) {
            self.paste_after_cursor();
            return;
        }
        if self.vim_normal_keymap.start_delete_operator.is_pressed(event) {
            self.vim_pending = VimPending::Operator(VimOperator::Delete);
            return;
        }
        if self.vim_normal_keymap.start_yank_operator.is_pressed(event) {
            self.vim_pending = VimPending::Operator(VimOperator::Yank);
            return;
        }
        if self.vim_normal_keymap.start_change_operator.is_pressed(event) {
            self.vim_pending = VimPending::Operator(VimOperator::Change);
            return;
        }
        if self.vim_normal_keymap.cancel_operator.is_pressed(event) {
            self.vim_pending = VimPending::None;
            return;
        }
        self.handle_vim_extra_command(event);
    }

    fn handle_vim_operator(&mut self, op: VimOperator, event: KeyEvent) -> bool {
        if op == VimOperator::Delete && self.vim_operator_keymap.delete_line.is_pressed(event) {
            self.kill_current_line();
            return true;
        }
        if op == VimOperator::Yank && self.vim_operator_keymap.yank_line.is_pressed(event) {
            self.yank_current_line();
            return true;
        }
        if self.vim_operator_keymap.cancel.is_pressed(event) {
            return true;
        }
        if let Some(scope) = self.vim_text_object_scope_for_event(event) {
            self.vim_pending = VimPending::TextObject { operator: op, scope };
            return true;
        }

        if let Some(motion) = self.vim_motion_for_event(event) {
            self.apply_vim_operator(op, motion);
            return true;
        }
        if op == VimOperator::Change
            && self.vim_normal_keymap.start_change_operator.is_pressed(event)
        {
            let range = self.beginning_of_current_line()..self.end_of_current_line();
            self.kill_line_range(range);
            self.vim_mode = VimMode::Insert;
            return true;
        }
        false
    }

    fn handle_vim_text_object(
        &mut self,
        op: VimOperator,
        scope: VimTextObjectScope,
        event: KeyEvent,
    ) -> bool {
        if self.vim_text_object_keymap.cancel.is_pressed(event) {
            return true;
        }
        let Some(object) = self.vim_text_object_for_event(event) else {
            return false;
        };
        if let Some(range) = self.text_object_range(object, scope) {
            self.apply_vim_operator_to_range(op, range);
        }
        true
    }

    fn vim_motion_for_event(&self, event: KeyEvent) -> Option<VimMotion> {
        if self.vim_operator_keymap.motion_left.is_pressed(event) {
            return Some(VimMotion::Left);
        }
        if self.vim_operator_keymap.motion_right.is_pressed(event) {
            return Some(VimMotion::Right);
        }
        if self.vim_operator_keymap.motion_down.is_pressed(event) {
            return Some(VimMotion::Down);
        }
        if self.vim_operator_keymap.motion_up.is_pressed(event) {
            return Some(VimMotion::Up);
        }
        if self.vim_operator_keymap.motion_word_forward.is_pressed(event) {
            return Some(VimMotion::WordForward);
        }
        if self.vim_operator_keymap.motion_word_backward.is_pressed(event) {
            return Some(VimMotion::WordBackward);
        }
        if self.vim_operator_keymap.motion_word_end.is_pressed(event) {
            return Some(VimMotion::WordEnd);
        }
        if self.vim_operator_keymap.motion_line_start.is_pressed(event) {
            return Some(VimMotion::LineStart);
        }
        if self.vim_operator_keymap.motion_line_end.is_pressed(event) {
            return Some(VimMotion::LineEnd);
        }
        None
    }

    fn apply_vim_operator(&mut self, op: VimOperator, motion: VimMotion) {
        if op == VimOperator::Change && motion == VimMotion::WordForward {
            let target = if self.text[self.cursor_pos..]
                .chars()
                .next()
                .is_some_and(|ch| !ch.is_whitespace())
            {
                self.end_of_next_word()
            } else {
                self.beginning_of_next_word().min(self.end_of_current_line())
            };
            if target > self.cursor_pos {
                self.apply_vim_operator_to_range(op, self.cursor_pos..target);
            } else {
                self.vim_mode = VimMode::Insert;
            }
            return;
        }
        let Some(range) = self.range_for_motion(motion) else {
            if op == VimOperator::Change && motion == VimMotion::LineEnd {
                self.vim_mode = VimMode::Insert;
            }
            return;
        };
        if op == VimOperator::Change && matches!(motion, VimMotion::Up | VimMotion::Down) {
            if motion == VimMotion::Up && self.beginning_of_current_line() == 0
                || motion == VimMotion::Down && self.end_of_current_line() == self.text.len()
            {
                return;
            }
            let retain_newline =
                range.end < self.text.len() && self.text[range.clone()].ends_with('\n');
            let start = range.start;
            self.kill_line_range(range);
            if retain_newline {
                self.insert_str_at(start, "\n");
                self.set_cursor(start);
            }
            self.vim_mode = VimMode::Insert;
            return;
        }
        self.apply_vim_operator_to_range(op, range);
    }

    fn apply_vim_operator_to_range(&mut self, op: VimOperator, range: Range<usize>) {
        match op {
            VimOperator::Delete => self.kill_range(range),
            VimOperator::Yank => self.yank_range(range),
            VimOperator::Change => {
                self.kill_range(range);
                self.vim_mode = VimMode::Insert;
            }
        }
    }

    fn range_for_motion(&mut self, motion: VimMotion) -> Option<Range<usize>> {
        if matches!(motion, VimMotion::Up | VimMotion::Down) {
            return self.linewise_range_for_vertical_motion(motion);
        }
        let start = self.cursor_pos;
        let target = self.target_for_motion(motion);
        if start == target {
            return None;
        }
        let (range_start, range_end) =
            if target < start { (target, start) } else { (start, target) };
        Some(range_start..range_end)
    }

    fn linewise_range_for_vertical_motion(&self, motion: VimMotion) -> Option<Range<usize>> {
        let current = self.current_line_range_with_newline();
        let range = match motion {
            VimMotion::Up => {
                let start = if current.start == 0 {
                    current.start
                } else {
                    self.beginning_of_line(current.start.saturating_sub(1))
                };
                start..current.end
            }
            VimMotion::Down => {
                let end = if current.end >= self.text.len() {
                    current.end
                } else {
                    let next_eol = self.end_of_line(current.end);
                    if next_eol < self.text.len() { next_eol + 1 } else { next_eol }
                };
                current.start..end
            }
            VimMotion::Left
            | VimMotion::Right
            | VimMotion::WordForward
            | VimMotion::WordBackward
            | VimMotion::WordEnd
            | VimMotion::LineStart
            | VimMotion::LineEnd => return None,
        };
        (range.start < range.end).then_some(range)
    }

    fn target_for_motion(&mut self, motion: VimMotion) -> usize {
        let original_cursor = self.cursor_pos;
        let original_preferred = self.preferred_col;
        match motion {
            VimMotion::Left => self.move_cursor_left(),
            VimMotion::Right => self.move_cursor_right(),
            VimMotion::Up => self.move_cursor_up(),
            VimMotion::Down => self.move_cursor_down(),
            VimMotion::WordForward => self.set_cursor(self.beginning_of_next_word()),
            VimMotion::WordBackward => self.set_cursor(self.beginning_of_previous_word()),
            VimMotion::WordEnd => self.set_cursor(self.vim_word_end_exclusive()),
            VimMotion::LineStart => self.set_cursor(self.beginning_of_current_line()),
            VimMotion::LineEnd => self.set_cursor(self.end_of_current_line()),
        }
        let target = self.cursor_pos;
        self.cursor_pos = original_cursor;
        self.preferred_col = original_preferred;
        target
    }

    // ## 输入函数
    // ####### Input Functions #######
    pub fn delete_backward(&mut self, n: usize) {
        if n == 0 || self.cursor_pos == 0 {
            return;
        }
        let mut target = self.cursor_pos;
        for _ in 0..n {
            if let Some((boundary, ch)) = self.text[..target].char_indices().next_back()
                && matches!(
                    ch,
                    // 泰语特殊字符处理不理想，若变得复杂则重构。
                    // Thai-only special casing is not ideal; refactor if it becomes more complex.
                    // All Thai nonspacing marks: vowel signs, tone marks, and other diacritics.
                    '\u{0e31}' | '\u{0e34}'..='\u{0e3a}' | '\u{0e47}'..='\u{0e4e}'
                )
                && !self
                    .elements
                    .iter()
                    .any(|element| target > element.range.start && target <= element.range.end)
            {
                target = boundary;
            } else {
                target = self.prev_atomic_boundary(target);
            }
            if target == 0 {
                break;
            }
        }
        self.replace_range(target..self.cursor_pos, "");
    }

    pub fn delete_forward(&mut self, n: usize) {
        if n == 0 || self.cursor_pos >= self.text.len() {
            return;
        }
        let mut target = self.cursor_pos;
        for _ in 0..n {
            target = self.next_atomic_boundary(target);
            if target >= self.text.len() {
                break;
            }
        }
        self.replace_range(self.cursor_pos..target, "");
    }

    pub fn delete_forward_kill(&mut self, n: usize) {
        if n == 0 || self.cursor_pos >= self.text.len() {
            return;
        }
        let mut target = self.cursor_pos;
        for _ in 0..n {
            target = self.next_atomic_boundary(target);
            if target >= self.text.len() {
                break;
            }
        }
        self.kill_range(self.cursor_pos..target);
    }

    pub fn delete_backward_word(&mut self) {
        let start = self.beginning_of_previous_word();
        self.kill_range(start..self.cursor_pos);
    }

    /// 使用"词"语义删除光标右侧的文字。 从当前光标位置开始，直到下一个由 `end_of_next_word()` 确定的单词结束，所有光标与该单词之间的空白（包括换行符）均包含在内。
    /// Delete text to the right of the cursor using "word" semantics.
    ///
    /// Deletes from the current cursor position through the end of the next word as determined
    /// by `end_of_next_word()`. Any whitespace (including newlines) between the cursor and that
    /// word is included in the deletion.
    pub fn delete_forward_word(&mut self) {
        let end = self.end_of_next_word();
        if end > self.cursor_pos {
            self.kill_range(self.cursor_pos..end);
        }
    }

    /// 从光标位置终止当前逻辑行。
    /// Kill from the cursor to the end of the current logical line.
    ///
    /// If the cursor is already at end-of-line and a trailing newline exists, this kills that
    /// newline so repeated invocations continue making progress. The removed text becomes the next
    /// yank target and remains available even if a caller later clears or rewrites the visible
    /// buffer via `set_text_*`.
    pub fn kill_to_end_of_line(&mut self) {
        let eol = self.end_of_current_line();
        let range = if self.cursor_pos == eol {
            if eol < self.text.len() { Some(self.cursor_pos..eol + 1) } else { None }
        } else {
            Some(self.cursor_pos..eol)
        };

        if let Some(range) = range {
            self.kill_range(range);
        }
    }

    fn vim_kill_to_end_of_line(&mut self) {
        let eol = self.end_of_current_line();
        if self.cursor_pos < eol {
            self.kill_range(self.cursor_pos..eol);
        }
    }

    pub fn kill_to_beginning_of_line(&mut self) {
        let bol = self.beginning_of_current_line();
        let range = if self.cursor_pos == bol {
            if bol > 0 { Some(bol - 1..bol) } else { None }
        } else {
            Some(bol..self.cursor_pos)
        };

        if let Some(range) = range {
            self.kill_range(range);
        }
    }

    /// 在光标处插入最近被杀的文字。 此操作使用 textarea 的单条目死缓冲。由于整个缓冲区替换 API 不会清除该缓冲，`yank` 可以在 Composer 级别的清理（如提交和 Slash Command Dispatch）后恢复文字。
    /// Insert the most recently killed text at the cursor.
    ///
    /// This uses the textarea's single-entry kill buffer. Because whole-buffer replacement APIs do
    /// not clear that buffer, `yank` can restore text after composer-level clears such as submit
    /// and slash-command dispatch.
    pub fn yank(&mut self) {
        if self.kill_buffer.is_empty() {
            return;
        }
        let text = self.kill_buffer.clone();
        self.insert_str(&text);
    }

    fn kill_range(&mut self, range: Range<usize>) {
        self.kill_range_with_kind(range, KillBufferKind::Characterwise);
    }

    fn kill_line_range(&mut self, range: Range<usize>) {
        self.kill_range_with_kind(range, KillBufferKind::Linewise);
    }

    fn kill_range_with_kind(&mut self, range: Range<usize>, kind: KillBufferKind) {
        let range = self.expand_range_to_element_boundaries(range);
        if range.start >= range.end {
            return;
        }

        let removed = self.text[range.clone()].to_string();
        if removed.is_empty() {
            return;
        }

        self.store_kill_buffer(removed, kind);
        self.replace_range_raw(range, "");
    }

    fn yank_range(&mut self, range: Range<usize>) {
        self.yank_range_with_kind(range, KillBufferKind::Characterwise);
    }

    fn yank_line_range(&mut self, range: Range<usize>) {
        self.yank_range_with_kind(range, KillBufferKind::Linewise);
    }

    fn yank_range_with_kind(&mut self, range: Range<usize>, kind: KillBufferKind) {
        let range = self.expand_range_to_element_boundaries(range);
        if range.start >= range.end {
            return;
        }
        let removed = self.text[range].to_string();
        if removed.is_empty() {
            return;
        }
        self.store_kill_buffer(removed, kind);
    }

    fn store_kill_buffer(&mut self, text: String, kind: KillBufferKind) {
        self.kill_buffer = text;
        self.kill_buffer_kind = kind;
    }

    fn paste_after_cursor(&mut self) {
        if self.kill_buffer.is_empty() {
            return;
        }
        if self.kill_buffer_kind == KillBufferKind::Linewise {
            self.paste_line_after_current_line();
            return;
        }
        let insert_at = self.next_atomic_boundary(self.cursor_pos);
        self.set_cursor(insert_at);
        let text = self.kill_buffer.clone();
        self.insert_str(&text);
    }

    fn paste_line_after_current_line(&mut self) {
        let eol = self.end_of_current_line();
        let insert_at = if eol < self.text.len() { eol + 1 } else { eol };
        let cursor = if eol < self.text.len() { insert_at } else { insert_at + 1 };
        let text = if eol < self.text.len() {
            if self.kill_buffer.ends_with('\n') {
                self.kill_buffer.clone()
            } else {
                format!("{}\n", self.kill_buffer)
            }
        } else {
            format!("\n{}", self.kill_buffer.trim_end_matches('\n'))
        };
        self.insert_str_at(insert_at, &text);
        self.set_cursor(cursor.min(self.text.len()));
    }

    fn yank_current_line(&mut self) {
        let range = self.current_line_range_with_newline();
        self.yank_line_range(range);
    }

    fn kill_current_line(&mut self) {
        let range = self.current_line_range_with_newline();
        self.kill_line_range(range);
    }

    fn current_line_range_with_newline(&self) -> Range<usize> {
        let bol = self.beginning_of_current_line();
        let eol = self.end_of_current_line();
        let end = if eol < self.text.len() { eol + 1 } else { eol };
        bol..end
    }

    /// 将光标向左移动一个图素簇。
    /// Move the cursor left by a single grapheme cluster.
    pub fn move_cursor_left(&mut self) {
        self.cursor_pos = self.prev_atomic_boundary(self.cursor_pos);
        self.preferred_col = None;
    }

    /// 移动光标向右移动一个图素簇。
    /// Move the cursor right by a single grapheme cluster.
    pub fn move_cursor_right(&mut self) {
        self.cursor_pos = self.next_atomic_boundary(self.cursor_pos);
        self.preferred_col = None;
    }

    pub fn move_cursor_up(&mut self) {
        // 如果存在缓存，优先跨越被包裹（视觉）行。
        // If we have a wrapping cache, prefer navigating across wrapped (visual) lines.
        if let Some((target_col, maybe_line)) = {
            let cache_ref = self.wrap_cache.borrow();
            if let Some(cache) = cache_ref.as_ref() {
                let lines = &cache.lines;
                if let Some((idx, col)) =
                    wrapping::cursor_position(&self.text, lines, cache.width, self.cursor_pos)
                {
                    let cur_range = &lines[idx];
                    // 已保存的列可超越行高。不要在显示在下一行的悬空空白处落地。
                    // A saved column can outlive a resize. Do not land in hanging whitespace
                    // that is displayed on the following row.
                    let target_col = self
                        .preferred_col
                        .unwrap_or(col)
                        .min(usize::from(cache.width.saturating_sub(1)));
                    if idx > 0 {
                        let prev = &lines[idx - 1];
                        let line_start = prev.start;
                        let mut line_end = prev.end.saturating_sub(1);
                        if line_end == cur_range.start {
                            line_end = self.prev_atomic_boundary(line_end).max(line_start);
                        }
                        Some((target_col, Some((line_start, line_end))))
                    } else {
                        Some((target_col, None))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } {
            // 我们已有包裹信息，据此调整移动。
            // We had wrapping info. Apply movement accordingly.
            match maybe_line {
                Some((line_start, line_end)) => {
                    if self.preferred_col.is_none() {
                        self.preferred_col = Some(target_col);
                    }
                    self.move_to_display_col_on_line(line_start, line_end, target_col);
                    return;
                }
                None => {
                    // 已位于第一行 -> 移动到起始位置
                    // Already at first visual line -> move to start
                    self.cursor_pos = 0;
                    self.preferred_col = None;
                    return;
                }
            }
        }

        // 如果尚未获得包裹信息，则采用逻辑行导航。
        // Fallback to logical line navigation if we don't have wrapping info yet.
        if let Some(prev_nl) = self.text[..self.cursor_pos].rfind('\n') {
            let target_col = match self.preferred_col {
                Some(c) => c,
                None => {
                    let c = self.current_display_col();
                    self.preferred_col = Some(c);
                    c
                }
            };
            let prev_line_start = self.text[..prev_nl].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let prev_line_end = prev_nl;
            self.move_to_display_col_on_line(prev_line_start, prev_line_end, target_col);
        } else {
            self.cursor_pos = 0;
            self.preferred_col = None;
        }
    }

    pub fn move_cursor_down(&mut self) {
        // 如果存在缓存，优先跨越被包裹（视觉）行。
        // If we have a wrapping cache, prefer navigating across wrapped (visual) lines.
        if let Some((target_col, move_to_last)) = {
            let cache_ref = self.wrap_cache.borrow();
            if let Some(cache) = cache_ref.as_ref() {
                let lines = &cache.lines;
                if let Some((idx, col)) =
                    wrapping::cursor_position(&self.text, lines, cache.width, self.cursor_pos)
                {
                    let target_col = self
                        .preferred_col
                        .unwrap_or(col)
                        .min(usize::from(cache.width.saturating_sub(1)));
                    if idx + 1 < lines.len() {
                        let next = &lines[idx + 1];
                        let line_start = next.start;
                        let mut line_end = next.end.saturating_sub(1);
                        if lines.get(idx + 2).is_some_and(|following| following.start == line_end) {
                            line_end = self.prev_atomic_boundary(line_end).max(line_start);
                        }
                        Some((target_col, Some((line_start, line_end))))
                    } else {
                        Some((target_col, None))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } {
            match move_to_last {
                Some((line_start, line_end)) => {
                    if self.preferred_col.is_none() {
                        self.preferred_col = Some(target_col);
                    }
                    self.move_to_display_col_on_line(line_start, line_end, target_col);
                    return;
                }
                None => {
                    // 已位于最后一行视觉线，移至末尾
                    // Already on last visual line -> move to end
                    self.cursor_pos = self.text.len();
                    self.preferred_col = None;
                    return;
                }
            }
        }

        // 如果尚未获得包裹信息，则采用逻辑行导航。
        // Fallback to logical line navigation if we don't have wrapping info yet.
        let target_col = match self.preferred_col {
            Some(c) => c,
            None => {
                let c = self.current_display_col();
                self.preferred_col = Some(c);
                c
            }
        };
        if let Some(next_nl) = self.text[self.cursor_pos..].find('\n').map(|i| i + self.cursor_pos)
        {
            let next_line_start = next_nl + 1;
            let next_line_end = self.text[next_line_start..]
                .find('\n')
                .map(|i| i + next_line_start)
                .unwrap_or(self.text.len());
            self.move_to_display_col_on_line(next_line_start, next_line_end, target_col);
        } else {
            self.cursor_pos = self.text.len();
            self.preferred_col = None;
        }
    }

    pub fn move_cursor_to_beginning_of_line(&mut self, move_up_at_bol: bool) {
        let bol = self.beginning_of_current_line();
        if move_up_at_bol && self.cursor_pos == bol {
            self.set_cursor(self.beginning_of_line(self.cursor_pos.saturating_sub(1)));
        } else {
            self.set_cursor(bol);
        }
        self.preferred_col = None;
    }

    pub fn move_cursor_to_end_of_line(&mut self, move_down_at_eol: bool) {
        let eol = self.end_of_current_line();
        if move_down_at_eol && self.cursor_pos == eol {
            let next_pos = (self.cursor_pos.saturating_add(1)).min(self.text.len());
            self.set_cursor(self.end_of_line(next_pos));
        } else {
            self.set_cursor(eol);
        }
    }

    // 文本元素支持
    // ===== Text elements support =====

    pub fn element_payloads(&self) -> Vec<String> {
        self.elements
            .iter()
            .filter_map(|e| self.text.get(e.range.clone()).map(str::to_string))
            .collect()
    }

    pub fn text_elements(&self) -> Vec<UserTextElement> {
        self.elements
            .iter()
            .map(|e| {
                let placeholder = self.text.get(e.range.clone()).map(str::to_string);
                UserTextElement::new(
                    ByteRange { start: e.range.start, end: e.range.end },
                    placeholder,
                )
            })
            .collect()
    }

    pub(crate) fn text_element_snapshots(&self) -> Vec<TextElementSnapshot> {
        self.elements
            .iter()
            .filter_map(|element| {
                self.text.get(element.range.clone()).map(|text| TextElementSnapshot {
                    id: element.id,
                    range: element.range.clone(),
                    text: text.to_string(),
                })
            })
            .collect()
    }

    /// 遍历递增顺序的原子元素范围，并迭代借有的原子元素。
    /// Iterates borrowed atomic element ranges in ascending start order.
    pub(crate) fn text_element_ranges(&self) -> impl Iterator<Item = &Range<usize>> {
        self.elements.iter().map(|element| &element.range)
    }

    /// 遍历重叠的原子元素范围，按顺序迭代。
    /// Iterates ordered atomic element ranges that overlap `range`.
    ///
    /// Elements ending exactly at the range start or starting exactly at its end are excluded.
    pub(crate) fn text_element_ranges_overlapping(
        &self,
        range: Range<usize>,
    ) -> impl Iterator<Item = &Range<usize>> {
        let first = self.elements.partition_point(|element| element.range.end <= range.start);
        self.elements[first..]
            .iter()
            .take_while(move |element| element.range.start < range.end)
            .map(|element| &element.range)
    }

    pub(crate) fn element_id_for_exact_range(&self, range: Range<usize>) -> Option<u64> {
        self.elements.iter().find(|element| element.range == range).map(|element| element.id)
    }

    /// 将单个文本元素原地重命名，保持原子性。
    /// Renames a single text element in-place, keeping it atomic.
    ///
    /// Use this when the element payload is an identifier (e.g. a placeholder) that must be
    /// updated without converting the element back into normal text.
    pub fn replace_element_payload(&mut self, old: &str, new: &str) -> bool {
        let Some(idx) =
            self.elements.iter().position(|e| self.text.get(e.range.clone()) == Some(old))
        else {
            return false;
        };

        let range = self.elements[idx].range.clone();
        let start = range.start;
        let end = range.end;
        if start > end || end > self.text.len() {
            return false;
        }

        let removed_len = end - start;
        let inserted_len = new.len();
        let diff = inserted_len as isize - removed_len as isize;

        self.text.replace_range(range, new);
        self.wrap_cache.replace(None);
        self.preferred_col = None;

        // 更新修改元素的范围
        // Update the modified element's range.
        self.elements[idx].range = start..(start + inserted_len);

        // 替换元素之后发生的位置范围
        // Shift element ranges that occur after the replaced element.
        if diff != 0 {
            for (j, e) in self.elements.iter_mut().enumerate() {
                if j == idx {
                    continue;
                }
                if e.range.end <= start {
                    continue;
                }
                if e.range.start >= end {
                    e.range.start = ((e.range.start as isize) + diff) as usize;
                    e.range.end = ((e.range.end as isize) + diff) as usize;
                    continue;
                }

                // 元素不应部分重叠；当发生重叠时，优雅降级为将任何与替换范围相交的元素映射到新边界。
                // Elements should not partially overlap each other; degrade gracefully by
                // snapping anything intersecting the replaced range to the new bounds.
                e.range.start = start.min(e.range.start);
                e.range.end = (start + inserted_len).max(e.range.end.saturating_add_signed(diff));
            }
        }

        // 更新光标位置，以应对编辑操作。
        // Update the cursor position to account for the edit.
        self.cursor_pos = if self.cursor_pos < start {
            self.cursor_pos
        } else if self.cursor_pos <= end {
            start + inserted_len
        } else {
            ((self.cursor_pos as isize) + diff) as usize
        };
        self.cursor_pos = self.clamp_pos_to_nearest_boundary(self.cursor_pos);

        // 保持元素顺序确定性
        // Keep element ordering deterministic.
        self.elements.sort_by_key(|e| e.range.start);

        true
    }

    pub fn insert_element(&mut self, text: &str) -> u64 {
        let start = self.clamp_pos_for_insertion(self.cursor_pos);
        self.insert_str_at(start, text);
        let end = start + text.len();
        let id = self.add_element(start..end);
        // 将光标定位到插入元素末尾
        // Place cursor at end of inserted element
        self.set_cursor(end);
        id
    }

    fn add_element(&mut self, range: Range<usize>) -> u64 {
        let id = self.next_element_id();
        self.elements.push(TextElement { id, range });
        self.elements.sort_by_key(|e| e.range.start);
        id
    }

    /// 标记一个已存在的文本范围作为原子元素，而不改变文本内容。
    /// Mark an existing text range as an atomic element without changing the text.
    ///
    /// This is used to convert already-typed tokens (like `/plan`) into elements
    /// so they render and edit atomically. Overlapping or duplicate ranges are ignored.
    pub fn add_element_range(&mut self, range: Range<usize>) -> Option<u64> {
        let start = self.clamp_pos_to_char_boundary(range.start.min(self.text.len()));
        let end = self.clamp_pos_to_char_boundary(range.end.min(self.text.len()));
        if start >= end {
            return None;
        }
        if self.elements.iter().any(|e| e.range.start == start && e.range.end == end) {
            return None;
        }
        if self.elements.iter().any(|e| start < e.range.end && end > e.range.start) {
            return None;
        }
        let id = self.add_element(start..end);
        Some(id)
    }

    pub fn remove_element_range(&mut self, range: Range<usize>) -> bool {
        let start = self.clamp_pos_to_char_boundary(range.start.min(self.text.len()));
        let end = self.clamp_pos_to_char_boundary(range.end.min(self.text.len()));
        if start >= end {
            return false;
        }
        let len_before = self.elements.len();
        self.elements.retain(|elem| elem.range.start != start || elem.range.end != end);
        len_before != self.elements.len()
    }

    fn next_element_id(&mut self) -> u64 {
        let id = self.next_element_id;
        self.next_element_id = self.next_element_id.saturating_add(1);
        id
    }
    fn find_element_containing(&self, pos: usize) -> Option<usize> {
        self.elements.iter().position(|e| pos > e.range.start && pos < e.range.end)
    }

    fn clamp_pos_to_char_boundary(&self, pos: usize) -> usize {
        let pos = pos.min(self.text.len());
        if self.text.is_char_boundary(pos) {
            return pos;
        }
        let mut prev = pos;
        while prev > 0 && !self.text.is_char_boundary(prev) {
            prev -= 1;
        }
        let mut next = pos;
        while next < self.text.len() && !self.text.is_char_boundary(next) {
            next += 1;
        }
        if pos.saturating_sub(prev) <= next.saturating_sub(pos) { prev } else { next }
    }

    fn clamp_pos_to_nearest_boundary(&self, pos: usize) -> usize {
        let pos = self.clamp_pos_to_char_boundary(pos);
        if let Some(idx) = self.find_element_containing(pos) {
            let e = &self.elements[idx];
            let dist_start = pos.saturating_sub(e.range.start);
            let dist_end = e.range.end.saturating_sub(pos);
            if dist_start <= dist_end {
                self.clamp_pos_to_char_boundary(e.range.start)
            } else {
                self.clamp_pos_to_char_boundary(e.range.end)
            }
        } else {
            pos
        }
    }

    fn clamp_pos_for_insertion(&self, pos: usize) -> usize {
        let pos = self.clamp_pos_to_char_boundary(pos);
        // 不允许在元素中间插入内容
        // Do not allow inserting into the middle of an element
        if let Some(idx) = self.find_element_containing(pos) {
            let e = &self.elements[idx];
            // 选择插入的最近边
            // Choose closest edge for insertion
            let dist_start = pos.saturating_sub(e.range.start);
            let dist_end = e.range.end.saturating_sub(pos);
            if dist_start <= dist_end {
                self.clamp_pos_to_char_boundary(e.range.start)
            } else {
                self.clamp_pos_to_char_boundary(e.range.end)
            }
        } else {
            pos
        }
    }

    fn expand_range_to_element_boundaries(&self, mut range: Range<usize>) -> Range<usize> {
        // 将包含所有重叠元素的任何部分全部展开
        // Expand to include any intersecting elements fully
        loop {
            let mut changed = false;
            for e in &self.elements {
                if e.range.start < range.end && e.range.end > range.start {
                    let new_start = range.start.min(e.range.start);
                    let new_end = range.end.max(e.range.end);
                    if new_start != range.start || new_end != range.end {
                        range.start = new_start;
                        range.end = new_end;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        range
    }

    fn shift_elements(&mut self, at: usize, removed: usize, inserted: usize) {
        // 通用移位：纯插入时移量为 0；删除时移量为 0。
        // Generic shift: for pure insert, removed = 0; for delete, inserted = 0.
        let end = at + removed;
        let diff = inserted as isize - removed as isize;
        // 移除被操作完全删除的元素，并将其余元素向前移动
        // Remove elements fully deleted by the operation and shift the rest
        self.elements.retain(|e| !(e.range.start >= at && e.range.end <= end));
        for e in &mut self.elements {
            if e.range.end <= at {
                // 编辑前
                // before edit
            } else if e.range.start >= end {
                // 编辑后
                // after edit
                e.range.start = ((e.range.start as isize) + diff) as usize;
                e.range.end = ((e.range.end as isize) + diff) as usize;
            } else {
                // 元素重叠但不完全包含（使用元素感知替换时不应发生，但可通过将元素锚定到新边界来优雅降级）
                // Overlap with element but not fully contained (shouldn't happen when using
                // element-aware replace, but degrade gracefully by snapping element to new bounds)
                let new_start = at.min(e.range.start);
                let new_end = at + inserted.max(e.range.end.saturating_sub(end));
                e.range.start = new_start;
                e.range.end = new_end;
            }
        }
    }

    fn update_elements_after_replace(&mut self, start: usize, end: usize, inserted_len: usize) {
        self.shift_elements(start, end.saturating_sub(start), inserted_len);
    }

    fn prev_atomic_boundary(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        // 如果当前处于元素末尾或内部，则跳转到该元素的起始位置。
        // If currently at an element end or inside, jump to start of that element.
        if let Some(idx) =
            self.elements.iter().position(|e| pos > e.range.start && pos <= e.range.end)
        {
            return self.elements[idx].range.start;
        }
        let mut gc = unicode_segmentation::GraphemeCursor::new(pos, self.text.len(), false);
        match gc.prev_boundary(&self.text, 0) {
            Ok(Some(b)) => {
                if let Some(idx) = self.find_element_containing(b) {
                    self.elements[idx].range.start
                } else {
                    b
                }
            }
            Ok(None) => 0,
            Err(_) => pos.saturating_sub(1),
        }
    }

    fn next_atomic_boundary(&self, pos: usize) -> usize {
        if pos >= self.text.len() {
            return self.text.len();
        }
        // 如果当前处于元素起始位置或内部，则跳转到该元素的末尾。
        // If currently at an element start or inside, jump to end of that element.
        if let Some(idx) =
            self.elements.iter().position(|e| pos >= e.range.start && pos < e.range.end)
        {
            return self.elements[idx].range.end;
        }
        let mut gc = unicode_segmentation::GraphemeCursor::new(pos, self.text.len(), false);
        match gc.next_boundary(&self.text, 0) {
            Ok(Some(b)) => {
                if let Some(idx) = self.find_element_containing(b) {
                    self.elements[idx].range.end
                } else {
                    b
                }
            }
            Ok(None) => self.text.len(),
            Err(_) => pos.saturating_add(1),
        }
    }

    pub(crate) fn beginning_of_previous_word(&self) -> usize {
        let prefix = &self.text[..self.cursor_pos];
        let Some((first_non_ws_idx, ch)) =
            prefix.char_indices().rev().find(|&(_, ch)| !ch.is_whitespace())
        else {
            return 0;
        };
        let run_start = prefix[..first_non_ws_idx]
            .char_indices()
            .rev()
            .find(|&(_, ch)| ch.is_whitespace())
            .map_or(0, |(idx, ch)| idx + ch.len_utf8());
        let run_end = first_non_ws_idx + ch.len_utf8();
        let pieces = split_word_pieces(&prefix[run_start..run_end]);
        let mut pieces = pieces.into_iter().rev().peekable();
        let Some((piece_start, piece)) = pieces.next() else {
            return run_start;
        };
        let mut start = run_start + piece_start;

        if piece.chars().all(is_word_separator) {
            while let Some((idx, piece)) = pieces.peek() {
                if !piece.chars().all(is_word_separator) {
                    break;
                }
                start = run_start + *idx;
                pieces.next();
            }
        }

        self.adjust_pos_out_of_elements(start, /*prefer_start*/ true)
    }

    pub(crate) fn end_of_next_word(&self) -> usize {
        self.end_of_next_word_from(self.cursor_pos)
    }

    fn end_of_next_word_from(&self, cursor_pos: usize) -> usize {
        let suffix = &self.text[cursor_pos..];
        let Some(first_non_ws) = suffix.find(|ch: char| !ch.is_whitespace()) else {
            return self.text.len();
        };
        let run = &suffix[first_non_ws..];
        let run = &run[..run.find(char::is_whitespace).unwrap_or(run.len())];
        let mut pieces = split_word_pieces(run).into_iter().peekable();
        let Some((start, piece)) = pieces.next() else {
            return cursor_pos + first_non_ws;
        };
        let word_start = cursor_pos + first_non_ws + start;
        let mut end = word_start + piece.len();
        if piece.chars().all(is_word_separator) {
            while let Some((idx, piece)) = pieces.peek() {
                if !piece.chars().all(is_word_separator) {
                    break;
                }
                end = cursor_pos + first_non_ws + *idx + piece.len();
                pieces.next();
            }
        }

        self.adjust_pos_out_of_elements(end, /*prefer_start*/ false)
    }

    fn vim_word_end_exclusive(&self) -> usize {
        let end = self.end_of_next_word();
        let target = if end > self.cursor_pos { self.prev_atomic_boundary(end) } else { end };
        if target == self.cursor_pos && end < self.text.len() {
            self.end_of_next_word_from(end)
        } else {
            end
        }
    }

    fn vim_word_end_cursor(&self) -> usize {
        let end = self.vim_word_end_exclusive();
        if end > self.cursor_pos { self.prev_atomic_boundary(end) } else { end }
    }

    fn vim_line_end_cursor(&self) -> usize {
        let bol = self.beginning_of_current_line();
        let eol = self.end_of_current_line();
        if eol > bol { self.prev_atomic_boundary(eol).max(bol) } else { eol }
    }

    pub(crate) fn beginning_of_next_word(&self) -> usize {
        let Some(first_non_ws) = self.text[self.cursor_pos..].find(|c: char| !c.is_whitespace())
        else {
            return self.text.len();
        };
        let word_start = self.cursor_pos + first_non_ws;
        if word_start != self.cursor_pos {
            return self.adjust_pos_out_of_elements(word_start, /*prefer_start*/ true);
        }
        let end = self.end_of_next_word();
        if end >= self.text.len() {
            return self.text.len();
        }
        let Some(next_non_ws) = self.text[end..].find(|c: char| !c.is_whitespace()) else {
            return self.text.len();
        };
        self.adjust_pos_out_of_elements(end + next_non_ws, /*prefer_start*/ true)
    }

    fn adjust_pos_out_of_elements(&self, pos: usize, prefer_start: bool) -> usize {
        if let Some(idx) = self.find_element_containing(pos) {
            let e = &self.elements[idx];
            if prefer_start { e.range.start } else { e.range.end }
        } else {
            pos
        }
    }

    /// 返回缓存的图灵安全视觉范围，包括光标位置 sentinel 字节。 溢出空格不分离部分空白延续符与下一个单词，现有单词断点保持完整，完整逻辑行获得续行以保留插入点可见性。
    /// Returns cached grapheme-safe visual ranges, including cursor-position sentinel bytes.
    ///
    /// Overflowing spaces wrap without separating a partial whitespace continuation from the next
    /// word, existing word breakpoints stay intact, and full logical lines receive a continuation
    /// row so their insertion point stays visible.
    #[expect(clippy::unwrap_used)]
    fn wrapped_lines(&self, width: u16) -> Ref<'_, Vec<Range<usize>>> {
        // 确保缓存已就绪（可能通过可共享的借用，然后立即销毁）
        // Ensure cache is ready (potentially mutably borrow, then drop)
        {
            let mut cache = self.wrap_cache.borrow_mut();
            let needs_recalc = match cache.as_ref() {
                Some(c) => c.width != width,
                None => true,
            };
            if needs_recalc {
                let display_text = text_for_display(&self.text);
                let lines = wrapping::wrapped_lines(display_text.as_ref(), width);
                *cache = Some(WrapCache { width, lines });
            }
        }

        let cache = self.wrap_cache.borrow();
        Ref::map(cache, |c| &c.as_ref().unwrap().lines)
    }

    /// 计算满足当前区域大小和滚动行数约束所需的滚动偏移量。
    /// Calculate the scroll offset that should be used to satisfy the
    /// invariants given the current area size and wrapped lines.
    ///
    /// - Cursor is always on screen.
    /// - No scrolling if content fits in the area.
    fn effective_scroll(&self, area: Rect, lines: &[Range<usize>], current_scroll: u16) -> u16 {
        let total_lines = lines.len() as u16;
        if area.height >= total_lines {
            return 0;
        }

        let cursor_line_idx =
            wrapping::cursor_position(&self.text, lines, area.width, self.cursor_pos)
                .map_or(0, |(row, _)| row) as u16;

        let max_scroll = total_lines.saturating_sub(area.height);
        let mut scroll = current_scroll.min(max_scroll);

        // 确保光标在 [scroll, scroll + area_height) 范围内可见
        // Ensure cursor is visible within [scroll, scroll + area_height)
        if cursor_line_idx < scroll {
            scroll = cursor_line_idx;
        } else if cursor_line_idx >= scroll + area.height {
            scroll = cursor_line_idx + 1 - area.height;
        }
        scroll
    }
}

impl WidgetRef for &TextArea {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let lines = self.wrapped_lines(area.width);
        self.render_lines(
            area,
            buf,
            &lines,
            0..lines.len().min(usize::from(area.height)),
            Style::default(),
            &[],
        );
    }
}

impl StatefulWidgetRef for &TextArea {
    type State = TextAreaState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area, &lines, state.scroll);
        state.scroll = scroll;

        let start = scroll as usize;
        let end = (scroll + area.height).min(lines.len() as u16) as usize;
        self.render_lines(area, buf, &lines, start..end, Style::default(), &[]);
    }
}

impl TextArea {
    pub(crate) fn render_ref_masked(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut TextAreaState,
        mask_char: char,
    ) {
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area, &lines, state.scroll);
        state.scroll = scroll;

        let start = scroll as usize;
        let end = (scroll + area.height).min(lines.len() as u16) as usize;
        self.render_lines_masked(area, buf, &lines, start..end, mask_char);
    }

    /// 使用 base_style 样式渲染 textarea，并添加额外的仅渲染的 highlight 范围。
    /// Render the textarea with `base_style` plus additional render-only highlight ranges.
    ///
    /// Highlight ranges are byte ranges in `self.text`. They affect only the buffer rendering and
    /// do not mutate the editable text, cursor, element metadata, or wrapping cache.
    pub(crate) fn render_ref_styled_with_highlights(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut TextAreaState,
        base_style: Style,
        highlights: &[(Range<usize>, Style)],
    ) {
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area, &lines, state.scroll);
        state.scroll = scroll;

        let start = scroll as usize;
        let end = (scroll + area.height).min(lines.len() as u16) as usize;
        self.render_lines(area, buf, &lines, start..end, base_style, highlights);
    }

    /// 在文本域视口之外渲染可见文本与样式化覆盖层
    /// Renders visible text and styled overlays without writing outside the textarea viewport.
    fn render_lines(
        &self,
        area: Rect,
        buf: &mut Buffer,
        lines: &[Range<usize>],
        range: std::ops::Range<usize>,
        base_style: Style,
        highlights: &[(Range<usize>, Style)],
    ) {
        let element_style = base_style.fg(Color::Cyan);
        for (row, idx) in range.enumerate() {
            let r = &lines[idx];
            let y = area.y + row as u16;
            let visible = wrapping::visible_prefix(&self.text[r.start..r.end - 1], area.width);
            let line_range = r.start..r.start + visible.len();
            buf.set_style(Rect::new(area.x, y, area.width, 1), base_style);
            // 使用提供的样式绘制基础线
            // Draw base line with the provided style.
            buf.set_stringn(
                area.x,
                y,
                text_for_display(visible),
                usize::from(area.width),
                base_style,
            );

            // 在样式元素上保持可见的搜索高亮效果。
            // Apply search highlights last so they remain visible over styled elements.
            let overlays = self
                .elements
                .iter()
                .map(|element| (&element.range, element_style))
                .chain(highlights.iter().map(|(range, style)| (range, *style)));
            for (overlay_range, style) in overlays {
                let overlap_start = overlay_range.start.max(line_range.start);
                let overlap_end = overlay_range.end.min(line_range.end);
                if overlap_start >= overlap_end {
                    continue;
                }
                let styled = &self.text[overlap_start..overlap_end];
                let x_off = display_width(&self.text[line_range.start..overlap_start]);
                if x_off >= usize::from(area.width) {
                    continue;
                }
                let x_off = x_off as u16;
                buf.set_stringn(
                    area.x + x_off,
                    y,
                    text_for_display(styled),
                    usize::from(area.width.saturating_sub(x_off)),
                    style,
                );
            }
        }
    }

    /// 在文本域视口之外不渲染宽度保持的掩码字符。
    /// Renders width-preserving mask glyphs without writing outside the textarea viewport.
    fn render_lines_masked(
        &self,
        area: Rect,
        buf: &mut Buffer,
        lines: &[Range<usize>],
        range: std::ops::Range<usize>,
        mask_char: char,
    ) {
        for (row, idx) in range.enumerate() {
            let r = &lines[idx];
            let y = area.y + row as u16;
            let visible = wrapping::visible_prefix(&self.text[r.start..r.end - 1], area.width);
            let masked = visible
                .graphemes(/*is_extended*/ true)
                .flat_map(|grapheme| std::iter::repeat_n(mask_char, display_width(grapheme)))
                .collect::<String>();
            buf.set_stringn(area.x, y, &masked, usize::from(area.width), Style::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_hint;
    // 此处故意未导入 crossterm 类型，以避免不必要的警告
    // crossterm types are intentionally not imported here to avoid unused warnings
    use pretty_assertions::assert_eq;
    #[cfg(any())]
    use rand::prelude::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[cfg(any())]
    fn rand_grapheme(rng: &mut rand::rngs::StdRng) -> String {
        let r: u8 = rng.random_range(0..100);
        match r {
            0..=4 => "\n".to_string(),
            5..=12 => " ".to_string(),
            13..=35 => (rng.random_range(b'a'..=b'z') as char).to_string(),
            36..=45 => (rng.random_range(b'A'..=b'Z') as char).to_string(),
            46..=52 => (rng.random_range(b'0'..=b'9') as char).to_string(),
            53..=65 => {
                // 一些宽字符 Emoji
                // Some emoji (wide graphemes)
                let choices = ["👍", "😊", "🐍", "🚀", "🧪", "🌟"];
                choices[rng.random_range(0..choices.len())].to_string()
            }
            66..=75 => {
                // CJK 宽字符
                // CJK wide characters
                let choices = ["漢", "字", "測", "試", "你", "好", "界", "编", "码"];
                choices[rng.random_range(0..choices.len())].to_string()
            }
            76..=85 => {
                // 合并标记序列
                // Combining mark sequences
                let base = ["e", "a", "o", "n", "u"][rng.random_range(0..5)];
                let marks = ["\u{0301}", "\u{0308}", "\u{0302}", "\u{0303}"];
                format!("{base}{}", marks[rng.random_range(0..marks.len())])
            }
            86..=92 => {
                // 一些非拉丁字符的单字代码（希腊字母、西里尔字母、希伯来字母）
                // Some non-latin single codepoints (Greek, Cyrillic, Hebrew)
                let choices = ["Ω", "β", "Ж", "ю", "ש", "م", "ह"];
                choices[rng.random_range(0..choices.len())].to_string()
            }
            _ => {
                // ZWJ 序列（单字符但多码点）
                // ZWJ sequences (single graphemes but multi-codepoint)
                let choices = [
                    "👩\u{200D}💻", // woman technologist
                    "👨\u{200D}💻", // man technologist
                    "🏳️\u{200D}🌈", // rainbow flag
                ];
                choices[rng.random_range(0..choices.len())].to_string()
            }
        }
    }

    fn ta_with(text: &str) -> TextArea {
        let mut t = TextArea::new();
        t.insert_str(text);
        t
    }

    // 验证 `insert_and_replace_update_cursor_and_text` 场景的触发条件与预期结果。
    #[test]
    fn insert_and_replace_update_cursor_and_text() {
        // 插入辅助函数
        // insert helpers
        let mut t = ta_with("hello");
        t.set_cursor(/*pos*/ 5);
        t.insert_str("!");
        assert_eq!(t.text(), "hello!");
        assert_eq!(t.cursor(), 6);

        t.insert_str_at(/*pos*/ 0, "X");
        assert_eq!(t.text(), "Xhello!");
        assert_eq!(t.cursor(), 7);

        // 插入后光标不应移动
        // Insert after the cursor should not move it
        t.set_cursor(/*pos*/ 1);
        let end = t.text().len();
        t.insert_str_at(end, "Y");
        assert_eq!(t.text(), "Xhello!Y");
        assert_eq!(t.cursor(), 1);

        // 替换范围情况
        // replace_range cases
        // 1) cursor before range
        let mut t = ta_with("abcd");
        t.set_cursor(/*pos*/ 1);
        t.replace_range(2..3, "Z");
        assert_eq!(t.text(), "abZd");
        assert_eq!(t.cursor(), 1);

        // 在范围内内显示光标
        // 2) cursor inside range
        let mut t = ta_with("abcd");
        t.set_cursor(/*pos*/ 2);
        t.replace_range(1..3, "Q");
        assert_eq!(t.text(), "aQd");
        assert_eq!(t.cursor(), 2);

        // 光标在偏移量范围内
        // 3) cursor after range with shifted by diff
        let mut t = ta_with("abcd");
        t.set_cursor(/*pos*/ 4);
        t.replace_range(0..1, "AA");
        assert_eq!(t.text(), "AAbcd");
        assert_eq!(t.cursor(), 5);
    }

    // 验证 `v3_textarea_preserves_unicode_cursor_and_paste_behavior` 场景的触发条件与预期结果。
    #[test]
    fn v3_textarea_preserves_unicode_cursor_and_paste_behavior() {
        use crate::bottom_pane::paste_burst::{CharDecision, FlushResult, PasteBurst};
        use std::time::Instant;

        let mut textarea = TextArea::new();
        for ch in "你🙂e\u{301}".chars() {
            textarea.input(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        assert_eq!(textarea.text(), "你🙂e\u{301}");
        assert_eq!(textarea.cursor(), textarea.text().len());

        // 光标跟随扩展字符而非 UTF-8 字节或标量值。
        // Cursor movement follows extended graphemes, not UTF-8 bytes or scalar values.
        textarea.move_cursor_left();
        assert_eq!(textarea.cursor(), "你🙂".len());
        textarea.move_cursor_left();
        assert_eq!(textarea.cursor(), "你".len());
        textarea.input(KeyEvent::new(KeyCode::Char('好'), KeyModifiers::NONE));
        assert_eq!(textarea.text(), "你好🙂e\u{301}");

        let mut burst = PasteBurst::default();
        let start = Instant::now();
        assert!(matches!(burst.on_plain_char('多', start), CharDecision::RetainFirstChar));
        assert!(matches!(
            burst.on_plain_char('行', start + std::time::Duration::from_millis(1)),
            CharDecision::BeginBufferFromPending
        ));
        burst.append_char_to_buffer('粘', start);
        burst.append_newline_if_active(start);
        burst.append_char_to_buffer('贴', start);
        assert!(burst.newline_should_insert_instead_of_submit(start));
        assert!(matches!(
            burst.flush_before_modified_input(),
            Some(text) if text.contains('粘') && text.contains('贴')
        ));
        assert!(matches!(
            burst.flush_if_due(start + PasteBurst::recommended_active_flush_delay()),
            FlushResult::None
        ));
    }

    // 验证 `insert_str_at_clamps_to_char_boundary` 场景的触发条件与预期结果。
    #[test]
    fn insert_str_at_clamps_to_char_boundary() {
        let mut t = TextArea::new();
        t.insert_str("你");
        t.set_cursor(/*pos*/ 0);
        t.insert_str_at(/*pos*/ 1, "A");
        assert_eq!(t.text(), "A你");
        assert_eq!(t.cursor(), 1);
    }

    // 验证 `set_text_clamps_cursor_to_char_boundary` 场景的触发条件与预期结果。
    #[test]
    fn set_text_clamps_cursor_to_char_boundary() {
        let mut t = TextArea::new();
        t.insert_str("abcd");
        t.set_cursor(/*pos*/ 1);
        t.set_text_clearing_elements("你");
        assert_eq!(t.cursor(), 0);
        t.insert_str("a");
        assert_eq!(t.text(), "a你");
    }

    // 验证 `delete_backward_and_forward_edges` 场景的触发条件与预期结果。
    #[test]
    fn delete_backward_and_forward_edges() {
        let mut t = ta_with("abc");
        t.set_cursor(/*pos*/ 1);
        t.delete_backward(/*n*/ 1);
        assert_eq!(t.text(), "bc");
        assert_eq!(t.cursor(), 0);

        // 删除操作在开始处是无效的
        // deleting backward at start is a no-op
        t.set_cursor(/*pos*/ 0);
        t.delete_backward(/*n*/ 1);
        assert_eq!(t.text(), "bc");
        assert_eq!(t.cursor(), 0);

        // 删除下一个图符
        // forward delete removes next grapheme
        t.set_cursor(/*pos*/ 1);
        t.delete_forward(/*n*/ 1);
        assert_eq!(t.text(), "b");
        assert_eq!(t.cursor(), 1);

        // 删除操作在末尾是空操作
        // forward delete at end is a no-op
        t.set_cursor(t.text().len());
        t.delete_forward(/*n*/ 1);
        assert_eq!(t.text(), "b");
    }

    // 验证 `delete_backward_removes_thai_marks_one_at_a_time` 场景的触发条件与预期结果。
    #[test]
    fn delete_backward_removes_thai_marks_one_at_a_time() {
        let mut t = ta_with("ที่");

        t.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));

        let area = Rect::new(0, 0, /*width*/ 8, /*height*/ 1);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();
        insta::assert_snapshot!(
            "textarea_thai_backspace_preserves_remaining_marks",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );
        assert_eq!(t.text(), "ที");

        t.delete_backward(/*n*/ 1);
        assert_eq!(t.text(), "ท");

        t.delete_backward(/*n*/ 1);
        assert_eq!(t.text(), "");
    }

    // 验证 `delete_backward_preserves_other_grapheme_behavior` 场景的触发条件与预期结果。
    #[test]
    fn delete_backward_preserves_other_grapheme_behavior() {
        let mut thai_spacing_vowel = ta_with("ซ้ำ");
        thai_spacing_vowel.delete_backward(/*n*/ 1);
        assert_eq!(thai_spacing_vowel.text(), "ซ้");

        let mut decomposed_latin = ta_with("e\u{301}");
        decomposed_latin.delete_backward(/*n*/ 1);
        assert_eq!(decomposed_latin.text(), "");

        let mut family_emoji = ta_with("👨\u{200d}👩\u{200d}👧\u{200d}👦");
        family_emoji.delete_backward(/*n*/ 1);
        assert_eq!(family_emoji.text(), "");
    }

    // 验证 `delete_backward_keeps_thai_elements_atomic` 场景的触发条件与预期结果。
    #[test]
    fn delete_backward_keeps_thai_elements_atomic() {
        let mut t = TextArea::new();
        t.insert_str("before ");
        t.insert_element("ที่");
        t.insert_str(" after");

        t.set_cursor(t.elements[0].range.end);
        t.delete_backward(/*n*/ 1);

        assert_eq!(t.text(), "before  after");
        assert_eq!(t.cursor(), "before ".len());
        assert!(t.elements.is_empty());
    }

    // 验证 `delete_forward_deletes_element_at_left_edge` 场景的触发条件与预期结果。
    #[test]
    fn delete_forward_deletes_element_at_left_edge() {
        let mut t = TextArea::new();
        t.insert_str("a");
        t.insert_element("<element>");
        t.insert_str("b");

        let elem_start = t.elements[0].range.start;
        t.set_cursor(elem_start);
        t.delete_forward(/*n*/ 1);

        assert_eq!(t.text(), "ab");
        assert_eq!(t.cursor(), elem_start);
    }

    // 验证 `vim_insert_and_escape` 场景的触发条件与预期结果。
    #[test]
    fn vim_insert_and_escape() {
        let mut t = TextArea::new();
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(t.text(), "h");
        assert_eq!(t.vim_mode_label(), Some("Normal"));
        assert_eq!(t.cursor(), 0);
    }

    // 验证 `vim_insert_key_enters_insert_mode` 场景的触发条件与预期结果。
    #[test]
    fn vim_insert_key_enters_insert_mode() {
        let mut t = TextArea::new();
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Insert, KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

        assert_eq!(t.text(), "h");
        assert_eq!(t.vim_mode_label(), Some("Insert"));
    }

    // 验证 `vim_normal_arrow_keys_move_cursor` 场景的触发条件与预期结果。
    #[test]
    fn vim_normal_arrow_keys_move_cursor() {
        let mut t = ta_with("ab\ncd");
        t.set_cursor(/*pos*/ 1);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert_eq!(t.cursor(), 2);

        t.input(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(t.cursor(), 5);

        t.input(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(t.cursor(), 4);

        t.input(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(t.cursor(), 1);
    }

    // 验证 `vim_escape_from_insert_at_start_does_not_underflow` 场景的触发条件与预期结果。
    #[test]
    fn vim_escape_from_insert_at_start_does_not_underflow() {
        let mut t = TextArea::new();
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(t.vim_mode_label(), Some("Normal"));
        assert_eq!(t.cursor(), 0);
    }

    // 验证 `vim_escape_from_insert_at_line_start_stays_on_line` 场景的触发条件与预期结果。
    #[test]
    fn vim_escape_from_insert_at_line_start_stays_on_line() {
        let mut t = ta_with("one\ntwo");
        t.set_cursor(/*pos*/ "one\n".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(t.vim_mode_label(), Some("Normal"));
        assert_eq!(t.cursor(), "one\n".len());
    }

    // 验证 `vim_escape_moves_by_grapheme_boundary` 场景的触发条件与预期结果。
    #[test]
    fn vim_escape_moves_by_grapheme_boundary() {
        let mut t = ta_with("👍👍");
        t.set_cursor(t.text().len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(t.vim_mode_label(), Some("Normal"));
        assert_eq!(t.cursor(), "👍".len());
    }

    // 验证 `vim_escape_respects_atomic_element_boundary` 场景的触发条件与预期结果。
    #[test]
    fn vim_escape_respects_atomic_element_boundary() {
        let mut t = TextArea::new();
        t.insert_str("a");
        t.insert_element("<element>");
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(t.vim_mode_label(), Some("Normal"));
        assert_eq!(t.cursor(), 1);
    }

    // 验证 `vim_shift_i_enters_insert_at_first_non_blank_with_shift_only_binding` 场景的触发条件与预期结果。
    #[test]
    fn vim_shift_i_enters_insert_at_first_non_blank_with_shift_only_binding() {
        let mut t = ta_with("hello\n  world");
        t.vim_normal_keymap.insert_line_start = vec![key_hint::shift(KeyCode::Char('i'))];
        t.set_cursor(/*pos*/ "hello\n  wor".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::NONE));

        assert_eq!(t.vim_mode_label(), Some("Insert"));
        assert_eq!(t.cursor(), "hello\n  ".len());
    }

    // 验证 `vim_shift_a_enters_insert_at_line_end_with_shift_only_binding` 场景的触发条件与预期结果。
    #[test]
    fn vim_shift_a_enters_insert_at_line_end_with_shift_only_binding() {
        let mut t = ta_with("hello\nworld");
        t.vim_normal_keymap.append_line_end = vec![key_hint::shift(KeyCode::Char('a'))];
        t.set_cursor(/*pos*/ 8);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));

        assert_eq!(t.vim_mode_label(), Some("Insert"));
        assert_eq!(t.cursor(), 11);
    }

    // 验证 `vim_shift_c_changes_to_line_end_and_enters_insert_mode` 场景的触发条件与预期结果。
    #[test]
    fn vim_shift_c_changes_to_line_end_and_enters_insert_mode() {
        let mut t = ta_with("hello world\nnext line");
        t.set_cursor(/*pos*/ 6);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::SHIFT));

        assert_eq!(t.text(), "hello \nnext line");
        assert_eq!(t.vim_mode_label(), Some("Insert"));
        assert_eq!(t.cursor(), 6);
        assert_eq!(t.kill_buffer, "world");
    }

    // 验证 `vim_uppercase_c_changes_to_line_end` 场景的触发条件与预期结果。
    #[test]
    fn vim_uppercase_c_changes_to_line_end() {
        let mut t = ta_with("hello world\nnext line");
        t.set_cursor(/*pos*/ 6);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::NONE));

        assert_eq!(t.text(), "hello \nnext line");
        assert_eq!(t.vim_mode_label(), Some("Insert"));
        assert_eq!(t.cursor(), 6);
    }

    // 验证 `vim_s_substitutes_current_character_and_enters_insert_mode` 场景的触发条件与预期结果。
    #[test]
    fn vim_s_substitutes_current_character_and_enters_insert_mode() {
        let mut t = ta_with("abc");
        t.set_cursor(/*pos*/ 1);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

        assert_eq!(t.text(), "ac");
        assert_eq!(t.cursor(), 1);
        assert_eq!(t.vim_mode_label(), Some("Insert"));

        t.input(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));

        assert_eq!(t.text(), "aXc");
        assert_eq!(t.cursor(), 2);
        assert_eq!(t.vim_mode_label(), Some("Insert"));
    }

    // 验证 `vim_s_on_empty_line_enters_insert_without_deleting_newline` 场景的触发条件与预期结果。
    #[test]
    fn vim_s_on_empty_line_enters_insert_without_deleting_newline() {
        let mut t = ta_with("before\n\nnext");
        t.set_cursor(/*pos*/ "before\n".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));

        assert_eq!(t.text(), "before\n\nnext");
        assert_eq!(t.cursor(), "before\n".len());
        assert_eq!(t.vim_mode_label(), Some("Insert"));
    }

    // 验证 `vim_d_at_line_end_does_not_remove_newline` 场景的触发条件与预期结果。
    #[test]
    fn vim_d_at_line_end_does_not_remove_newline() {
        let mut t = ta_with("hello\nworld");
        t.set_cursor(/*pos*/ "hello".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE));

        assert_eq!(t.text(), "hello\nworld");
        assert_eq!(t.vim_mode_label(), Some("Normal"));
        assert_eq!(t.kill_buffer, "");
    }

    // 验证 `vim_c_at_line_end_enters_insert_without_removing_newline` 场景的触发条件与预期结果。
    #[test]
    fn vim_c_at_line_end_enters_insert_without_removing_newline() {
        let mut t = ta_with("hello\nworld");
        t.set_cursor(/*pos*/ "hello".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::NONE));

        assert_eq!(t.text(), "hello\nworld");
        assert_eq!(t.vim_mode_label(), Some("Insert"));
        assert_eq!(t.cursor(), "hello".len());
        assert_eq!(t.kill_buffer, "");
    }

    // 验证 `vim_shift_o_opens_line_above_with_shift_only_binding` 场景的触发条件与预期结果。
    #[test]
    fn vim_shift_o_opens_line_above_with_shift_only_binding() {
        let mut t = ta_with("hello\nworld");
        t.vim_normal_keymap.open_line_above = vec![key_hint::shift(KeyCode::Char('o'))];
        t.set_cursor(/*pos*/ 8);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::NONE));

        assert_eq!(t.text(), "hello\n\nworld");
        assert_eq!(t.vim_mode_label(), Some("Insert"));
        assert_eq!(t.cursor(), 6);
    }

    // 验证 `vim_o_opens_line_below_on_inserted_line` 场景的触发条件与预期结果。
    #[test]
    fn vim_o_opens_line_below_on_inserted_line() {
        let mut t = ta_with("one\ntwo");
        t.set_cursor(/*pos*/ 1);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

        assert_eq!(t.text(), "one\n\ntwo");
        assert_eq!(t.vim_mode_label(), Some("Insert"));
        assert_eq!(t.cursor(), "one\n".len());
    }

    // 验证 `vim_o_opens_line_below_final_line_and_moves_to_new_line` 场景的触发条件与预期结果。
    #[test]
    fn vim_o_opens_line_below_final_line_and_moves_to_new_line() {
        let mut t = ta_with("one");
        t.set_cursor(/*pos*/ 1);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

        assert_eq!(t.text(), "one\n");
        assert_eq!(t.vim_mode_label(), Some("Insert"));
        assert_eq!(t.cursor(), "one\n".len());
    }

    // 验证 `vim_delete_word` 场景的触发条件与预期结果。
    #[test]
    fn vim_delete_word() {
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ 0);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

        assert_eq!(t.text(), "world");
        assert_eq!(t.kill_buffer, "hello ");
    }

    // 验证 `vim_change_inner_word_deletes_word_and_enters_insert` 场景的触发条件与预期结果。
    #[test]
    fn vim_change_inner_word_deletes_word_and_enters_insert() {
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ "hello ".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

        assert_eq!(t.text(), "hello ");
        assert_eq!(t.kill_buffer, "world");
        assert_eq!(t.cursor(), "hello ".len());
        assert_eq!(t.vim_mode_label(), Some("Insert"));
    }

    // 验证 `vim_word_text_objects_cover_delete_yank_and_big_word` 场景的触发条件与预期结果。
    #[test]
    fn vim_word_text_objects_cover_delete_yank_and_big_word() {
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ 1);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

        assert_eq!(t.text(), "hello world");
        assert_eq!(t.kill_buffer, "hello ");
        assert_eq!(t.vim_mode_label(), Some("Normal"));

        let mut t = ta_with("foo.bar/baz qux");
        t.set_cursor(/*pos*/ "foo.".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('W'), KeyModifiers::NONE));

        assert_eq!(t.text(), " qux");
        assert_eq!(t.kill_buffer, "foo.bar/baz");
    }

    // 验证 `vim_word_text_objects_accept_cursor_at_word_end` 场景的触发条件与预期结果。
    #[test]
    fn vim_word_text_objects_accept_cursor_at_word_end() {
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ "hello".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));

        assert_eq!(t.text(), "world");
        assert_eq!(t.kill_buffer, "hello ");

        let mut t = ta_with("foo bar");
        t.set_cursor(t.text().len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('W'), KeyModifiers::NONE));

        assert_eq!(t.text(), "foo ");
        assert_eq!(t.kill_buffer, "bar");
        assert_eq!(t.cursor(), "foo ".len());
        assert_eq!(t.vim_mode_label(), Some("Insert"));
    }

    // 验证 `vim_delimiter_text_objects_select_innermost_pair_and_aliases` 场景的触发条件与预期结果。
    #[test]
    fn vim_delimiter_text_objects_select_innermost_pair_and_aliases() {
        let mut t = ta_with("a(b(c)d)e");
        t.set_cursor(/*pos*/ "a(b(".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));

        assert_eq!(t.text(), "a(b()d)e");
        assert_eq!(t.kill_buffer, "c");
        assert_eq!(t.vim_mode_label(), Some("Insert"));

        let mut t = ta_with("a [b] c");
        t.set_cursor(/*pos*/ "a [".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));

        assert_eq!(t.text(), "a  c");
        assert_eq!(t.kill_buffer, "[b]");
    }

    // 验证 `vim_empty_inner_text_objects_are_valid_targets` 场景的触发条件与预期结果。
    #[test]
    fn vim_empty_inner_text_objects_are_valid_targets() {
        let mut t = ta_with("call()");
        t.set_cursor(/*pos*/ "call(".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('('), KeyModifiers::NONE));

        assert_eq!(t.text(), "call()");
        assert_eq!(t.kill_buffer, "");
        assert_eq!(t.cursor(), "call(".len());
        assert_eq!(t.vim_mode_label(), Some("Insert"));

        let mut t = ta_with(r#"say "" now"#);
        t.set_cursor(/*pos*/ r#"say ""#.len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE));

        assert_eq!(t.text(), r#"say "" now"#);
        assert_eq!(t.kill_buffer, "");
        assert_eq!(t.cursor(), r#"say ""#.len());
        assert_eq!(t.vim_mode_label(), Some("Insert"));
    }

    // 验证 `vim_quote_text_objects_are_line_local_and_handle_escapes` 场景的触发条件与预期结果。
    #[test]
    fn vim_quote_text_objects_are_line_local_and_handle_escapes() {
        let mut t = ta_with(r#"say "a \"b\" c" now"#);
        t.set_cursor(/*pos*/ r#"say "a \"#.len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('"'), KeyModifiers::SHIFT));

        assert_eq!(t.text(), r#"say "" now"#);
        assert_eq!(t.kill_buffer, r#"a \"b\" c"#);
        assert_eq!(t.vim_mode_label(), Some("Insert"));

        let mut t = ta_with("one \"two\nthree\" four");
        t.set_cursor(/*pos*/ "one \"two\n".len());
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('"'), KeyModifiers::NONE));

        assert_eq!(t.text(), "one \"two\nthree\" four");
        assert_eq!(t.kill_buffer, "");
    }

    // 验证 `vim_text_object_cancellation_does_not_edit` 场景的触发条件与预期结果。
    #[test]
    fn vim_text_object_cancellation_does_not_edit() {
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ 1);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
        assert!(t.is_vim_operator_pending());
        t.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert_eq!(t.text(), "hello world");
        assert_eq!(t.kill_buffer, "");
        assert!(!t.is_vim_operator_pending());
    }

    // 验证 `vim_operator_invalid_motion_is_consumed` 场景的触发条件与预期结果。
    #[test]
    fn vim_operator_invalid_motion_is_consumed() {
        let mut t = ta_with("hello");
        t.set_cursor(/*pos*/ 0);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(t.is_vim_operator_pending());

        t.input(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));

        assert_eq!(t.text(), "hello");
        assert_eq!(t.vim_mode_label(), Some("Normal"));
        assert_eq!(t.cursor(), 0);
        assert!(!t.is_vim_operator_pending());
    }

    // 验证 `vim_e_lands_on_word_end_character` 场景的触发条件与预期结果。
    #[test]
    fn vim_e_lands_on_word_end_character() {
        let mut t = ta_with("abc");
        t.set_cursor(/*pos*/ 0);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

        assert_eq!(t.cursor(), 2);

        t.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert_eq!(t.text(), "ab");
        assert_eq!(t.kill_buffer, "c");
    }

    // 验证 `vim_e_advances_from_each_word_end` 场景的触发条件与预期结果。
    #[test]
    fn vim_e_advances_from_each_word_end() {
        let mut t = ta_with("alpha beta gamma");
        t.set_cursor("alph".len()); // codespell:ignore alph
        t.set_vim_enabled(/*enabled*/ true);
        let mut states = Vec::new();

        for _ in 0..3 {
            t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
            states.push(format!("{}\n{}^", t.text(), " ".repeat(t.cursor())));
        }

        insta::assert_snapshot!("vim_e_advances_from_each_word_end", states.join("\n\n"));
    }

    // 验证 `vim_delete_to_word_end_advances_from_existing_word_end` 场景的触发条件与预期结果。
    #[test]
    fn vim_delete_to_word_end_advances_from_existing_word_end() {
        let mut t = ta_with("alpha beta gamma");
        t.set_cursor("alph".len()); // codespell:ignore alph
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

        assert_eq!(t.text(), "alph gamma"); // codespell:ignore alph
        assert_eq!(t.kill_buffer, "a beta");
    }

    // 验证 `vim_e_from_word_end_can_land_on_trailing_space` 场景的触发条件与预期结果。
    #[test]
    fn vim_e_from_word_end_can_land_on_trailing_space() {
        let mut t = ta_with("alpha   ");
        t.set_cursor("alph".len()); // codespell:ignore alph
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

        assert_eq!(t.cursor(), "alpha  ".len());
    }

    // 验证 `vim_e_advances_across_atomic_element_word_ends` 场景的触发条件与预期结果。
    #[test]
    fn vim_e_advances_across_atomic_element_word_ends() {
        let mut t = TextArea::new();
        t.insert_str("alpha ");
        t.insert_element("<element>");
        t.insert_str(" gamma");
        let element_start = t.elements[0].range.start;
        t.set_cursor("alph".len()); // codespell:ignore alph
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(t.cursor(), element_start);

        t.input(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
        assert_eq!(t.cursor(), "alpha <element> gamm".len());
    }

    // 验证 `vim_dollar_lands_on_line_end_character` 场景的触发条件与预期结果。
    #[test]
    fn vim_dollar_lands_on_line_end_character() {
        let mut t = ta_with("abc\n123");
        t.set_cursor(/*pos*/ 1);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('$'), KeyModifiers::NONE));

        assert_eq!(t.cursor(), 2);

        t.input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));

        assert_eq!(t.text(), "ab\n123");
        assert_eq!(t.kill_buffer, "c");
    }

    // 验证 `vim_linewise_yank_pastes_below_current_line` 场景的触发条件与预期结果。
    #[test]
    fn vim_linewise_yank_pastes_below_current_line() {
        let mut t = ta_with("abc\n123\nxyz");
        t.set_cursor(/*pos*/ 1);
        t.set_vim_enabled(/*enabled*/ true);

        t.input(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        t.input(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));

        assert_eq!(t.text(), "abc\nabc\n123\nxyz");
        assert_eq!(t.cursor(), "abc\n".len());
        assert_eq!(t.kill_buffer, "abc\n");
        assert_eq!(t.kill_buffer_kind, KillBufferKind::Linewise);
    }

    // 验证 `delete_backward_word_and_kill_line_variants` 场景的触发条件与预期结果。
    #[test]
    fn delete_backward_word_and_kill_line_variants() {
        // 删除末尾单词会移除前一个单词
        // delete backward word at end removes the whole previous word
        let mut t = ta_with("hello   world  ");
        t.set_cursor(t.text().len());
        t.delete_backward_word();
        assert_eq!(t.text(), "hello   ");
        assert_eq!(t.cursor(), 8);

        // 从单词内部删除到光标位置
        // From inside a word, delete from word start to cursor
        let mut t = ta_with("foo bar");
        t.set_cursor(/*pos*/ 6); // inside "bar" (after 'a')
        t.delete_backward_word();
        assert_eq!(t.text(), "foo r");
        assert_eq!(t.cursor(), 4);

        // 从末尾删除最后一个单词
        // From end, delete the last word only
        let mut t = ta_with("foo bar");
        t.set_cursor(t.text().len());
        t.delete_backward_word();
        assert_eq!(t.text(), "foo ");
        assert_eq!(t.cursor(), 4);

        // 当不在行尾时，执行终止换行
        // kill_to_end_of_line when not at EOL
        let mut t = ta_with("abc\ndef");
        t.set_cursor(/*pos*/ 1); // on first line, middle
        t.kill_to_end_of_line();
        assert_eq!(t.text(), "a\ndef");
        assert_eq!(t.cursor(), 1);

        // 当到达行尾时，执行删除换行符
        // kill_to_end_of_line when at EOL deletes newline
        let mut t = ta_with("abc\ndef");
        t.set_cursor(/*pos*/ 3); // EOL of first line
        t.kill_to_end_of_line();
        assert_eq!(t.text(), "abcdef");
        assert_eq!(t.cursor(), 3);

        // 从中间开始结束整行
        // kill_to_beginning_of_line from middle of line
        let mut t = ta_with("abc\ndef");
        t.set_cursor(/*pos*/ 5); // on second line, after 'e'
        t.kill_to_beginning_of_line();
        assert_eq!(t.text(), "abc\nef");

        // 在行首非第一行处移除前一个换行符
        // kill_to_beginning_of_line at beginning of non-first line removes the previous newline
        let mut t = ta_with("abc\ndef");
        t.set_cursor(/*pos*/ 4); // beginning of second line
        t.kill_to_beginning_of_line();
        assert_eq!(t.text(), "abcdef");
        assert_eq!(t.cursor(), 3);
    }

    // 验证 `kill_current_line_removes_current_line_linewise` 场景的触发条件与预期结果。
    #[test]
    fn kill_current_line_removes_current_line_linewise() {
        let mut t = ta_with("abc\ndef\nghi");
        t.set_cursor(/*pos*/ 5);

        t.kill_current_line();

        assert_eq!(t.text(), "abc\nghi");
        assert_eq!(t.cursor(), 4);
        assert_eq!(t.kill_buffer, "def\n");
        assert_eq!(t.kill_buffer_kind, KillBufferKind::Linewise);
    }

    // 验证 `kill_current_line_keeps_previous_newline_for_final_line` 场景的触发条件与预期结果。
    #[test]
    fn kill_current_line_keeps_previous_newline_for_final_line() {
        let mut t = ta_with("abc\ndef");
        t.set_cursor(/*pos*/ 5);

        t.kill_current_line();

        assert_eq!(t.text(), "abc\n");
        assert_eq!(t.cursor(), 4);
        assert_eq!(t.kill_buffer, "def");
        assert_eq!(t.kill_buffer_kind, KillBufferKind::Linewise);
    }

    // 验证 `kill_whole_line_keymap_dispatch_uses_linewise_kill` 场景的触发条件与预期结果。
    #[test]
    fn kill_whole_line_keymap_dispatch_uses_linewise_kill() {
        let mut t = ta_with("abc\ndef\nghi");
        t.set_cursor(/*pos*/ 5);
        let mut keymap = RuntimeKeymap::defaults().editor;
        Arc::make_mut(&mut keymap).kill_line_start.clear();
        Arc::make_mut(&mut keymap).kill_whole_line = vec![key_hint::ctrl(KeyCode::Char('u'))];

        t.input_with_keymap(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL), &keymap);

        assert_eq!(t.text(), "abc\nghi");
        assert_eq!(t.cursor(), 4);
        assert_eq!(t.kill_buffer, "def\n");
        assert_eq!(t.kill_buffer_kind, KillBufferKind::Linewise);
    }

    // 验证 `delete_forward_word_variants` 场景的触发条件与预期结果。
    #[test]
    fn delete_forward_word_variants() {
        let mut t = ta_with("hello   world ");
        t.set_cursor(/*pos*/ 0);
        t.delete_forward_word();
        assert_eq!(t.text(), "   world ");
        assert_eq!(t.cursor(), 0);

        let mut t = ta_with("hello   world ");
        t.set_cursor(/*pos*/ 1);
        t.delete_forward_word();
        assert_eq!(t.text(), "h   world ");
        assert_eq!(t.cursor(), 1);

        let mut t = ta_with("hello   world");
        t.set_cursor(t.text().len());
        t.delete_forward_word();
        assert_eq!(t.text(), "hello   world");
        assert_eq!(t.cursor(), t.text().len());

        let mut t = ta_with("foo   \nbar");
        t.set_cursor(/*pos*/ 3);
        t.delete_forward_word();
        assert_eq!(t.text(), "foo");
        assert_eq!(t.cursor(), 3);

        let mut t = ta_with("foo\nbar");
        t.set_cursor(/*pos*/ 3);
        t.delete_forward_word();
        assert_eq!(t.text(), "foo");
        assert_eq!(t.cursor(), 3);

        let mut t = ta_with("hello   world ");
        t.set_cursor(t.text().len() + 10);
        t.delete_forward_word();
        assert_eq!(t.text(), "hello   world ");
        assert_eq!(t.cursor(), t.text().len());
    }

    // 验证 `delete_forward_word_handles_atomic_elements` 场景的触发条件与预期结果。
    #[test]
    fn delete_forward_word_handles_atomic_elements() {
        let mut t = TextArea::new();
        t.insert_element("<element>");
        t.insert_str(" tail");

        t.set_cursor(/*pos*/ 0);
        t.delete_forward_word();
        assert_eq!(t.text(), " tail");
        assert_eq!(t.cursor(), 0);

        let mut t = TextArea::new();
        t.insert_str("   ");
        t.insert_element("<element>");
        t.insert_str(" tail");

        t.set_cursor(/*pos*/ 0);
        t.delete_forward_word();
        assert_eq!(t.text(), " tail");
        assert_eq!(t.cursor(), 0);

        let mut t = TextArea::new();
        t.insert_str("prefix ");
        t.insert_element("<element>");
        t.insert_str(" tail");

        // 在元素中间，删除前缀单词会删除该元素
        // cursor in the middle of the element, delete_forward_word deletes the element
        let elem_range = t.elements[0].range.clone();
        t.cursor_pos = elem_range.start + (elem_range.len() / 2);
        t.delete_forward_word();
        assert_eq!(t.text(), "prefix  tail");
        assert_eq!(t.cursor(), elem_range.start);
    }

    // 验证 `delete_backward_word_respects_word_separators` 场景的触发条件与预期结果。
    #[test]
    fn delete_backward_word_respects_word_separators() {
        let mut t = ta_with("path/to/file");
        t.set_cursor(t.text().len());
        t.delete_backward_word();
        assert_eq!(t.text(), "path/to/");
        assert_eq!(t.cursor(), t.text().len());

        t.delete_backward_word();
        assert_eq!(t.text(), "path/to");
        assert_eq!(t.cursor(), t.text().len());

        let mut t = ta_with("foo/ ");
        t.set_cursor(t.text().len());
        t.delete_backward_word();
        assert_eq!(t.text(), "foo");
        assert_eq!(t.cursor(), 3);

        let mut t = ta_with("foo /");
        t.set_cursor(t.text().len());
        t.delete_backward_word();
        assert_eq!(t.text(), "foo ");
        assert_eq!(t.cursor(), 4);
    }

    // 验证 `delete_forward_word_respects_word_separators` 场景的触发条件与预期结果。
    #[test]
    fn delete_forward_word_respects_word_separators() {
        let mut t = ta_with("path/to/file");
        t.set_cursor(/*pos*/ 0);
        t.delete_forward_word();
        assert_eq!(t.text(), "/to/file");
        assert_eq!(t.cursor(), 0);

        t.delete_forward_word();
        assert_eq!(t.text(), "to/file");
        assert_eq!(t.cursor(), 0);

        let mut t = ta_with("/ foo");
        t.set_cursor(/*pos*/ 0);
        t.delete_forward_word();
        assert_eq!(t.text(), " foo");
        assert_eq!(t.cursor(), 0);

        let mut t = ta_with(" /foo");
        t.set_cursor(/*pos*/ 0);
        t.delete_forward_word();
        assert_eq!(t.text(), "foo");
        assert_eq!(t.cursor(), 0);
    }

    // 验证 `yank_restores_last_kill` 场景的触发条件与预期结果。
    #[test]
    fn yank_restores_last_kill() {
        let mut t = ta_with("hello");
        t.set_cursor(/*pos*/ 0);
        t.kill_to_end_of_line();
        assert_eq!(t.text(), "");
        assert_eq!(t.cursor(), 0);

        t.yank();
        assert_eq!(t.text(), "hello");
        assert_eq!(t.cursor(), 5);

        let mut t = ta_with("hello world");
        t.set_cursor(t.text().len());
        t.delete_backward_word();
        assert_eq!(t.text(), "hello ");
        assert_eq!(t.cursor(), 6);

        t.yank();
        assert_eq!(t.text(), "hello world");
        assert_eq!(t.cursor(), 11);

        let mut t = ta_with("hello");
        t.set_cursor(/*pos*/ 5);
        t.kill_to_beginning_of_line();
        assert_eq!(t.text(), "");
        assert_eq!(t.cursor(), 0);

        t.yank();
        assert_eq!(t.text(), "hello");
        assert_eq!(t.cursor(), 5);
    }

    // 验证 `kill_buffer_persists_across_set_text` 场景的触发条件与预期结果。
    #[test]
    fn kill_buffer_persists_across_set_text() {
        let mut t = ta_with("restore me");
        t.set_cursor(/*pos*/ 0);
        t.kill_to_end_of_line();
        assert!(t.text().is_empty());

        t.set_text_clearing_elements("/diff");
        t.set_text_clearing_elements("");
        t.yank();

        assert_eq!(t.text(), "restore me");
        assert_eq!(t.cursor(), "restore me".len());
    }

    // 验证 `cursor_left_and_right_handle_graphemes` 场景的触发条件与预期结果。
    #[test]
    fn cursor_left_and_right_handle_graphemes() {
        let mut t = ta_with("a👍b");
        t.set_cursor(t.text().len());

        t.move_cursor_left(); // before 'b'
        let after_first_left = t.cursor();
        t.move_cursor_left(); // before '👍'
        let after_second_left = t.cursor();
        t.move_cursor_left(); // before 'a'
        let after_third_left = t.cursor();

        assert!(after_first_left < t.text().len());
        assert!(after_second_left < after_first_left);
        assert!(after_third_left < after_second_left);

        // 安全地回到末尾
        // Move right back to end safely
        t.move_cursor_right();
        t.move_cursor_right();
        t.move_cursor_right();
        assert_eq!(t.cursor(), t.text().len());
    }

    // 验证 `control_b_and_f_move_cursor` 场景的触发条件与预期结果。
    #[test]
    fn control_b_and_f_move_cursor() {
        let mut t = ta_with("abcd");
        t.set_cursor(/*pos*/ 1);

        t.input(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));
        assert_eq!(t.cursor(), 2);

        t.input(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        assert_eq!(t.cursor(), 1);
    }

    // 验证 `control_b_f_fallback_control_chars_move_cursor` 场景的触发条件与预期结果。
    #[test]
    fn control_b_f_fallback_control_chars_move_cursor() {
        let mut t = ta_with("abcd");
        t.set_cursor(/*pos*/ 2);

        // 模拟发送不带控制字符的终端，并移除 CONTROL 修饰符。
        // Simulate terminals that send C0 control chars without CONTROL modifier.
        // ^B (U+0002) should move left
        t.input(KeyEvent::new(KeyCode::Char('\u{0002}'), KeyModifiers::NONE));
        assert_eq!(t.cursor(), 1);

        // ^F (Unicode U+0006) 应向右移动
        // ^F (U+0006) should move right
        t.input(KeyEvent::new(KeyCode::Char('\u{0006}'), KeyModifiers::NONE));
        assert_eq!(t.cursor(), 2);
    }

    // 验证 `c0_line_feed_inserts_newline_through_insert_newline_keymap` 场景的触发条件与预期结果。
    #[test]
    fn c0_line_feed_inserts_newline_through_insert_newline_keymap() {
        let mut t = ta_with("ab");
        t.set_cursor(/*pos*/ 1);

        t.input(KeyEvent::new(KeyCode::Char('\u{000a}'), KeyModifiers::NONE));

        assert_eq!(t.text(), "a\nb");
        assert_eq!(t.cursor(), 2);
    }

    // 验证 `c0_control_chars_respect_unbound_editor_movement` 场景的触发条件与预期结果。
    #[test]
    fn c0_control_chars_respect_unbound_editor_movement() {
        let mut t = ta_with("a\nb");
        t.set_cursor(/*pos*/ 2);
        let mut keymap = RuntimeKeymap::defaults().editor;
        Arc::make_mut(&mut keymap).move_up.clear();

        t.input_with_keymap(KeyEvent::new(KeyCode::Char('\u{0010}'), KeyModifiers::NONE), &keymap);

        assert_eq!(t.cursor(), 2);
    }

    // 验证 `c0_control_chars_respect_remapped_editor_movement` 场景的触发条件与预期结果。
    #[test]
    fn c0_control_chars_respect_remapped_editor_movement() {
        let mut t = ta_with("a\nb");
        t.set_cursor(/*pos*/ 0);
        let mut keymap = RuntimeKeymap::defaults().editor;
        Arc::make_mut(&mut keymap).move_up.clear();
        Arc::make_mut(&mut keymap).move_down = vec![crate::key_hint::ctrl(KeyCode::Char('p'))];

        t.input_with_keymap(KeyEvent::new(KeyCode::Char('\u{0010}'), KeyModifiers::NONE), &keymap);

        assert_eq!(t.cursor(), 2);
    }

    // 验证 `delete_backward_word_alt_keys` 场景的触发条件与预期结果。
    #[test]
    fn delete_backward_word_alt_keys() {
        // 测试自定义 Alt+Ctrl+h 键绑定
        // Test the custom Alt+Ctrl+h binding
        let mut t = ta_with("hello world");
        t.set_cursor(t.text().len()); // cursor at the end
        t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL | KeyModifiers::ALT));
        assert_eq!(t.text(), "hello ");
        assert_eq!(t.cursor(), 6);

        // 测试标准 Alt+Backspace 键绑定
        // Test the standard Alt+Backspace binding
        let mut t = ta_with("hello world");
        t.set_cursor(t.text().len()); // cursor at the end
        t.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(t.text(), "hello ");
        assert_eq!(t.cursor(), 6);
    }

    // 验证 `shift_backspace_and_shift_delete_keep_grapheme_delete_behavior` 场景的触发条件与预期结果。
    #[test]
    fn shift_backspace_and_shift_delete_keep_grapheme_delete_behavior() {
        let mut t = ta_with("abc");
        t.set_cursor(/*pos*/ 2);

        t.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::SHIFT));
        assert_eq!(t.text(), "ac");
        assert_eq!(t.cursor(), 1);

        let mut t = ta_with("abc");
        t.set_cursor(/*pos*/ 1);

        t.input(KeyEvent::new(KeyCode::Delete, KeyModifiers::SHIFT));
        assert_eq!(t.text(), "ac");
        assert_eq!(t.cursor(), 1);
    }

    // 验证 `control_backspace_variants_delete_backward_word` 场景的触发条件与预期结果。
    #[test]
    fn control_backspace_variants_delete_backward_word() {
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::CONTROL | KeyModifiers::SHIFT] {
            let mut t = ta_with("hello world");
            t.set_cursor(t.text().len());

            t.input(KeyEvent::new(KeyCode::Backspace, modifiers));
            assert_eq!(t.text(), "hello ");
            assert_eq!(t.cursor(), 6);
        }
    }

    // 验证 `control_delete_variants_delete_forward_word` 场景的触发条件与预期结果。
    #[test]
    fn control_delete_variants_delete_forward_word() {
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::CONTROL | KeyModifiers::SHIFT] {
            let mut t = ta_with("hello world");
            t.set_cursor(/*pos*/ 0);

            t.input(KeyEvent::new(KeyCode::Delete, modifiers));
            assert_eq!(t.text(), " world");
            assert_eq!(t.cursor(), 0);
        }
    }

    // 验证 `delete_backward_word_handles_narrow_no_break_space` 场景的触发条件与预期结果。
    #[test]
    fn delete_backward_word_handles_narrow_no_break_space() {
        let mut t = ta_with("32\u{202F}AM");
        t.set_cursor(t.text().len());
        t.input(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT));
        pretty_assertions::assert_eq!(t.text(), "32\u{202F}");
        pretty_assertions::assert_eq!(t.cursor(), t.text().len());
    }

    // 验证 `delete_forward_word_with_without_alt_modifier` 场景的触发条件与预期结果。
    #[test]
    fn delete_forward_word_with_without_alt_modifier() {
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ 0);
        t.input(KeyEvent::new(KeyCode::Delete, KeyModifiers::ALT));
        assert_eq!(t.text(), " world");
        assert_eq!(t.cursor(), 0);

        let mut t = ta_with("hello");
        t.set_cursor(/*pos*/ 0);
        t.input(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(t.text(), "ello");
        assert_eq!(t.cursor(), 0);
    }

    // 验证 `delete_forward_word_alt_d` 场景的触发条件与预期结果。
    #[test]
    fn delete_forward_word_alt_d() {
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ 6);
        t.input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::ALT));
        pretty_assertions::assert_eq!(t.text(), "hello ");
        pretty_assertions::assert_eq!(t.cursor(), 6);
    }

    // 验证 `control_h_backspace` 场景的触发条件与预期结果。
    #[test]
    fn control_h_backspace() {
        // 按 Ctrl + H 键作为回退
        // Test Ctrl+H as backspace
        let mut t = ta_with("12345");
        t.set_cursor(/*pos*/ 3); // cursor after '3'
        t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
        assert_eq!(t.text(), "1245");
        assert_eq!(t.cursor(), 2);

        // 在测试 Ctrl+H 键时（应为无操作）
        // Test Ctrl+H at beginning (should be no-op)
        t.set_cursor(/*pos*/ 0);
        t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
        assert_eq!(t.text(), "1245");
        assert_eq!(t.cursor(), 0);

        // 在文件末尾执行 Ctrl+H
        // Test Ctrl+H at end
        t.set_cursor(t.text().len());
        t.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
        assert_eq!(t.text(), "124");
        assert_eq!(t.cursor(), 3);
    }

    #[cfg_attr(not(windows), ignore = "AltGr modifier only applies on Windows")]
    // 验证 `altgr_ctrl_alt_char_inserts_literal` 场景的触发条件与预期结果。
    #[test]
    fn altgr_ctrl_alt_char_inserts_literal() {
        let mut t = ta_with("");
        t.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL | KeyModifiers::ALT));
        assert_eq!(t.text(), "c");
        assert_eq!(t.cursor(), 1);
    }

    // 验证 `cursor_vertical_movement_across_lines_and_bounds` 场景的触发条件与预期结果。
    #[test]
    fn cursor_vertical_movement_across_lines_and_bounds() {
        let mut t = ta_with("short\nloooooooooong\nmid");
        // 在第二行第 5 列处放置光标
        // Place cursor on second line, column 5
        let second_line_start = 6; // after first '\n'
        t.set_cursor(second_line_start + 5);

        // 上移：列目标保持不变，受行长限制
        // Move up: target column preserved, clamped by line length
        t.move_cursor_up();
        assert_eq!(t.cursor(), 5); // first line has len 5

        // 再次向上移动将回到文本起始处
        // Move up again goes to start of text
        t.move_cursor_up();
        assert_eq!(t.cursor(), 0);

        // 向下移动：从起始列追踪到目标列
        // Move down: from start to target col tracked
        t.move_cursor_down();
        // 第一次下移时，应落在第二行第 0 列（目标列已记住为 0）
        // On first move down, we should land on second line, at col 0 (target col remembered as 0)
        let pos_after_down = t.cursor();
        assert!(pos_after_down >= second_line_start);

        // 再次向下移动至第三行，并限制其长度
        // Move down again to third line; clamp to its length
        t.move_cursor_down();
        let third_line_start = t.text().find("mid").unwrap();
        let third_line_end = third_line_start + 3;
        assert!(t.cursor() >= third_line_start && t.cursor() <= third_line_end);

        // 最后一行向下移动将到达末尾
        // Moving down at last line jumps to end
        t.move_cursor_down();
        assert_eq!(t.cursor(), t.text().len());
    }

    // 验证 `home_end_and_emacs_style_home_end` 场景的触发条件与预期结果。
    #[test]
    fn home_end_and_emacs_style_home_end() {
        let mut t = ta_with("one\ntwo\nthree");
        // 第二行中间位置
        // Position at middle of second line
        let second_line_start = t.text().find("two").unwrap();
        t.set_cursor(second_line_start + 1);

        t.move_cursor_to_beginning_of_line(/*move_up_at_bol*/ false);
        assert_eq!(t.cursor(), second_line_start);

        // Ctrl+A 行为：在行首位置时，跳转到上一行的开始
        // Ctrl-A behavior: if at BOL, go to beginning of previous line
        t.move_cursor_to_beginning_of_line(/*move_up_at_bol*/ true);
        assert_eq!(t.cursor(), 0); // beginning of first line

        // 将光标移动到第一行末尾
        // Move to EOL of first line
        t.move_cursor_to_end_of_line(/*move_down_at_eol*/ false);
        assert_eq!(t.cursor(), 3);

        // Ctrl+E：若行尾为换行符，则跳转至下一行的末尾
        // Ctrl-E: if at EOL, go to end of next line
        t.move_cursor_to_end_of_line(/*move_down_at_eol*/ true);
        // 第二行末尾（即 "two"）在 \n 之前是结束符
        // end of second line ("two") is right before its '\n'
        let end_second_nl = t.text().find("\nthree").unwrap();
        assert_eq!(t.cursor(), end_second_nl);
    }

    // 验证 `end_of_line_or_down_at_end_of_text` 场景的触发条件与预期结果。
    #[test]
    fn end_of_line_or_down_at_end_of_text() {
        let mut t = ta_with("one\ntwo");
        // 将光标定位到文本末尾
        // Place cursor at absolute end of the text
        t.set_cursor(t.text().len());
        // 保持于结束处，不引发 panic
        // Should remain at end without panicking
        t.move_cursor_to_end_of_line(/*move_down_at_eol*/ true);
        assert_eq!(t.cursor(), t.text().len());

        // 在非最终行末尾时验证行为
        // Also verify behavior when at EOL of a non-final line:
        let eol_first_line = 3; // index of '\n' in "one\ntwo"
        t.set_cursor(eol_first_line);
        t.move_cursor_to_end_of_line(/*move_down_at_eol*/ true);
        assert_eq!(t.cursor(), t.text().len()); // moves to end of next (last) line
    }

    // 验证 `word_navigation_helpers` 场景的触发条件与预期结果。
    #[test]
    fn word_navigation_helpers() {
        let t = ta_with("  alpha  beta   gamma");
        let mut t = t; // make mutable for set_cursor
        // 在"alpha"之后放置光标
        // Put cursor after "alpha"
        let after_alpha = t.text().find("alpha").unwrap() + "alpha".len();
        t.set_cursor(after_alpha);
        assert_eq!(t.beginning_of_previous_word(), 2); // skip initial spaces

        // 将光标置于 Beta 的起始位置
        // Put cursor at start of beta
        let beta_start = t.text().find("beta").unwrap();
        t.set_cursor(beta_start);
        assert_eq!(t.end_of_next_word(), beta_start + "beta".len());

        // 如果结束，则返回下一个单词的结束位置长度
        // If at end, end_of_next_word returns len
        t.set_cursor(t.text().len());
        assert_eq!(t.end_of_next_word(), t.text().len());
    }

    // 验证 `word_navigation_cjk_each_char_is_boundary` 场景的触发条件与预期结果。
    #[test]
    fn word_navigation_cjk_each_char_is_boundary() {
        let text = "你好世界";
        let mut t = ta_with(text);

        t.set_cursor(/*pos*/ text.len());
        assert_eq!(t.beginning_of_previous_word(), 9);

        t.set_cursor(/*pos*/ 9);
        assert_eq!(t.beginning_of_previous_word(), 6);

        t.set_cursor(/*pos*/ 6);
        assert_eq!(t.beginning_of_previous_word(), 3);

        t.set_cursor(/*pos*/ 3);
        assert_eq!(t.beginning_of_previous_word(), 0);
    }

    // 验证 `word_navigation_cjk_forward` 场景的触发条件与预期结果。
    #[test]
    fn word_navigation_cjk_forward() {
        let text = "你好世界";
        let mut t = ta_with(text);

        t.set_cursor(/*pos*/ 0);
        assert_eq!(t.end_of_next_word(), 3);

        t.set_cursor(/*pos*/ 3);
        assert_eq!(t.end_of_next_word(), 6);

        t.set_cursor(/*pos*/ 6);
        assert_eq!(t.end_of_next_word(), 9);

        t.set_cursor(/*pos*/ 9);
        assert_eq!(t.end_of_next_word(), 12);
    }

    // 验证 `word_navigation_mixed_ascii_cjk` 场景的触发条件与预期结果。
    #[test]
    fn word_navigation_mixed_ascii_cjk() {
        let text = "hello你好";
        let mut t = ta_with(text);

        t.set_cursor(/*pos*/ 0);
        assert_eq!(t.end_of_next_word(), 5);

        t.set_cursor(/*pos*/ 5);
        assert_eq!(t.end_of_next_word(), 8);

        t.set_cursor(/*pos*/ text.len());
        assert_eq!(t.beginning_of_previous_word(), 8);

        t.set_cursor(/*pos*/ 8);
        assert_eq!(t.beginning_of_previous_word(), 5);

        t.set_cursor(/*pos*/ 5);
        assert_eq!(t.beginning_of_previous_word(), 0);
    }

    // 验证 `word_navigation_preserves_separator_breaks_within_unicode_segments` 场景的触发条件与预期结果。
    #[test]
    fn word_navigation_preserves_separator_breaks_within_unicode_segments() {
        let mut t = ta_with("can't 32.3 foo.bar");

        t.set_cursor(/*pos*/ 5);
        assert_eq!(t.beginning_of_previous_word(), 4);

        t.set_cursor(/*pos*/ 4);
        assert_eq!(t.beginning_of_previous_word(), 3);

        t.set_cursor(/*pos*/ 10);
        assert_eq!(t.beginning_of_previous_word(), 9);

        t.set_cursor(/*pos*/ 18);
        assert_eq!(t.beginning_of_previous_word(), 15);
    }

    // 验证 `wrapping_and_cursor_positions` 场景的触发条件与预期结果。
    #[test]
    fn wrapping_and_cursor_positions() {
        let mut t = ta_with("hello world here");
        let area = Rect::new(0, 0, 6, 10); // width 6 -> wraps words
        // 期望高度将包裹多行
        // desired height counts wrapped lines
        assert!(t.desired_height(area.width) >= 3);

        // 在"世界"中放置光标
        // Place cursor in "world"
        let world_start = t.text().find("world").unwrap();
        t.set_cursor(world_start + 3);
        let (_x, y) = t.cursor_pos(area).unwrap();
        assert_eq!(y, 1); // world should be on second wrapped line

        // 状态与较小高度下，光标映射到可见行
        // With state and small height, cursor is mapped onto visible row
        let mut state = TextAreaState::default();
        let small_area = Rect::new(0, 0, 6, 1);
        // 第一调用：光标不可见，因此有效滚动确保其可见
        // First call: cursor not visible -> effective scroll ensures it is
        let (_x, y) = t.cursor_pos_with_state(small_area, state).unwrap();
        assert_eq!(y, 0);

        // 以状态渲染实际滚动值
        // Render with state to update actual scroll value
        let mut buf = Buffer::empty(small_area);
        ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), small_area, &mut buf, &mut state);
        // 渲染完成后，滚动状态应调整，使光标行与内容对齐
        // After render, state.scroll should be adjusted so cursor row fits
        let effective_lines = t.desired_height(small_area.width);
        assert!(state.scroll < effective_lines);
    }

    // 验证 `cursor_wraps_after_filling_the_final_line` 场景的触发条件与预期结果。
    #[test]
    fn cursor_wraps_after_filling_the_final_line() {
        for text in ["abad", "界界", "abｶﾞ"] {
            let mut t = ta_with(text);
            t.set_cursor(t.text().len());
            let area = Rect::new(2, 3, /*width*/ 4, /*height*/ 2);

            assert_eq!(t.desired_height(area.width), 2);
            assert_eq!(t.cursor_pos(area), Some((2, 4)));
        }
    }

    // 验证 `full_non_final_lines_reserve_visible_cursor_rows` 场景的触发条件与预期结果。
    #[test]
    fn full_non_final_lines_reserve_visible_cursor_rows() {
        for text in ["abad\nef", "界界\nef", "abｶﾞ\nef"] {
            let mut t = ta_with(text);
            let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
            let newline = text.find('\n').unwrap();

            assert_eq!(t.desired_height(area.width), 3);

            t.set_cursor(newline);
            assert_eq!(t.cursor_pos(area), Some((0, 1)));

            t.set_cursor(newline + 1);
            assert_eq!(t.cursor_pos(area), Some((0, 2)));
        }
    }

    // 验证 `full_non_final_line_cursor_snapshot` 场景的触发条件与预期结果。
    #[test]
    fn full_non_final_line_cursor_snapshot() {
        let mut t = ta_with("abad\nef");
        t.set_cursor(/*pos*/ 4);
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();

        insta::assert_snapshot!(
            "textarea_full_non_final_line_has_a_visible_cursor_row",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );
    }

    // 验证 `trailing_spaces_wrap_without_letting_the_cursor_escape` 场景的触发条件与预期结果。
    #[test]
    fn trailing_spaces_wrap_without_letting_the_cursor_escape() {
        let mut t = ta_with("abad        ");
        t.set_cursor(t.text().len());
        let area = Rect::new(0, 0, /*width*/ 5, /*height*/ 3);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();

        assert_eq!(t.desired_height(area.width), 3);
        insta::assert_snapshot!(
            "textarea_trailing_spaces_wrap_without_cursor_overflow",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );
    }

    // 验证 `separator_after_full_line_hangs_before_following_text` 场景的触发条件与预期结果。
    #[test]
    fn separator_after_full_line_hangs_before_following_text() {
        let mut t = ta_with("abad a");
        t.set_cursor(t.text().len());
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 2);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();

        assert_eq!(t.desired_height(area.width), 2);
        assert_eq!(t.cursor_pos(area), Some((1, 1)));
        insta::assert_snapshot!(
            "textarea_separator_after_full_line_hangs_before_following_text",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );

        for text in ["界界 a", "abｶﾞ a", "abad\ta"] {
            let mut t = ta_with(text);
            t.set_cursor(t.text().len());

            assert_eq!(t.desired_height(area.width), 2);
            assert_eq!(t.cursor_pos(area), Some((1, 1)));
        }
    }

    // 验证 `breakable_unicode_space_hangs_before_following_text` 场景的触发条件与预期结果。
    #[test]
    fn breakable_unicode_space_hangs_before_following_text() {
        let mut t = ta_with("abad\u{3000}abcde");
        t.set_cursor(t.text().len());
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();

        assert_eq!(t.desired_height(area.width), 3);
        assert_eq!(t.cursor_pos(area), Some((1, 2)));
        insta::assert_snapshot!(
            "textarea_breakable_unicode_space_hangs_before_following_text",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );
    }

    // 验证 `leading_space_reflows_across_following_wrapped_fragments` 场景的触发条件与预期结果。
    #[test]
    fn leading_space_reflows_across_following_wrapped_fragments() {
        let mut t = ta_with("abad abcde wxyz");
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 5);
        assert_eq!(t.desired_height(area.width), 5);
        for (cursor, expected) in [(11, (0, 3)), (14, (3, 3)), (15, (0, 4))] {
            t.set_cursor(cursor);
            assert_eq!(t.cursor_pos(area), Some(expected));
        }

        let mut t = ta_with("abad abcde xy zz");
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 4);
        assert_eq!(t.desired_height(area.width), 4);
        for (cursor, expected) in [
            (4, (0, 1)),
            (5, (0, 1)),
            (8, (3, 1)),
            (9, (0, 2)),
            (10, (1, 2)),
            (11, (2, 2)),
            (12, (3, 2)),
            (13, (0, 3)),
            (14, (0, 3)),
            (16, (2, 3)),
        ] {
            t.set_cursor(cursor);
            assert_eq!(t.cursor_pos(area), Some(expected));
        }

        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();

        insta::assert_snapshot!(
            "textarea_leading_space_reflows_across_following_wrapped_fragments",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );
    }

    // 验证 `reflow_keeps_fitting_remainder_on_current_row` 场景的触发条件与预期结果。
    #[test]
    fn reflow_keeps_fitting_remainder_on_current_row() {
        for (text, width, expected_height, expected_cursor) in [
            ("  -", 1, 4, (0, 3)),
            ("p    a b", 3, 3, (0, 2)),
            ("p     a b", 5, 2, (3, 1)),
            (" aaaa a   ", 3, 4, (1, 3)),
        ] {
            let mut t = ta_with(text);
            t.set_cursor(t.text().len());
            let area = Rect::new(0, 0, width, expected_height);

            assert_eq!(t.desired_height(area.width), expected_height);
            assert_eq!(t.cursor_pos(area), Some(expected_cursor));
            for range in t.wrapped_lines(width).iter() {
                assert!(
                    display_width(text[range.start..range.end - 1].trim_end_matches(' '))
                        <= usize::from(width)
                );
            }
        }

        let mut t = ta_with("p     a b");
        t.set_cursor(t.text().len());
        let area = Rect::new(0, 0, /*width*/ 5, /*height*/ 2);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();

        insta::assert_snapshot!(
            "textarea_reflow_keeps_fitting_remainder_on_current_row",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );
    }

    // 验证 `reflow_preserves_hyphenated_word_breakpoint_when_rendering` 场景的触发条件与预期结果。
    #[test]
    fn reflow_preserves_hyphenated_word_breakpoint_when_rendering() {
        let mut t = ta_with("a foo-barbaz");
        t.set_cursor(t.text().len());
        let area = Rect::new(0, 0, /*width*/ 10, /*height*/ 2);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();

        insta::assert_snapshot!(
            "textarea_reflow_preserves_hyphenated_word_breakpoint",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );
    }

    // 验证 `space_runs_hang_before_later_text` 场景的触发条件与预期结果。
    #[test]
    fn space_runs_hang_before_later_text() {
        let mut t = ta_with("abad     next");
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);

        assert_eq!(t.desired_height(area.width), 3);

        for cursor in 4..=9 {
            t.set_cursor(cursor);
            assert_eq!(t.cursor_pos(area), Some((0, 1)));
        }
    }

    // 验证 `full_final_line_scrolls_to_its_visible_cursor_row` 场景的触发条件与预期结果。
    #[test]
    fn full_final_line_scrolls_to_its_visible_cursor_row() {
        let mut t = ta_with("abcdefgh");
        t.set_cursor(t.text().len());
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 2);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                StatefulWidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut(), &mut state);
            })
            .unwrap();

        assert_eq!(t.desired_height(area.width), 3);
        insta::assert_snapshot!(
            "textarea_full_final_line_scrolls_to_visible_cursor_row",
            format!(
                "scroll: {}\ncursor: {:?}\n{}",
                state.scroll,
                t.cursor_pos_with_state(area, state),
                terminal.backend()
            )
        );
    }

    // 验证 `text_rendering_does_not_overwrite_cells_outside_the_textarea` 场景的触发条件与预期结果。
    #[test]
    fn text_rendering_does_not_overwrite_cells_outside_the_textarea() {
        let t = ta_with("abad     next");
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
        let mut state = TextAreaState::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, /*width*/ 8, /*height*/ 3));
        let protected_style = Style::default().fg(Color::Red);
        buf.set_style(Rect::new(4, 0, /*width*/ 4, /*height*/ 3), protected_style);
        let protected_cell = buf[(4, 0)].clone();

        StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);

        assert_eq!(buf[(4, 0)], protected_cell);
    }

    // 验证 `styled_text_rendering_does_not_overwrite_cells_outside_the_textarea` 场景的触发条件与预期结果。
    #[test]
    fn styled_text_rendering_does_not_overwrite_cells_outside_the_textarea() {
        let mut snapshots = Vec::new();
        for overlay in ["element", "highlight"] {
            let mut t = ta_with("abad     next");
            if overlay == "element" {
                t.add_element_range(0..9);
            }
            let highlights = if overlay == "highlight" {
                vec![(0..9, Style::default().fg(Color::Magenta))]
            } else {
                Vec::new()
            };
            let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 3);
            let mut terminal = Terminal::new(TestBackend::new(/*width*/ 8, area.height)).unwrap();
            let mut state = TextAreaState::default();
            terminal
                .draw(|frame| {
                    frame.buffer_mut().set_string(
                        /*x*/ 4,
                        /*y*/ 0,
                        "KEEP",
                        Style::default().fg(Color::Red),
                    );
                    t.render_ref_styled_with_highlights(
                        area,
                        frame.buffer_mut(),
                        &mut state,
                        Style::default(),
                        &highlights,
                    );
                })
                .unwrap();

            assert_eq!(terminal.backend().buffer()[(4, 0)].symbol(), "K");
            snapshots.push(format!("{overlay}\n{}", terminal.backend()));
        }

        insta::assert_snapshot!(
            "textarea_styled_overlays_stay_inside_the_textarea",
            snapshots.join("\n\n")
        );
    }

    // 验证 `empty_textarea_areas_do_not_have_a_cursor` 场景的触发条件与预期结果。
    #[test]
    fn empty_textarea_areas_do_not_have_a_cursor() {
        let t = ta_with("abad");

        assert_eq!(t.cursor_pos(Rect::new(0, 0, /*width*/ 0, /*height*/ 1)), None);
        assert_eq!(t.cursor_pos(Rect::new(0, 0, /*width*/ 1, /*height*/ 0)), None);
    }

    // 验证 `render_highlights_apply_style_without_mutating_text` 场景的触发条件与预期结果。
    #[test]
    fn render_highlights_apply_style_without_mutating_text() {
        let t = ta_with("hello world");
        let area = Rect::new(0, 0, 20, 1);
        let mut state = TextAreaState::default();
        let mut buf = Buffer::empty(area);
        let highlight_style = Style::default().add_modifier(ratatui::style::Modifier::REVERSED);

        t.render_ref_styled_with_highlights(
            area,
            &mut buf,
            &mut state,
            Style::default(),
            &[(6..11, highlight_style)],
        );

        assert_eq!(t.text(), "hello world");
        assert!(!buf[(0, 0)].style().add_modifier.contains(ratatui::style::Modifier::REVERSED));
        assert!(buf[(6, 0)].style().add_modifier.contains(ratatui::style::Modifier::REVERSED));
        assert!(buf[(10, 0)].style().add_modifier.contains(ratatui::style::Modifier::REVERSED));
    }

    // 验证 `tabs_render_as_spaces_and_align_with_cursor_snapshot` 场景的触发条件与预期结果。
    #[test]
    fn tabs_render_as_spaces_and_align_with_cursor_snapshot() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let text = "❌\tSimulation\tformatter[large/dataset.py]\t7.4 ms\t8.1 ms\t-8.29%";
        let mut t = ta_with(text);
        t.set_cursor(text.len());

        let mut terminal = Terminal::new(TestBackend::new(/*width*/ 100, /*height*/ 1)).unwrap();
        terminal
            .draw(|frame| {
                ratatui::widgets::WidgetRef::render_ref(&(&t), frame.area(), frame.buffer_mut());
            })
            .unwrap();

        let cursor = t.cursor_pos(terminal.backend().buffer().area).unwrap();
        assert_eq!(terminal.backend().buffer()[(cursor.0 - 1, cursor.1)].symbol(), "%");
        insta::assert_snapshot!(
            "textarea_tabs_render_as_spaces_and_align_with_cursor",
            format!("cursor: {cursor:?}\n{}", terminal.backend())
        );
    }

    // 验证 `tabs_wrap_at_their_rendered_width` 场景的触发条件与预期结果。
    #[test]
    fn tabs_wrap_at_their_rendered_width() {
        let text = "1234\t5";
        let mut t = ta_with(text);
        t.set_cursor(text.len());
        let area = Rect::new(0, 0, /*width*/ 5, /*height*/ 2);
        let mut buf = Buffer::empty(area);

        ratatui::widgets::WidgetRef::render_ref(&(&t), area, &mut buf);

        assert_eq!(t.desired_height(area.width), 2);
        assert_eq!(t.cursor_pos(area), Some((1, 1)));
        assert_eq!(buf[(0, 1)].symbol(), "5");
    }

    // 验证 `halfwidth_sound_marks_wrap_and_align_with_the_cursor` 场景的触发条件与预期结果。
    #[test]
    fn halfwidth_sound_marks_wrap_and_align_with_the_cursor() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut snapshots = Vec::new();
        for (label, text, width, height, cursor, cells) in [
            ("dakuten", "12ｶﾞx", 4, 2, (1, 1), [(2, 0, "ｶﾞ"), (0, 1, "x")]),
            ("handakuten", "abﾊﾟc", 3, 3, (0, 2), [(0, 1, "ﾊﾟ"), (2, 1, "c")]),
            ("standalone", "a ﾞb", 2, 3, (0, 2), [(0, 1, "ﾞ"), (1, 1, "b")]),
        ] {
            let mut t = ta_with(text);
            t.set_cursor(text.len());
            let area = Rect::new(0, 0, width, height);
            let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
            terminal
                .draw(|frame| {
                    ratatui::widgets::WidgetRef::render_ref(
                        &(&t),
                        frame.area(),
                        frame.buffer_mut(),
                    );
                })
                .unwrap();

            assert_eq!(t.desired_height(area.width), height);
            assert_eq!(t.cursor_pos(area), Some(cursor));
            for (x, y, expected) in cells {
                assert_eq!(terminal.backend().buffer()[(x, y)].symbol(), expected);
            }
            snapshots.push(format!(
                "{label}\ncursor: {:?}\n{}",
                t.cursor_pos(area),
                terminal.backend()
            ));
        }

        insta::assert_snapshot!(
            "textarea_halfwidth_sound_marks_wrap_and_align_with_cursor",
            snapshots.join("\n\n")
        );
    }

    // 验证 `masked_graphemes_align_with_the_cursor` 场景的触发条件与预期结果。
    #[test]
    fn masked_graphemes_align_with_the_cursor() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let text = "界ﾞa";
        let mut t = ta_with(text);
        t.set_cursor(text.len());
        let area = Rect::new(0, 0, /*width*/ 4, /*height*/ 2);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        let mut state = TextAreaState::default();
        terminal
            .draw(|frame| {
                t.render_ref_masked(frame.area(), frame.buffer_mut(), &mut state, '*');
            })
            .unwrap();

        assert_eq!(t.cursor_pos(area), Some((0, 1)));
        insta::assert_snapshot!(
            "textarea_masked_graphemes_align_with_cursor",
            format!("cursor: {:?}\n{}", t.cursor_pos(area), terminal.backend())
        );
    }

    // 验证 `overwide_halfwidth_sound_marks_keep_the_final_cursor_visible` 场景的触发条件与预期结果。
    #[test]
    fn overwide_halfwidth_sound_marks_keep_the_final_cursor_visible() {
        let area = Rect::new(0, 0, /*width*/ 2, /*height*/ 3);

        for grapheme in ["ｶﾞﾞ", "界ﾞ"] {
            let text = format!("{grapheme}ab");
            let mut t = ta_with(&text);
            t.set_cursor(text.len());

            assert_eq!(t.desired_height(area.width), 3);
            assert_eq!(t.cursor_pos(area), Some((0, 2)));

            t.set_cursor(grapheme.len());
            assert_eq!(t.cursor_pos(area), Some((0, 1)));
        }
    }

    // 验证 `cursor_pos_with_state_basic_and_scroll_behaviors` 场景的触发条件与预期结果。
    #[test]
    fn cursor_pos_with_state_basic_and_scroll_behaviors() {
        // 第 1 项：无需包裹，高度适配——滚动被忽略，Y 值直接映射。
        // Case 1: No wrapping needed, height fits — scroll ignored, y maps directly.
        let mut t = ta_with("hello world");
        t.set_cursor(/*pos*/ 3);
        let area = Rect::new(2, 5, 20, 3);
        // 即使提供荒谬的滚动，当内容填满区域时，有效滚动量为 0，且光标位置与 cursor_pos 一致。
        // Even if an absurd scroll is provided, when content fits the area the
        // effective scroll is 0 and the cursor position matches cursor_pos.
        let bad_state = TextAreaState { scroll: 999 };
        let (x1, y1) = t.cursor_pos(area).unwrap();
        let (x2, y2) = t.cursor_pos_with_state(area, bad_state).unwrap();
        assert_eq!((x2, y2), (x1, y1));

        // 第 2 步：当前窗口下方光标——y 值应在调整有效滚动后，被限制在高度为 (area.height - 1) 的行（即最后一行）
        // Case 2: Cursor below the current window — y should be clamped to the
        // bottom row (area.height - 1) after adjusting effective scroll.
        let mut t = ta_with("one two three four five six");
        // 强制将内容包裹在多条视觉行中。
        // Force wrapping to many visual lines.
        let wrap_width = 4;
        let _ = t.desired_height(wrap_width);
        // 将光标放置在窗口末尾附近，确保其始终位于第一个窗口之下。
        // Put cursor somewhere near the end so it's definitely below the first window.
        t.set_cursor(t.text().len().saturating_sub(2));
        let small_area = Rect::new(0, 0, wrap_width, 2);
        let state = TextAreaState { scroll: 0 };
        let (_x, y) = t.cursor_pos_with_state(small_area, state).unwrap();
        assert_eq!(y, small_area.y + small_area.height - 1);

        // 当提供的滚动量过大时，当前窗口上方的光标应位于第一行（索引为 0）
        // Case 3: Cursor above the current window — y should be top row (0)
        // when the provided scroll is too large.
        let mut t = ta_with("alpha beta gamma delta epsilon zeta");
        let wrap_width = 5;
        let lines = t.desired_height(wrap_width);
        // 将光标置于起始位置，使过度滚动将其移至第一行。
        // Place cursor near start so an excessive scroll moves it to top row.
        t.set_cursor(/*pos*/ 1);
        let area = Rect::new(0, 0, wrap_width, 3);
        let state = TextAreaState { scroll: lines.saturating_mul(2) };
        let (_x, y) = t.cursor_pos_with_state(area, state).unwrap();
        assert_eq!(y, area.y);
    }

    // 验证 `wrapped_navigation_across_visual_lines` 场景的触发条件与预期结果。
    #[test]
    fn wrapped_navigation_across_visual_lines() {
        let mut t = ta_with("abcdefghij");
        // 强制在宽度 4 处包裹：行 -> ['abcd', 'efgh', 'ij']
        // Force wrapping at width 4: lines -> ["abcd", "efgh", "ij"]
        let _ = t.desired_height(/*width*/ 4);

        // 从开始，向下移动应前往下一行（索引 4）的起始位置
        // From the very start, moving down should go to the start of the next wrapped line (index 4)
        t.set_cursor(/*pos*/ 0);
        t.move_cursor_down();
        assert_eq!(t.cursor(), 4);

        // 在索引 4 处应显示第二个被包裹行开始的 Cursor
        // Cursor at boundary index 4 should be displayed at start of second wrapped line
        t.set_cursor(/*pos*/ 4);
        let area = Rect::new(0, 0, 4, 10);
        let (x, y) = t.cursor_pos(area).unwrap();
        assert_eq!((x, y), (0, 1));

        // 在状态和较小的高度下，光标应在第 0 行第 0 列可见
        // With state and small height, cursor should be visible at row 0, col 0
        let small_area = Rect::new(0, 0, 4, 1);
        let state = TextAreaState::default();
        let (x, y) = t.cursor_pos_with_state(small_area, state).unwrap();
        assert_eq!((x, y), (0, 0));

        // 在 "efgh" 第二个被包裹的行中间放置光标，在字符 'g' 处
        // Place cursor in the middle of the second wrapped line ("efgh"), at 'g'
        t.set_cursor(/*pos*/ 6);
        // 上一行向上移动应停留在上一行的相同列上，索引为 2（字符 'c'）
        // Move up should go to same column on previous wrapped line -> index 2 ('c')
        t.move_cursor_up();
        assert_eq!(t.cursor(), 2);

        // 向下移动应返回到下一次行末尾的相同位置，即索引 6（'g'）
        // Move down should return to same position on the next wrapped line -> back to index 6 ('g')
        t.move_cursor_down();
        assert_eq!(t.cursor(), 6);

        // 再次向下移动应到达第三行。目标列数为 2，但该行长度为 2，因此限制为末尾
        // Move down again should go to third wrapped line. Target col is 2, but the line has len 2 -> clamp to end
        t.move_cursor_down();
        assert_eq!(t.cursor(), t.text().len());
    }

    // 验证 `cursor_pos_with_state_after_movements` 场景的触发条件与预期结果。
    #[test]
    fn cursor_pos_with_state_after_movements() {
        let mut t = ta_with("abcdefghij");
        // 将宽度 4 包裹为视觉行：abcd | efgh | ij
        // Wrap width 4 -> visual lines: abcd | efgh | ij
        let _ = t.desired_height(/*width*/ 4);
        let area = Rect::new(0, 0, 4, 2);
        let mut state = TextAreaState::default();
        let mut buf = Buffer::empty(area);

        // 从开始处开始
        // Start at beginning
        t.set_cursor(/*pos*/ 0);
        ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
        let (x, y) = t.cursor_pos_with_state(area, state).unwrap();
        assert_eq!((x, y), (0, 0));

        // 移动到第二行视觉线；应在视窗内第 1 行的最底部
        // Move down to second visual line; should be at bottom row (row 1) within 2-line viewport
        t.move_cursor_down();
        ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
        let (x, y) = t.cursor_pos_with_state(area, state).unwrap();
        assert_eq!((x, y), (0, 1));

        // 向下移动到第三行视觉线；视口滚动并保持在最底部一行
        // Move down to third visual line; viewport scrolls and keeps cursor on bottom row
        t.move_cursor_down();
        ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
        let (x, y) = t.cursor_pos_with_state(area, state).unwrap();
        assert_eq!((x, y), (0, 1));

        // 向上移动至第二行视觉线；在当前滚动状态下，该行出现在最上方
        // Move up to second visual line; with current scroll, it appears on top row
        t.move_cursor_up();
        ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
        let (x, y) = t.cursor_pos_with_state(area, state).unwrap();
        assert_eq!((x, y), (0, 0));

        // 跨移动保持列：第一行设为第 2 列，向下移动
        // Column preservation across moves: set to col 2 on first line, move down
        t.set_cursor(/*pos*/ 2);
        ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
        let (x0, y0) = t.cursor_pos_with_state(area, state).unwrap();
        assert_eq!((x0, y0), (2, 0));
        t.move_cursor_down();
        ratatui::widgets::StatefulWidgetRef::render_ref(&(&t), area, &mut buf, &mut state);
        let (x1, y1) = t.cursor_pos_with_state(area, state).unwrap();
        assert_eq!((x1, y1), (2, 1));
    }

    // 验证 `wrapped_navigation_with_newlines_and_spaces` 场景的触发条件与预期结果。
    #[test]
    fn wrapped_navigation_with_newlines_and_spaces() {
        // 包含空格并显式换行以测试边界
        // Include spaces and an explicit newline to exercise boundaries
        let mut t = ta_with("word1  word2\nword3");
        // 第一行有两个分隔符，宽度为 6。
        // Width 6 leaves both separators on the first visual row.
        let _ = t.desired_height(/*width*/ 6);

        // 将光标定位到单词 'word2' 的第 1 列。
        // Put the cursor at column 1 of "word2".
        let start_word2 = t.text().find("word2").unwrap();
        t.set_cursor(start_word2 + 1);

        // 向上移动直接指向前一行，并保留列对齐。
        // Up moves directly to the preceding visual line while preserving the visual column.
        t.move_cursor_up();
        assert_eq!(t.cursor(), 1);

        // Down 返回到与 'word2' 相同的视觉列。
        // Down returns to the same visual column of "word2".
        t.move_cursor_down();
        assert_eq!(t.cursor(), start_word2 + 1);

        // 向下跨越逻辑换行符，到达下一个视觉行（"word3"）
        // Down crosses the logical newline to the next visual line ("word3").
        t.move_cursor_down();
        let start_word3 = t.text().find("word3").unwrap();
        assert_eq!(t.cursor(), start_word3 + 1);
    }

    // 验证 `wrapped_navigation_with_wide_graphemes` 场景的触发条件与预期结果。
    #[test]
    fn wrapped_navigation_with_wide_graphemes() {
        // 四指向上，每个显示宽度为 2，宽度为 3 以强制在图元边界内包裹
        // Four thumbs up, each of display width 2, with width 3 to force wrapping inside grapheme boundaries
        let mut t = ta_with("👍👍👍👍");
        let _ = t.desired_height(/*width*/ 3);

        // 在第二个 Emoji 之后放置光标
        // Put cursor after the second emoji (which should be on first wrapped line)
        t.set_cursor("👍👍".len());

        // 向下移动应到达下一行开始的位置（保持列对齐，但被限制）
        // Move down should go to the start of the next wrapped line (same column preserved but clamped)
        t.move_cursor_down();
        // 我们期望在第三个 Emoji 中着陆，或者在其开头
        // We expect to land somewhere within the third emoji or at the start of it
        let pos_after_down = t.cursor();
        assert!(pos_after_down >= "👍👍".len());

        // 向上移动将使我们回到原始位置
        // Moving up should take us back to the original position
        t.move_cursor_up();
        assert_eq!(t.cursor(), "👍👍".len());
    }

    #[cfg(any())]
    // 验证 `fuzz_textarea_randomized` 场景的触发条件与预期结果。
    #[test]
    fn fuzz_textarea_randomized() {
        // 确定种子用于可重复性 在太平洋时区（PST/PDT）中根据当前日期初始化随机数生成器。这确保了 fuzz 测试在一天内保持确定性，同时仍随天而异以提高覆盖率。
        // Deterministic seed for reproducibility
        // Seed the RNG based on the current day in Pacific Time (PST/PDT). This
        // keeps the fuzz test deterministic within a day while still varying
        // day-to-day to improve coverage.
        let pst_today_seed: u64 = (chrono::Utc::now() - chrono::Duration::hours(8))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp() as u64;
        let mut rng = rand::rngs::StdRng::seed_from_u64(pst_today_seed);

        for _case in 0..500 {
            let mut ta = TextArea::new();
            let mut state = TextAreaState::default();
            // 跟踪插入的元素载荷。载荷使用字符'['和']'，这些字符rand_grapheme()不会生成，以避免意外的碰撞。
            // Track element payloads we insert. Payloads use characters '[' and ']' which
            // are not produced by rand_grapheme(), avoiding accidental collisions.
            let mut elem_texts: Vec<String> = Vec::new();
            let mut next_elem_id: usize = 0;
            // 以随机基础字符串开始
            // Start with a random base string
            let base_len = rng.random_range(0..30);
            let mut base = String::new();
            for _ in 0..base_len {
                base.push_str(&rand_grapheme(&mut rng));
            }
            ta.set_text_clearing_elements(&base);
            // 选择初始光标的有效字符边界
            // Choose a valid char boundary for initial cursor
            let mut boundaries: Vec<usize> = vec![0];
            boundaries.extend(ta.text().char_indices().map(|(i, _)| i).skip(1));
            boundaries.push(ta.text().len());
            let init = boundaries[rng.random_range(0..boundaries.len())];
            ta.set_cursor(init);

            let mut width: u16 = rng.random_range(1..=12);
            let mut height: u16 = rng.random_range(1..=4);

            for _step in 0..60 {
                // 宽度与高度通常稳定，偶尔会发生变化
                // Mostly stable width/height, occasionally change
                if rng.random_bool(0.1) {
                    width = rng.random_range(1..=12);
                }
                if rng.random_bool(0.1) {
                    height = rng.random_range(1..=4);
                }

                // 选择操作
                // Pick an operation
                match rng.random_range(0..18) {
                    0 => {
                        // 在光标处插入一个小随机字符串
                        // insert small random string at cursor
                        let len = rng.random_range(0..6);
                        let mut s = String::new();
                        for _ in 0..len {
                            s.push_str(&rand_grapheme(&mut rng));
                        }
                        ta.insert_str(&s);
                    }
                    1 => {
                        // 使用小随机切片替换
                        // replace_range with small random slice
                        let mut b: Vec<usize> = vec![0];
                        b.extend(ta.text().char_indices().map(|(i, _)| i).skip(1));
                        b.push(ta.text().len());
                        let i1 = rng.random_range(0..b.len());
                        let i2 = rng.random_range(0..b.len());
                        let (start, end) =
                            if b[i1] <= b[i2] { (b[i1], b[i2]) } else { (b[i2], b[i1]) };
                        let insert_len = rng.random_range(0..=4);
                        let mut s = String::new();
                        for _ in 0..insert_len {
                            s.push_str(&rand_grapheme(&mut rng));
                        }
                        let before = ta.text().len();
                        // 如果选择的范围与元素相交，则 replace_range 将扩展至元素边界，因此默认的增量大小断言不再成立。
                        // If the chosen range intersects an element, replace_range will expand to
                        // element boundaries, so the naive size delta assertion does not hold.
                        let intersects_element = elem_texts.iter().any(|payload| {
                            if let Some(pstart) = ta.text().find(payload) {
                                let pend = pstart + payload.len();
                                pstart < end && pend > start
                            } else {
                                false
                            }
                        });
                        ta.replace_range(start..end, &s);
                        if !intersects_element {
                            let after = ta.text().len();
                            assert_eq!(
                                after as isize,
                                before as isize + (s.len() as isize) - ((end - start) as isize)
                            );
                        }
                    }
                    2 => ta.delete_backward(rng.random_range(0..=3)),
                    3 => ta.delete_forward(rng.random_range(0..=3)),
                    4 => ta.delete_backward_word(),
                    5 => ta.kill_to_beginning_of_line(),
                    6 => ta.kill_to_end_of_line(),
                    7 => ta.move_cursor_left(),
                    8 => ta.move_cursor_right(),
                    9 => ta.move_cursor_up(),
                    10 => ta.move_cursor_down(),
                    11 => ta.move_cursor_to_beginning_of_line(/*move_up_at_bol*/ true),
                    12 => ta.move_cursor_to_end_of_line(/*move_down_at_eol*/ true),
                    13 => {
                        // 插入一个具有唯一占位符 Payload 的元素
                        // Insert an element with a unique sentinel payload
                        let payload =
                            format!("[[EL#{}:{}]]", next_elem_id, rng.random_range(1000..9999));
                        next_elem_id += 1;
                        ta.insert_element(&payload);
                        elem_texts.push(payload);
                    }
                    14 => {
                        // 在现有元素内部尝试插入内容（应限制到边界）
                        // Try inserting inside an existing element (should clamp to boundary)
                        if let Some(payload) = elem_texts.choose(&mut rng).cloned()
                            && let Some(start) = ta.text().find(&payload)
                        {
                            let end = start + payload.len();
                            if end - start > 2 {
                                let pos = rng.random_range(start + 1..end - 1);
                                let ins = rand_grapheme(&mut rng);
                                ta.insert_str_at(pos, &ins);
                            }
                        }
                    }
                    15 => {
                        // 将包含元素范围的替换整个元素
                        // Replace a range that intersects an element -> whole element should be replaced
                        if let Some(payload) = elem_texts.choose(&mut rng).cloned()
                            && let Some(start) = ta.text().find(&payload)
                        {
                            let end = start + payload.len();
                            // 创建重叠的区间 [start-δ, end-δ2)
                            // Create an intersecting range [start-δ, end-δ2)
                            let mut s = start.saturating_sub(rng.random_range(0..=2));
                            let mut e = (end + rng.random_range(0..=2)).min(ta.text().len());
                            // 将字符串对齐以满足 replace_range 契约
                            // Align to char boundaries to satisfy String::replace_range contract
                            let txt = ta.text();
                            while s > 0 && !txt.is_char_boundary(s) {
                                s -= 1;
                            }
                            while e < txt.len() && !txt.is_char_boundary(e) {
                                e += 1;
                            }
                            if s < e {
                                // 小替换文本
                                // Small replacement text
                                let mut srep = String::new();
                                for _ in 0..rng.random_range(0..=2) {
                                    srep.push_str(&rand_grapheme(&mut rng));
                                }
                                ta.replace_range(s..e, &srep);
                            }
                        }
                    }
                    16 => {
                        // 尝试将光标定位到元素内部；它应限制其范围
                        // Try setting the cursor to a position inside an element; it should clamp out
                        if let Some(payload) = elem_texts.choose(&mut rng).cloned()
                            && let Some(start) = ta.text().find(&payload)
                        {
                            let end = start + payload.len();
                            if end - start > 2 {
                                let pos = rng.random_range(start + 1..end - 1);
                                ta.set_cursor(pos);
                            }
                        }
                    }
                    _ => {
                        // 跳转到单词边界
                        // Jump to word boundaries
                        if rng.random_bool(0.5) {
                            let p = ta.beginning_of_previous_word();
                            ta.set_cursor(p);
                        } else {
                            let p = ta.end_of_next_word();
                            ta.set_cursor(p);
                        }
                    }
                }

                // 安全性规范
                // Sanity invariants
                assert!(ta.cursor() <= ta.text().len());

                // 元素不变量
                // Element invariants
                for payload in &elem_texts {
                    if let Some(start) = ta.text().find(payload) {
                        let end = start + payload.len();
                        // 元素内部的内容与初始设置的 Payload 一致
                        // 1) Text inside elements matches the initially set payload
                        assert_eq!(&ta.text()[start..end], payload);
                        // 光标始终不严格位于元素内部
                        // 2) Cursor is never strictly inside an element
                        let c = ta.cursor();
                        assert!(
                            c <= start || c >= end,
                            "cursor inside element: {start}..{end} at {c}"
                        );
                    }
                }

                // 渲染并计算光标位置；确保其位于有效范围内，避免崩溃
                // Render and compute cursor positions; ensure they are in-bounds and do not panic
                let area = Rect::new(0, 0, width, height);
                // 将状态转换为足够容纳所有嵌套行的区域
                // Stateless render into an area tall enough for all wrapped lines
                let total_lines = ta.desired_height(width);
                let full_area = Rect::new(0, 0, width, total_lines.max(1));
                let mut buf = Buffer::empty(full_area);
                ratatui::widgets::WidgetRef::render_ref(&(&ta), full_area, &mut buf);

                // 光标辅助功能必须保持插入点在视口内。
                // Both cursor helpers must keep the insertion point inside the viewport.
                for (x, y) in [ta.cursor_pos(area), ta.cursor_pos_with_state(area, state)]
                    .into_iter()
                    .flatten()
                {
                    assert!(area.contains((x, y).into()), "cursor escaped {area:?}: ({x}, {y})");
                }

                // 状态渲染不应导致崩溃，且滚动更新应平滑
                // Stateful render should not panic, and updates scroll
                let mut sbuf = Buffer::empty(area);
                ratatui::widgets::StatefulWidgetRef::render_ref(
                    &(&ta),
                    area,
                    &mut sbuf,
                    &mut state,
                );

                // 在渲染完成后，所需的行数等于不滚动时显示的行数。
                // After wrapping, desired height equals the number of lines we would render without scroll
                let total_lines = total_lines as usize;
                // 当内容在高度内时，滚动状态不能超过总行数
                // state.scroll must not exceed total_lines when content fits within area height
                if (height as usize) >= total_lines {
                    assert_eq!(state.scroll, 0);
                }
            }
        }
    }
}
