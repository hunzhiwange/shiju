//! Pure outbound Markdown formatting shared by Lark and Feishu.

use pulldown_cmark::{Event, Options, Parser, Tag};
use serde_json::json;

/// The message type and JSON-encoded content expected by the message API.
pub(super) struct FormattedLarkMessage {
    pub(super) msg_type: &'static str,
    pub(super) content: String,
}

/// Keep simple replies as posts; use a Markdown card for code blocks and tables.
pub(super) fn format_lark_message(markdown: &str) -> FormattedLarkMessage {
    let mut soft_breaks = Vec::new();
    for (event, range) in Parser::new_ext(markdown, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_) | Tag::Table(_)) => {
                return FormattedLarkMessage {
                    msg_type: "interactive",
                    content: json!({
                        "schema": "2.0",
                        "body": {"elements": [{"tag": "markdown", "content": markdown}]},
                    })
                    .to_string(),
                };
            }
            Event::SoftBreak => soft_breaks.push(range.start),
            _ => {}
        }
    }

    // Insert only at parser-recognized soft breaks, preserving all other source
    // bytes (including inline code, existing hard breaks and paragraph breaks).
    let mut text = String::with_capacity(markdown.len() + soft_breaks.len() * 2);
    let mut start = 0;
    for offset in soft_breaks {
        text.push_str(&markdown[start..offset]);
        text.push_str("  ");
        start = offset;
    }
    text.push_str(&markdown[start..]);

    FormattedLarkMessage {
        msg_type: "post",
        content: json!({"zh_cn": {"content": [[{"tag": "md", "text": text}]]}}).to_string(),
    }
}
