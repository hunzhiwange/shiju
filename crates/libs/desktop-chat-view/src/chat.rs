//! 聊天输入编辑与文件位置格式工具。

use iced::widget::text_editor;
use std::sync::Arc;

/// 公开的 append_line 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn append_line(input_editor: &mut text_editor::Content, s: &str) {
    let mut text = input_editor.text().to_string();
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(s);
    *input_editor = text_editor::Content::with_text(&text);

    input_editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
}

/// 公开的 insert_at_cursor 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn insert_at_cursor(input_editor: &mut text_editor::Content, s: &str) {
    input_editor.perform(text_editor::Action::Move(text_editor::Motion::DocumentEnd));
    input_editor
        .perform(text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(s.to_string()))));
}

/// 公开的 format_position 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn format_position(path: &str, line: usize, col: usize) -> String {
    format!("文件:{} 行:{} 列:{}", path, line, col)
}

/// 公开的 format_selection_positions 函数。
///
/// 参数由调用方提供，返回值表达该步骤的计算结果；遇到不可恢复的外部状态时通过现有返回类型向上层传播错误或空结果。
pub fn format_selection_positions(
    path: &str,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
) -> String {
    format!("@{}:{}:{}-{}:{}", path, start_line, start_col, end_line, end_col)
}

#[cfg(test)]
#[path = "chat_tests.rs"]
mod chat_tests;
