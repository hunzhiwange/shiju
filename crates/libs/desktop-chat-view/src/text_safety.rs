//! 只读编辑器的输入边界；保持原有字节数和行字符数限制。
use iced::widget::text_editor;
pub fn is_safe_for_text_editor(content: &str) -> bool {
    if content.len() > 20_000 { return false; }
    content.lines().map(|line| line.chars().count()).max().unwrap_or(0) <= 2_000
}
pub fn read_only_editor_content(text: &str) -> Option<text_editor::Content> {
    if text.trim().is_empty() || !is_safe_for_text_editor(text) { return None; }
    Some(text_editor::Content::with_text(text))
}
