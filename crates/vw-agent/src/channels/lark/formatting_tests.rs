use super::formatting::format_lark_message;
use serde_json::{Value, json};

fn payload(markdown: &str, msg_type: &str) -> Value {
    let formatted = format_lark_message(markdown);
    assert_eq!(formatted.msg_type, msg_type);
    serde_json::from_str(&formatted.content).unwrap()
}

#[test]
fn lark_formatter_materializes_soft_breaks_in_post() {
    let content = payload("第一行\n**第二行**\n\n- 项目\n  续行\n\n`inline`", "post");
    assert_eq!(
        content["zh_cn"]["content"][0][0],
        json!({"tag":"md", "text":"第一行  \n**第二行**\n\n- 项目  \n  续行\n\n`inline`"})
    );
}

#[test]
fn lark_formatter_uses_card_for_fenced_code() {
    for markdown in ["```rust\nlet ready = true;\n```", "~~~\n代码\n~~~", "    indented code\n"] {
        let content = payload(markdown, "interactive");
        assert_eq!(
            content,
            json!({"schema":"2.0", "body":{"elements":[{"tag":"markdown", "content":markdown}]}})
        );
    }
}

#[test]
fn lark_formatter_uses_card_for_gfm_table() {
    let markdown = "| 项目 | 状态 |\n| --- | --- |\n| 飞书 | ✅ |";
    let content = payload(markdown, "interactive");
    assert_eq!(content["schema"], "2.0");
    assert_eq!(content["body"]["elements"][0], json!({"tag":"markdown", "content":markdown}));
    payload("普通 | 文本\n不是表格", "post");
}

#[test]
fn lark_formatter_serializes_special_characters() {
    for markdown in ["引号 \" 和反斜杠 \\ 与制表符\t", "```\n\"\\\t\n```"] {
        let kind = if markdown.starts_with("```") { "interactive" } else { "post" };
        let content = payload(markdown, kind);
        let text = if kind == "post" {
            &content["zh_cn"]["content"][0][0]["text"]
        } else {
            &content["body"]["elements"][0]["content"]
        };
        assert_eq!(text, markdown);
    }
}
