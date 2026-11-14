#[cfg(test)]
use ratatui::text::Line;

/// Consider a line blank if it has no spans or only spans whose contents are
/// empty or consist solely of spaces (no tabs/newlines).
/// 没有 span，或所有 span 仅含普通空格时视为空白行；制表符与换行不在此定义内。
#[cfg(test)]
pub fn is_blank_line_spaces_only(line: &Line<'_>) -> bool {
    if line.spans.is_empty() {
        return true;
    }
    line.spans.iter().all(|s| s.content.is_empty() || s.content.chars().all(|c| c == ' '))
}
