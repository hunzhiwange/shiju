//! Pure outbound Markdown formatting shared by Lark and Feishu.

use super::super::markdown_chunk::split_markdown;
use super::super::outbound_format::normalize_feishu_markdown;
use pulldown_cmark::{Event, Options, Parser, Tag};
use serde_json::json;

/// The message type and JSON-encoded content expected by the message API.
pub(super) struct FormattedLarkMessage {
    pub(super) msg_type: &'static str,
    pub(super) content: String,
}

/// Materialize post soft breaks before splitting so they count toward the byte budget.
fn normalize_lark_markdown(markdown: &str) -> String {
    let markdown = normalize_feishu_markdown(markdown);
    let soft_breaks = Parser::new_ext(&markdown, Options::ENABLE_TABLES)
        .into_offset_iter()
        .filter_map(|(event, range)| matches!(event, Event::SoftBreak).then_some(range.start))
        .collect::<Vec<_>>();
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

    text
}

/// Keep simple replies as posts; use a Markdown card for code blocks and tables.
pub(super) fn format_lark_message(markdown: &str) -> FormattedLarkMessage {
    let text = normalize_feishu_markdown(markdown);
    if Parser::new_ext(&text, Options::ENABLE_TABLES)
        .any(|event| matches!(event, Event::Start(Tag::CodeBlock(_) | Tag::Table(_))))
    {
        return FormattedLarkMessage {
            msg_type: "interactive",
            content: json!({
                "schema": "2.0",
                "body": {"elements": [{"tag": "markdown", "content": text}]},
            })
            .to_string(),
        };
    }
    let text = normalize_lark_markdown(&text);
    FormattedLarkMessage {
        msg_type: "post",
        content: json!({"zh_cn": {"content": [[{"tag": "md", "text": text}]]}}).to_string(),
    }
}

/// Preflight the actual double-encoded message bodies before acquiring a token or sending.
pub(super) fn prepare_lark_message_bodies(
    markdown: &str,
    recipient: &str,
) -> anyhow::Result<Vec<serde_json::Value>> {
    const BODY_BUDGET: usize = 3000;
    const JSON_BUDGET: usize = 16 * 1024;
    let normalized = normalize_lark_markdown(markdown);
    let chunks = split_markdown(&normalized, BODY_BUDGET)?;
    // Preserve existing short cards, including their surrounding prose bytes.
    if chunks.len() == 1 && chunks[0] == normalized {
        let formatted = format_lark_message(markdown);
        let short_body = json!({
            "receive_id": recipient,
            "msg_type": formatted.msg_type,
            "content": formatted.content,
        });
        if serde_json::to_vec(&short_body)?.len() <= JSON_BUDGET {
            return Ok(vec![short_body]);
        }
    }
    let mut pending = std::collections::VecDeque::from(chunks);
    let mut bodies = Vec::new();
    while let Some(chunk) = pending.pop_front() {
        let formatted = format_lark_message(&chunk);
        let body = json!({
            "receive_id": recipient,
            "msg_type": formatted.msg_type,
            "content": formatted.content,
        });
        if serde_json::to_vec(&body)?.len() <= JSON_BUDGET {
            bodies.push(body);
            continue;
        }
        // Retry at a smaller Markdown budget; the shared splitter preserves atomic
        // code lines/table rows or rejects them. Never truncate the encoded body.
        let smaller = split_markdown(&chunk, chunk.len() / 2)?;
        anyhow::ensure!(smaller.len() > 1, "Lark request exceeds local JSON byte budget");
        for part in smaller.into_iter().rev() {
            pending.push_front(part);
        }
    }
    Ok(bodies)
}
