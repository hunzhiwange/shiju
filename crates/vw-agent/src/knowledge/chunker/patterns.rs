//! Markdown 标题与围栏标记的轻量识别规则，供分块器复用。
//!
//! 这里只识别分块所需的行级形状，不承担完整 Markdown 解析或渲染语义。

use once_cell::sync::Lazy;
use regex::Regex;

pub(super) static MARKDOWN_HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(#{1,6})[ \t]+(.+?)[ \t]*#*[ \t]*$")
        .expect("markdown heading regex should be valid")
});

#[derive(Debug, Clone, PartialEq, Eq)]
/// 解析后的 ATX 标题层级与文本。
pub(super) struct MarkdownHeading {
    /// 标题层级，范围为 1 到 6。
    pub level: usize,
    /// 去除可选闭合井号和首尾空白后的标题文本。
    pub title: String,
}

/// 解析一行 ATX 标题；空标题或非标题行返回 `None`。
pub(super) fn parse_markdown_heading(line: &str) -> Option<MarkdownHeading> {
    let captures = MARKDOWN_HEADING.captures(line.trim_end_matches('\r'))?;
    let hashes = captures.get(1)?.as_str();
    let title = captures.get(2)?.as_str().trim();
    if title.is_empty() {
        return None;
    }
    Some(MarkdownHeading { level: hashes.len(), title: title.to_string() })
}

/// 判断一行是否以 Markdown 三反引号围栏开始。
pub(super) fn is_fence_marker(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
