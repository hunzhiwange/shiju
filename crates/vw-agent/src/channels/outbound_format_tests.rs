use super::outbound_format::{
    format_wecom_markdown_tables, normalize_chat_markdown, normalize_feishu_markdown,
};

#[test]
fn markdown_normalizer_separates_numbered_lists_and_dedents_fences() {
    assert_eq!(
        normalize_chat_markdown("步骤：\n1. 第一项\n2. 第二项\n  ```rust\n  fn main() {}\n  ```"),
        "步骤：\n\n1. 第一项\n2. 第二项\n```rust\nfn main() {}\n```"
    );
}

#[test]
fn feishu_normalizer_inserts_newline_before_fence() {
    assert_eq!(
        normalize_feishu_markdown("说明```rust\nfn main() {}\n```"),
        "说明\n```rust\nfn main() {}\n```"
    );
}

#[test]
fn wecom_formatter_aligns_tables_but_preserves_fenced_code() {
    assert_eq!(
        format_wecom_markdown_tables(
            "| 名称 | 状态 |\n| --- | --- |\n| A | 已完成 |\n\n```\na | b\n```"
        ),
        "| 名称 | 状态  |\n| --- | --- |\n| A  | 已完成 |\n\n```\na | b\n```"
    );
}

#[test]
fn markdown_delivery_normalization_preserves_fenced_and_escaped_content() {
    for fence in ["````rust", "~~~rust"] {
        let closing = if fence.starts_with('`') { "````" } else { "~~~" };
        let code = format!(
            "{fence}\nlet s = \"```\";\n| a | b |\n| --- | --- |\n| x | y |\n1. keep\n{closing}"
        );
        assert_eq!(normalize_feishu_markdown(&code), code);
        assert_eq!(format_wecom_markdown_tables(&code), code);
        assert_eq!(normalize_chat_markdown(&code), code);
    }
    let input = "| a | b |\n| :--- | ---: |\n| x\\|y | `m|n` |";
    let output = format_wecom_markdown_tables(input);
    assert!(output.contains("x\\|y"));
    assert!(output.contains("`m|n`"));
    assert!(output.lines().nth(1).unwrap().contains(':'));
    assert_eq!(normalize_feishu_markdown("inline `a```b` stays"), "inline `a```b` stays");
}
