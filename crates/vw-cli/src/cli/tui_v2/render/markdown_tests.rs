use unicode_width::UnicodeWidthStr;

use super::TuiTheme;
use super::markdown::{render_markdown, split_streaming_markdown};

fn plain_lines(markdown: &str, width: u16) -> (Vec<String>, Vec<String>) {
    let rendered = render_markdown(markdown, width, TuiTheme::default());
    let lines = rendered
        .lines
        .iter()
        .map(|line| line.spans.iter().map(|span| span.content.as_ref()).collect::<String>())
        .collect();
    let links = rendered.links.into_iter().map(|link| link.target).collect();
    (lines, links)
}

#[test]
fn tui_markdown_matches_reference_blocks_tables_and_streaming() {
    let markdown = concat!(
        "# 标题 Title\n\n",
        "段落含 `inline()`、**bold** 和 [文档](https://example.com/very/long/path?q=1)。\n\n",
        "- 第一项\n- second item\n\n",
        "> 引用 quote\n\n",
        "```rust\nfn main() {\n    println!(\"你好\");\n}\n```\n\n",
        "| 名称 | Status | Description |\n",
        "| --- | ---: | :--- |\n",
        "| 超长中文名称 | ready | deterministic truncation |\n",
    );
    let (lines, links) = plain_lines(markdown, 32);
    let joined = lines.join("\n");

    for marker in
        ["# 标题 Title", "inline()", "• 第一项", "│ 引用 quote", "fn main()", "┌", "Status"]
    {
        assert!(joined.contains(marker), "missing markdown marker {marker:?}:\n{joined}");
    }
    assert_eq!(links, vec!["https://example.com/very/long/path?q=1"]);
    assert!(lines.iter().all(|line| UnicodeWidthStr::width(line.as_str()) <= 32));
    assert!(!joined.contains('\u{1b}'), "styles must remain Ratatui spans, not embedded ANSI");

    let (narrow_lines, _) = plain_lines(markdown, 18);
    assert!(narrow_lines.iter().all(|line| UnicodeWidthStr::width(line.as_str()) <= 18));
    assert!(
        narrow_lines.iter().any(|line| line.contains('…')),
        "narrow tables must truncate deterministically"
    );

    let unfinished = "# Stable\n\nparagraph\n\n```rust\nlet 文本 = 1;";
    let first = split_streaming_markdown(unfinished, "");
    assert_eq!(first.stable_prefix, "# Stable\n\nparagraph\n\n");
    assert!(first.unstable_suffix.starts_with("```rust"));
    let extended = format!("{unfinished}\nprintln!(\"{{文本}}\");");
    let second = split_streaming_markdown(extended.as_str(), first.stable_prefix.as_str());
    assert_eq!(second.stable_prefix, first.stable_prefix);
    assert!(
        plain_lines(second.unstable_suffix.as_str(), 18).0.iter().any(|line| line.contains("let"))
    );

    let replaced = split_streaming_markdown("replacement", first.stable_prefix.as_str());
    assert!(replaced.stable_prefix.is_empty());
    assert_eq!(replaced.unstable_suffix, "replacement");
}
