//! TUI 提示输入框的 Unicode 编辑、选区、撤销栈与视口计算。
//!
//! 光标和选区统一使用字素索引，只在修改 `String` 时转换为字节边界，
//! 避免将组合字符或宽字符拆开。模块仅修改 `PromptState`，不直接处理终端事件。

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::model::{PromptMotion, PromptState};

/// 用字素索引表示的选区，`anchor` 在扩展选择时保持不变。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EditorSelection {
    pub(crate) anchor: usize,
    pub(crate) active: usize,
}

impl EditorSelection {
    /// 按文本顺序返回选区的起止字素索引。
    pub(crate) fn ordered(self) -> (usize, usize) {
        (self.anchor.min(self.active), self.anchor.max(self.active))
    }
}

/// 撤销或重做所需的最小编辑器快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorSnapshot {
    value: String,
    cursor: usize,
    selection: Option<EditorSelection>,
}

/// 用于判断连续插入是否可合并为一次撤销的编辑类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorEditKind {
    Insert,
    Other,
}

/// 附着在 prompt 上的选区、撤销/重做、剪切缓冲和视口状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorState {
    pub(crate) selection: Option<EditorSelection>,
    undo: Vec<EditorSnapshot>,
    redo: Vec<EditorSnapshot>,
    pub(crate) kill_buffer: String,
    pub(crate) viewport_top_line: usize,
    pub(crate) viewport_height: usize,
    pub(crate) bracketed_paste: bool,
    last_edit: Option<EditorEditKind>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selection: None,
            undo: Vec::new(),
            redo: Vec::new(),
            kill_buffer: String::new(),
            viewport_top_line: 0,
            viewport_height: 1,
            bracketed_paste: false,
            last_edit: None,
        }
    }
}

impl EditorState {
    /// 返回当前可撤销的快照数。
    pub(crate) fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// 返回当前可重做的快照数。
    pub(crate) fn redo_depth(&self) -> usize {
        self.redo.len()
    }
}

/// 编辑器接受的语义命令，与具体终端按键解耦。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EditorCommand {
    Insert(String),
    Newline,
    Backspace,
    Delete,
    DeleteWordBackward,
    KillLine,
    Yank,
    Move { motion: PromptMotion, select: bool },
    MoveWord { direction: isize, select: bool },
    SelectAll,
    Undo,
    Redo,
    ViewportHeight(usize),
    PasteStart,
    PasteEnd,
}

/// 按当前视口高度裁剪后的可见行与光标显示位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorViewport {
    pub(crate) top_line: usize,
    pub(crate) cursor_row: usize,
    pub(crate) cursor_column: usize,
    pub(crate) visible_lines: Vec<String>,
}

/// 将一条语义命令应用到 prompt；返回值表示可见编辑状态是否发生变化。
pub(crate) fn apply_editor_command(prompt: &mut PromptState, command: EditorCommand) -> bool {
    match command {
        EditorCommand::Insert(text) => insert(prompt, normalize_newlines(text.as_str())),
        EditorCommand::Newline => insert(prompt, "\n".to_string()),
        EditorCommand::Backspace => backspace(prompt),
        EditorCommand::Delete => delete(prompt),
        EditorCommand::DeleteWordBackward => delete_word_backward(prompt),
        EditorCommand::KillLine => kill_line(prompt),
        EditorCommand::Yank => {
            let text = prompt.editor.kill_buffer.clone();
            if text.is_empty() { false } else { insert(prompt, text) }
        }
        EditorCommand::Move { motion, select } => move_cursor(prompt, motion, select),
        EditorCommand::MoveWord { direction, select } => move_word(prompt, direction, select),
        EditorCommand::SelectAll => {
            let end = grapheme_count(prompt.value.as_str());
            prompt.editor.selection = Some(EditorSelection { anchor: 0, active: end });
            prompt.cursor.char_index = end;
            prompt.editor.last_edit = None;
            true
        }
        EditorCommand::Undo => undo(prompt),
        EditorCommand::Redo => redo(prompt),
        EditorCommand::ViewportHeight(height) => {
            prompt.editor.viewport_height = height.max(1);
            ensure_cursor_visible(prompt);
            true
        }
        EditorCommand::PasteStart => {
            prompt.editor.bracketed_paste = true;
            prompt.editor.last_edit = None;
            true
        }
        EditorCommand::PasteEnd => {
            prompt.editor.bracketed_paste = false;
            prompt.editor.last_edit = None;
            true
        }
    }
}

/// 根据光标与视口状态返回当前可见的文本行。
pub(crate) fn editor_viewport(prompt: &PromptState) -> EditorViewport {
    let (line, column) = cursor_display_position(prompt.value.as_str(), prompt.cursor.char_index);
    let lines = prompt.value.split('\n').map(ToOwned::to_owned).collect::<Vec<_>>();
    let top = prompt.editor.viewport_top_line.min(lines.len().saturating_sub(1));
    let end = top.saturating_add(prompt.editor.viewport_height.max(1)).min(lines.len());
    EditorViewport {
        top_line: top,
        cursor_row: line.saturating_sub(top),
        cursor_column: column,
        visible_lines: lines[top..end].to_vec(),
    }
}

/// 计算 Unicode 字素数，用于与 prompt 光标索引保持同一坐标系。
pub(crate) fn grapheme_count(value: &str) -> usize {
    UnicodeSegmentation::graphemes(value, true).count()
}

/// 将字素索引转换为合法 UTF-8 字节边界；末尾索引映射到字符串长度。
pub(crate) fn byte_index_for_grapheme(value: &str, grapheme_index: usize) -> Option<usize> {
    if grapheme_index == grapheme_count(value) {
        return Some(value.len());
    }
    UnicodeSegmentation::grapheme_indices(value, true).nth(grapheme_index).map(|(index, _)| index)
}

/// 返回当前选区的完整 UTF-8 文本；空选区或非法边界返回空。
pub(crate) fn selected_text(prompt: &PromptState) -> Option<String> {
    let selection = prompt.editor.selection?;
    let (start, end) = selection.ordered();
    if start >= end {
        return None;
    }
    let start = byte_index_for_grapheme(prompt.value.as_str(), start)?;
    let end = byte_index_for_grapheme(prompt.value.as_str(), end)?;
    prompt.value.get(start..end).map(ToOwned::to_owned)
}

fn insert(prompt: &mut PromptState, text: String) -> bool {
    if text.is_empty() {
        return false;
    }
    let merge = prompt.editor.last_edit == Some(EditorEditKind::Insert)
        && prompt.editor.selection.is_none()
        && !prompt.editor.bracketed_paste;
    // 连续普通输入合并为一次撤销；粘贴和替换选区必须形成独立边界。
    begin_edit(prompt, EditorEditKind::Insert, merge);
    delete_selection_without_snapshot(prompt);
    let Some(byte_index) = byte_index_for_grapheme(&prompt.value, prompt.cursor.char_index) else {
        return false;
    };
    prompt.value.insert_str(byte_index, text.as_str());
    prompt.cursor.char_index =
        prompt.cursor.char_index.saturating_add(grapheme_count(text.as_str()));
    prompt.cursor.preferred_column = None;
    prompt.editor.selection = None;
    ensure_cursor_visible(prompt);
    true
}

fn backspace(prompt: &mut PromptState) -> bool {
    if prompt.editor.selection.is_some() {
        begin_edit(prompt, EditorEditKind::Other, false);
        return delete_selection_without_snapshot(prompt);
    }
    if prompt.cursor.char_index == 0 {
        return false;
    }
    begin_edit(prompt, EditorEditKind::Other, false);
    let end = byte_index_for_grapheme(&prompt.value, prompt.cursor.char_index);
    let start = byte_index_for_grapheme(&prompt.value, prompt.cursor.char_index.saturating_sub(1));
    let (Some(start), Some(end)) = (start, end) else {
        return false;
    };
    prompt.value.replace_range(start..end, "");
    prompt.cursor.char_index = prompt.cursor.char_index.saturating_sub(1);
    ensure_cursor_visible(prompt);
    true
}

fn delete(prompt: &mut PromptState) -> bool {
    if prompt.editor.selection.is_some() {
        begin_edit(prompt, EditorEditKind::Other, false);
        return delete_selection_without_snapshot(prompt);
    }
    if prompt.cursor.char_index >= grapheme_count(&prompt.value) {
        return false;
    }
    begin_edit(prompt, EditorEditKind::Other, false);
    let start = byte_index_for_grapheme(&prompt.value, prompt.cursor.char_index);
    let end = byte_index_for_grapheme(&prompt.value, prompt.cursor.char_index.saturating_add(1));
    let (Some(start), Some(end)) = (start, end) else {
        return false;
    };
    prompt.value.replace_range(start..end, "");
    ensure_cursor_visible(prompt);
    true
}

fn delete_word_backward(prompt: &mut PromptState) -> bool {
    if prompt.cursor.char_index == 0 && prompt.editor.selection.is_none() {
        return false;
    }
    begin_edit(prompt, EditorEditKind::Other, false);
    if delete_selection_without_snapshot(prompt) {
        return true;
    }
    let target = word_boundary(prompt.value.as_str(), prompt.cursor.char_index, -1);
    delete_grapheme_range(prompt, target, prompt.cursor.char_index)
}

fn kill_line(prompt: &mut PromptState) -> bool {
    let end = line_end_grapheme(prompt.value.as_str(), prompt.cursor.char_index);
    if end == prompt.cursor.char_index {
        return false;
    }
    begin_edit(prompt, EditorEditKind::Other, false);
    let start_byte = byte_index_for_grapheme(&prompt.value, prompt.cursor.char_index);
    let end_byte = byte_index_for_grapheme(&prompt.value, end);
    let (Some(start_byte), Some(end_byte)) = (start_byte, end_byte) else {
        return false;
    };
    prompt.editor.kill_buffer = prompt.value[start_byte..end_byte].to_string();
    prompt.value.replace_range(start_byte..end_byte, "");
    ensure_cursor_visible(prompt);
    true
}

fn move_cursor(prompt: &mut PromptState, motion: PromptMotion, select: bool) -> bool {
    let old = prompt.cursor.char_index;
    let (_, current_column) = cursor_display_position(prompt.value.as_str(), old);
    let preferred_column = prompt.cursor.preferred_column.unwrap_or(current_column);
    let next = match motion {
        PromptMotion::Left => old.saturating_sub(1),
        PromptMotion::Right => old.saturating_add(1).min(grapheme_count(&prompt.value)),
        PromptMotion::Home => line_start_grapheme(prompt.value.as_str(), old),
        PromptMotion::End => line_end_grapheme(prompt.value.as_str(), old),
        PromptMotion::Up | PromptMotion::Down => vertical_target(prompt, motion, preferred_column),
    };
    if next == old {
        if !select && prompt.editor.selection.take().is_some() {
            prompt.cursor.preferred_column = None;
            prompt.editor.last_edit = None;
            ensure_cursor_visible(prompt);
            return true;
        }
        return false;
    }
    update_selection(prompt, old, next, select);
    prompt.cursor.char_index = next;
    prompt.cursor.preferred_column = match motion {
        PromptMotion::Up | PromptMotion::Down => Some(preferred_column),
        _ => None,
    };
    prompt.editor.last_edit = None;
    ensure_cursor_visible(prompt);
    true
}

fn move_word(prompt: &mut PromptState, direction: isize, select: bool) -> bool {
    let old = prompt.cursor.char_index;
    let next = word_boundary(prompt.value.as_str(), old, direction);
    if next == old {
        return false;
    }
    update_selection(prompt, old, next, select);
    prompt.cursor.char_index = next;
    prompt.editor.last_edit = None;
    ensure_cursor_visible(prompt);
    true
}

fn undo(prompt: &mut PromptState) -> bool {
    let Some(previous) = prompt.editor.undo.pop() else {
        return false;
    };
    prompt.editor.redo.push(snapshot(prompt));
    restore_snapshot(prompt, previous);
    prompt.editor.last_edit = None;
    true
}

fn redo(prompt: &mut PromptState) -> bool {
    let Some(next) = prompt.editor.redo.pop() else {
        return false;
    };
    prompt.editor.undo.push(snapshot(prompt));
    restore_snapshot(prompt, next);
    prompt.editor.last_edit = None;
    true
}

fn begin_edit(prompt: &mut PromptState, kind: EditorEditKind, merge: bool) {
    if !merge {
        let current = snapshot(prompt);
        prompt.editor.undo.push(current);
    }
    prompt.editor.redo.clear();
    prompt.editor.last_edit = Some(kind);
}

fn snapshot(prompt: &PromptState) -> EditorSnapshot {
    EditorSnapshot {
        value: prompt.value.clone(),
        cursor: prompt.cursor.char_index,
        selection: prompt.editor.selection,
    }
}

fn restore_snapshot(prompt: &mut PromptState, snapshot: EditorSnapshot) {
    prompt.value = snapshot.value;
    prompt.cursor.char_index = snapshot.cursor;
    prompt.cursor.preferred_column = None;
    prompt.editor.selection = snapshot.selection;
    ensure_cursor_visible(prompt);
}

fn delete_selection_without_snapshot(prompt: &mut PromptState) -> bool {
    let Some(selection) = prompt.editor.selection.take() else {
        return false;
    };
    let (start, end) = selection.ordered();
    delete_grapheme_range(prompt, start, end)
}

fn delete_grapheme_range(prompt: &mut PromptState, start: usize, end: usize) -> bool {
    let start_byte = byte_index_for_grapheme(&prompt.value, start);
    let end_byte = byte_index_for_grapheme(&prompt.value, end);
    let (Some(start_byte), Some(end_byte)) = (start_byte, end_byte) else {
        return false;
    };
    if start_byte == end_byte {
        return false;
    }
    prompt.value.replace_range(start_byte..end_byte, "");
    prompt.cursor.char_index = start;
    prompt.editor.selection = None;
    ensure_cursor_visible(prompt);
    true
}

fn update_selection(prompt: &mut PromptState, old: usize, next: usize, select: bool) {
    if select {
        let anchor = prompt.editor.selection.map_or(old, |selection| selection.anchor);
        prompt.editor.selection =
            (anchor != next).then_some(EditorSelection { anchor, active: next });
    } else {
        prompt.editor.selection = None;
    }
}

fn word_boundary(value: &str, cursor: usize, direction: isize) -> usize {
    let graphemes = UnicodeSegmentation::graphemes(value, true).collect::<Vec<_>>();
    if direction < 0 {
        let mut index = cursor.min(graphemes.len());
        while index > 0 && is_space(graphemes[index - 1]) {
            index -= 1;
        }
        while index > 0 && !is_space(graphemes[index - 1]) {
            index -= 1;
        }
        index
    } else {
        let mut index = cursor.min(graphemes.len());
        while index < graphemes.len() && !is_space(graphemes[index]) {
            index += 1;
        }
        while index < graphemes.len() && is_space(graphemes[index]) {
            index += 1;
        }
        index
    }
}

fn is_space(grapheme: &str) -> bool {
    grapheme.chars().all(char::is_whitespace)
}

fn line_start_grapheme(value: &str, cursor: usize) -> usize {
    let graphemes = UnicodeSegmentation::graphemes(value, true).collect::<Vec<_>>();
    graphemes[..cursor.min(graphemes.len())]
        .iter()
        .rposition(|grapheme| *grapheme == "\n")
        .map_or(0, |index| index + 1)
}

fn line_end_grapheme(value: &str, cursor: usize) -> usize {
    let graphemes = UnicodeSegmentation::graphemes(value, true).collect::<Vec<_>>();
    graphemes[cursor.min(graphemes.len())..]
        .iter()
        .position(|grapheme| *grapheme == "\n")
        .map_or(graphemes.len(), |offset| cursor + offset)
}

fn vertical_target(prompt: &PromptState, motion: PromptMotion, preferred_column: usize) -> usize {
    let (line, _) = cursor_display_position(prompt.value.as_str(), prompt.cursor.char_index);
    let target = match motion {
        PromptMotion::Up => line.saturating_sub(1),
        PromptMotion::Down => line.saturating_add(1),
        _ => line,
    };
    grapheme_for_line_column(prompt.value.as_str(), target, preferred_column)
}

fn cursor_display_position(value: &str, cursor: usize) -> (usize, usize) {
    let graphemes = UnicodeSegmentation::graphemes(value, true).collect::<Vec<_>>();
    let prefix = &graphemes[..cursor.min(graphemes.len())];
    let line = prefix.iter().filter(|grapheme| **grapheme == "\n").count();
    let start = prefix.iter().rposition(|grapheme| *grapheme == "\n").map_or(0, |index| index + 1);
    let column = prefix[start..].iter().map(|grapheme| grapheme.width()).sum();
    (line, column)
}

fn grapheme_for_line_column(value: &str, target_line: usize, target_column: usize) -> usize {
    let graphemes = UnicodeSegmentation::graphemes(value, true).collect::<Vec<_>>();
    let mut line = 0usize;
    let mut column = 0usize;
    for (index, grapheme) in graphemes.iter().enumerate() {
        if line == target_line {
            if *grapheme == "\n" || column.saturating_add(grapheme.width()) > target_column {
                return index;
            }
            column = column.saturating_add(grapheme.width());
        } else if *grapheme == "\n" {
            line += 1;
            column = 0;
        }
    }
    graphemes.len()
}

fn ensure_cursor_visible(prompt: &mut PromptState) {
    let (line, _) = cursor_display_position(prompt.value.as_str(), prompt.cursor.char_index);
    let height = prompt.editor.viewport_height.max(1);
    if line < prompt.editor.viewport_top_line {
        prompt.editor.viewport_top_line = line;
    } else if line >= prompt.editor.viewport_top_line.saturating_add(height) {
        prompt.editor.viewport_top_line = line.saturating_add(1).saturating_sub(height);
    }
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
