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
fn lark_formatter_inserts_newline_before_fenced_code() {
    let content = payload("说明```rust\nfn main() {}\n```", "interactive");
    assert_eq!(content["body"]["elements"][0]["content"], "说明\n```rust\nfn main() {}\n```");
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

#[test]
fn markdown_delivery_lark_preflight_accounts_for_soft_breaks_and_double_json() {
    use super::formatting::prepare_lark_message_bodies;
    for source in ["a\nb\n".repeat(1000), "\0".repeat(6000), "中😀\\\"".repeat(1000)] {
        let bodies = prepare_lark_message_bodies(&source, "oc_chat").unwrap();
        assert!(bodies.len() > 1);
        let mut rebuilt = String::new();
        for body in bodies {
            assert!(serde_json::to_vec(&body).unwrap().len() <= 16 * 1024);
            let content: Value = serde_json::from_str(body["content"].as_str().unwrap()).unwrap();
            let text = content["zh_cn"]["content"][0][0]["text"].as_str().unwrap();
            assert!(text.len() <= 3000);
            rebuilt.push_str(text);
        }
        let expected = payload(&source, "post");
        assert_eq!(rebuilt, expected["zh_cn"]["content"][0][0]["text"]);
    }
}

#[test]
fn markdown_delivery_lark_cards_preserve_table_rows_and_fenced_code() {
    use super::formatting::prepare_lark_message_bodies;
    use pulldown_cmark::{Event, Options, Parser, Tag};
    let table = format!(
        "| key | value |\n| --- | --- |\n{}",
        (0..80).map(|i| format!("| {i} | {} |\n", "中".repeat(20))).collect::<String>()
    );
    let bodies = prepare_lark_message_bodies(&table, "oc_chat").unwrap();
    assert!(bodies.len() > 1);
    let mut rows = Vec::new();
    for body in bodies {
        assert_eq!(body["msg_type"], "interactive");
        let content: Value = serde_json::from_str(body["content"].as_str().unwrap()).unwrap();
        let text = content["body"]["elements"][0]["content"].as_str().unwrap();
        assert!(text.len() <= 3000);
        assert_eq!(
            Parser::new_ext(text, Options::ENABLE_TABLES)
                .filter(|e| matches!(e, Event::Start(Tag::Table(_))))
                .count(),
            1
        );
        rows.extend(text.lines().skip(2).map(str::to_string));
    }
    assert_eq!(rows, table.lines().skip(2).collect::<Vec<_>>());
    let short = "第一行\n第二行\n\n```rust\nready()\n```";
    let body = prepare_lark_message_bodies(short, "oc_chat").unwrap().remove(0);
    let content: Value = serde_json::from_str(body["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["body"]["elements"][0]["content"], short);
    let body = prepare_lark_message_bodies("```rust\nready()", "oc_chat").unwrap().remove(0);
    let content: Value = serde_json::from_str(body["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["body"]["elements"][0]["content"], "```rust\nready()\n```");
}
