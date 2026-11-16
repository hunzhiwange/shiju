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
use tui_text::width::display_width;
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
    payload: Option<String>,
    id: u64,
    range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextElementSnapshot {
    pub(crate) id: u64,
    pub(crate) range: Range<usize>,
    pub(crate) text: String,
}

/// Owned visible draft state, including atomic payloads and the UTF-8 cursor.
/// Editor configuration and the kill buffer remain local to the receiving editor.
#[derive(Debug, Clone, Eq)]
pub struct TextAreaDraft {
    text: String,
    cursor: usize,
    elements: Vec<TextElement>,
    next_element_id: u64,
}

impl PartialEq for TextAreaDraft {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.cursor == other.cursor
            && self.elements.len() == other.elements.len()
            && self
                .elements
                .iter()
                .zip(&other.elements)
                .all(|(left, right)| left.range == right.range && left.payload == right.payload)
    }
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
pub struct TextArea {
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
pub struct TextAreaState {
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

    pub fn draft(&self) -> TextAreaDraft {
        TextAreaDraft {
            text: self.text.clone(),
            cursor: self.cursor_pos,
            elements: self.elements.clone(),
            next_element_id: self.next_element_id,
        }
    }

    pub fn restore_draft(&mut self, draft: &TextAreaDraft) {
        self.set_text_clearing_elements(&draft.text);
        self.elements = draft.elements.clone();
        self.next_element_id = self.next_element_id.max(draft.next_element_id);
        self.set_cursor(draft.cursor);
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
                self.elements.push(TextElement { id, range: start..end, payload: None });
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

        let removed = self.expanded_range(range.clone());
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
        let removed = self.expanded_range(range);
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

    /// Insert a display-only atomic label whose original text is used on submission.
    pub fn insert_element_with_payload(&mut self, label: &str, payload: &str) -> u64 {
        let id = self.insert_element(label);
        if let Some(element) = self.elements.iter_mut().find(|element| element.id == id) {
            element.payload = Some(payload.to_owned());
        }
        id
    }

    /// Expand payloads in position order, preserving all surrounding whitespace.
    pub fn expanded_text(&self) -> String {
        self.expanded_range(0..self.text.len())
    }

    fn expanded_range(&self, range: Range<usize>) -> String {
        let mut result = String::new();
        let mut cursor = range.start;
        for element in &self.elements {
            if element.range.start < range.start || element.range.end > range.end {
                continue;
            }
            if let Some(payload) = &element.payload {
                result.push_str(&self.text[cursor..element.range.start]);
                result.push_str(payload);
                cursor = element.range.end;
            }
        }
        result.push_str(&self.text[cursor..range.end]);
        result
    }

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
    pub fn text_element_ranges(&self) -> impl Iterator<Item = &Range<usize>> {
        self.elements.iter().map(|element| &element.range)
    }

    /// 遍历重叠的原子元素范围，按顺序迭代。
    /// Iterates ordered atomic element ranges that overlap `range`.
    ///
    /// Elements ending exactly at the range start or starting exactly at its end are excluded.
    pub fn text_element_ranges_overlapping(
        &self,
        range: Range<usize>,
    ) -> impl Iterator<Item = &Range<usize>> {
        let first = self.elements.partition_point(|element| element.range.end <= range.start);
        self.elements[first..]
            .iter()
            .take_while(move |element| element.range.start < range.end)
            .map(|element| &element.range)
    }

    pub fn element_id_for_exact_range(&self, range: Range<usize>) -> Option<u64> {
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
        self.elements.push(TextElement { id, range, payload: None });
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
#[path = "textarea_tests.rs"]
mod tests;
