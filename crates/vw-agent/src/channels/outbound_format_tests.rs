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
