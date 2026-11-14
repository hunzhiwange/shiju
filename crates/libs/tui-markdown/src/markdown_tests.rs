use super::*;
use pretty_assertions::assert_eq;
use ratatui::text::Line;

fn lines_to_strings(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.spans.iter().map(|s| s.content.clone()).collect::<String>())
        .collect()
}

#[test]
// 引用标记在当前无上下文入口中按普通文本展示。
fn citations_render_as_plain_text() {
    let src = "Before 【F:/x.rs†L1】\nAfter 【F:/x.rs†L3】\n";
    let mut out = Vec::new();
    append_markdown(src, /*width*/ None, /*cwd*/ None, &mut out);
    let rendered = lines_to_strings(&out);
    assert_eq!(
        rendered,
        vec!["Before 【F:/x.rs†L1】".to_string(), "After 【F:/x.rs†L3】".to_string()]
    );
}

#[test]
// 缩进代码块必须保留前导空白，避免复制时改变代码含义。
fn indented_code_blocks_preserve_leading_whitespace() {
    // 基础校验：被空行包围的缩进代码应产生带原缩进的行。
    // Basic sanity: indented code with surrounding blank lines should produce the indented line.
    let src = "Before\n\n    code 1\n\nAfter\n";
    let mut out = Vec::new();
    append_markdown(src, /*width*/ None, /*cwd*/ None, &mut out);
    let lines = lines_to_strings(&out);
    assert_eq!(lines, vec!["Before", "", "    code 1", "", "After"]);
}

#[test]
// 通用追加入口不得丢失单行正文中的任意文本片段。
fn append_markdown_preserves_full_text_line() {
    let src = "Hi! How can I help with codex-rs today? Want me to explore the repo, run tests, or work on a specific change?\n";
    let mut out = Vec::new();
    append_markdown(src, /*width*/ None, /*cwd*/ None, &mut out);
    assert_eq!(out.len(), 1, "expected a single rendered line for plain text");
    let rendered: String = out
        .iter()
        .flat_map(|l| l.spans.iter())
        .map(|s| s.content.clone())
        .collect::<Vec<_>>()
        .join("");
    assert_eq!(
        rendered,
        "Hi! How can I help with codex-rs today? Want me to explore the repo, run tests, or work on a specific change?"
    );
}

#[test]
// 有序列表项的包装入口应与底层 TUI Markdown 渲染一致。
fn append_markdown_matches_tui_markdown_for_ordered_item() {
    let mut out = Vec::new();
    append_markdown("1. Tight item\n", /*width*/ None, /*cwd*/ None, &mut out);
    let lines = lines_to_strings(&out);
    assert_eq!(lines, vec!["1. Tight item".to_string()]);
}

#[test]
// 上下文中的有序列表标记和正文必须保持在同一展示行。
fn append_markdown_keeps_ordered_list_line_unsplit_in_context() {
    let src = "Loose vs. tight list items:\n1. Tight item\n";
    let mut out = Vec::new();
    append_markdown(src, /*width*/ None, /*cwd*/ None, &mut out);

    let lines = lines_to_strings(&out);

    // 预期有序列表标记与文本渲染为单行，而非标记单独占行。
    // Expect to find the ordered list line rendered as a single line,
    // not split into a marker-only line followed by the text.
    assert!(
        lines.iter().any(|s| s == "1. Tight item"),
        "expected '1. Tight item' rendered as a single line; got: {lines:?}"
    );
    assert!(
        !lines.windows(2).any(|w| w[0].trim_end() == "1." && w[1] == "Tight item"),
        "did not expect a split into ['1.', 'Tight item']; got: {lines:?}"
    );
}

#[test]
// 助手入口应解包 Markdown 围栏，使标准管道表格按表格渲染。
fn append_markdown_agent_unwraps_markdown_fences_for_table_rendering() {
    let src = "```markdown\n| A | B |\n|---|---|\n| 1 | 2 |\n```\n";
    let mut out = Vec::new();
    append_markdown_agent(src, /*width*/ None, &mut out);
    let rendered = lines_to_strings(&out);
    assert!(rendered.iter().any(|line| line.contains('━')));
    assert!(rendered.iter().any(|line| line.contains(" 1      2")));
}

#[test]
// 无外侧管道的表格在 Markdown 围栏中也应被识别并解包。
fn append_markdown_agent_unwraps_markdown_fences_for_no_outer_table_rendering() {
    let src = "```md\nCol A | Col B | Col C\n--- | --- | ---\nx | y | z\n10 | 20 | 30\n```\n";
    let mut out = Vec::new();
    append_markdown_agent(src, /*width*/ None, &mut out);
    let rendered = lines_to_strings(&out);
    assert!(rendered.iter().any(|line| line.contains('━')));
    assert!(rendered.iter().any(|line| line.contains(" Col A    Col B    Col C")));
    assert!(!rendered.iter().any(|line| line.trim() == "Col A | Col B | Col C"));
}

#[test]
// 两列无外侧管道表格应走与标准表格相同的原生渲染路径。
fn append_markdown_agent_unwraps_markdown_fences_for_two_column_no_outer_table() {
    let src = "```md\nA | B\n--- | ---\nleft | right\n```\n";
    let mut out = Vec::new();
    append_markdown_agent(src, /*width*/ None, &mut out);
    let rendered = lines_to_strings(&out);
    assert!(rendered.iter().any(|line| line.contains('━')));
    assert!(rendered.iter().any(|line| line.contains(" left    right")));
    assert!(!rendered.iter().any(|line| line.trim() == "A | B"));
}

#[test]
// 单列表格具备表头与分隔行时仍应解包。
fn append_markdown_agent_unwraps_markdown_fences_for_single_column_table() {
    let src = "```md\n| Only |\n|---|\n| value |\n```\n";
    let mut out = Vec::new();
    append_markdown_agent(src, /*width*/ None, &mut out);
    let rendered = lines_to_strings(&out);
    assert!(rendered.iter().any(|line| line.contains('━')));
    assert!(!rendered.iter().any(|line| line.trim() == "| Only |"));
}

#[test]
// 非 Markdown 语言围栏必须保持代码块语义。
fn append_markdown_agent_keeps_non_markdown_fences_as_code() {
    let src = "```rust\n| A | B |\n|---|---|\n| 1 | 2 |\n```\n";
    let mut out = Vec::new();
    append_markdown_agent(src, /*width*/ None, &mut out);
    let rendered = lines_to_strings(&out);
    assert_eq!(
        rendered,
        vec!["| A | B |".to_string(), "|---|---|".to_string(), "| 1 | 2 |".to_string(),]
    );
}

#[test]
// 块引用中的 Markdown 表格围栏应在保留引用结构的同时解包。
fn append_markdown_agent_unwraps_blockquoted_markdown_fence_table() {
    let src = "> ```markdown\n> | A | B |\n> |---|---|\n> | 1 | 2 |\n> ```\n";
    let rendered = unwrap_markdown_fences(src);
    assert!(
        !rendered.contains("```"),
        "expected markdown fence markers to be removed: {rendered:?}"
    );
}

#[test]
// 普通围栏内仅出现块引用表格示例时，不应误判并解包外围围栏。
fn append_markdown_agent_keeps_non_blockquoted_markdown_fence_with_blockquote_table_example() {
    let src = "```markdown\n> | A | B |\n> |---|---|\n> | 1 | 2 |\n```\n";
    let normalized = unwrap_markdown_fences(src);
    assert_eq!(normalized, src);
}

#[test]
// Markdown 围栏正文不是表格时继续渲染为代码块。
fn append_markdown_agent_keeps_markdown_fence_when_content_is_not_table() {
    let src = "```markdown\n**bold**\n```\n";
    let mut out = Vec::new();
    append_markdown_agent(src, /*width*/ None, &mut out);
    let rendered = lines_to_strings(&out);
    assert_eq!(rendered, vec!["**bold**".to_string()]);
}

#[test]
// 缺少相邻表头与分隔行对的复现场景必须保留围栏。
fn unwrap_markdown_fences_repro_keeps_fence_without_header_delimiter_pair() {
    let src = "```markdown\n| A | B |\nnot a delimiter row\n| --- | --- |\n# Heading\n```\n";
    let normalized = unwrap_markdown_fences(src);
    assert_eq!(normalized, src);
}

#[test]
// 表头与分隔行之间出现空行时不构成可解包表格。
fn append_markdown_agent_keeps_markdown_fence_with_blank_line_between_header_and_delimiter() {
    let src = "```markdown\n| A | B |\n\n|---|---|\n| 1 | 2 |\n```\n";
    let rendered = unwrap_markdown_fences(src);
    assert_eq!(rendered, src);
}
