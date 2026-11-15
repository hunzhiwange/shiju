//! 桌面语音与 Markdown 的组合适配。
pub(super) use desktop_chat_view::message_view::parse::{RenderBlock, borrowed_blocks, owned_blocks_from_raw};
pub(crate) use desktop_chat_view::message_view::parse::hash_chat_content;
use crate::app::models::ParsedChatBlock;

/// Speech consumes the same finalized prose blocks as the chat renderer.
pub(crate) fn assistant_speech_text(raw: &str) -> String {
    use super::markdown::{MarkdownBlockKind, parse_markdown_blocks};
    let mut prose = Vec::new();
    for block in owned_blocks_from_raw(raw) {
        let ParsedChatBlock::Text { content } = block else { continue };
        for block in parse_markdown_blocks(&content) {
            if matches!(
                block.kind,
                MarkdownBlockKind::CodeBlock(_)
                    | MarkdownBlockKind::Table(_)
                    | MarkdownBlockKind::Rule
            ) {
                continue;
            }
            let text = if block.inline.is_empty() {
                block.text
            } else {
                block
                    .inline
                    .into_iter()
                    .filter(|span| !span.style.code)
                    .map(|span| span.text)
                    .collect::<String>()
            };
            if !text.trim().is_empty() {
                prose.push(text.trim().to_owned());
            }
        }
    }
    prose.join("\n")
}
