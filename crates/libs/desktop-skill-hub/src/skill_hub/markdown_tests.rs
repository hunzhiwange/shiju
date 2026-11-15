use super::markdown::{parse_public_markdown, markdown_without_code_highlighting};

#[test]
fn public_markdown_disables_fenced_code_highlighting_without_losing_code() {
    let markdown = concat!(
        "```rust\n",
        "// 中文注释\n",
        "```\n",
        "  ~~~~bash extra\n",
        "# 中文脚本\n",
        "  ~~~~\n",
    );
    let normalized =
        concat!("```\n", "// 中文注释\n", "```\n", "  ~~~~\n", "# 中文脚本\n", "  ~~~~\n",);

    let items = parse_public_markdown(markdown);
    let code_blocks = items
        .iter()
        .filter_map(|item| match item {
            iced::widget::markdown::Item::CodeBlock { language, code, .. } => {
                Some((language, code))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(markdown_without_code_highlighting(markdown), normalized);
    assert_eq!(markdown_without_code_highlighting("    ```text\n"), "    ```text\n");
    assert_eq!(code_blocks.len(), 2);
    assert!(code_blocks.iter().all(|(language, _)| language.is_none()));
    assert_eq!(code_blocks[0].1, "// 中文注释\n");
    assert_eq!(code_blocks[1].1, "# 中文脚本\n");
}
