//! Markdown 标题与围栏标记的轻量识别规则，供分块器复用。

use once_cell::sync::Lazy;
use regex::Regex;

pub(super) static MARKDOWN_HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(#{1,6})[ \t]+(.+?)[ \t]*#*[ \t]*$")
        .expect("markdown heading regex should be valid")
});

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MarkdownHeading {
    pub level: usize,
    pub title: String,
}

pub(super) fn parse_markdown_heading(line: &str) -> Option<MarkdownHeading> {
    let captures = MARKDOWN_HEADING.captures(line.trim_end_matches('\r'))?;
    let hashes = captures.get(1)?.as_str();
    let title = captures.get(2)?.as_str().trim();
    if title.is_empty() {
        return None;
    }
    Some(MarkdownHeading { level: hashes.len(), title: title.to_string() })
}

pub(super) fn is_fence_marker(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;
