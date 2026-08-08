use super::parse::{RenderBlock, borrowed_blocks, hash_chat_content, owned_blocks_from_raw};
use crate::app::models::ParsedChatBlock;

#[test]
fn owned_blocks_from_raw_splits_think_tool_and_text() {
    let raw = "<think>plan</think>\nanswer\ntool bash\n{\"status\":\"completed\"}\n";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Think { content, open }) if content == "plan" && !open)
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Text { content }) if content.trim() == "answer")
    );
    assert!(
        matches!(blocks.get(2), Some(ParsedChatBlock::Tool { raw }) if raw.starts_with("tool bash"))
    );
}

#[test]
fn reasoning_marker_renders_as_closed_think_block() {
    let blocks = owned_blocks_from_raw("answer\n\n[reasoning]\nchain");

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content == "answer\n\n")
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Think { content, open }) if content == "chain" && !open)
    );
}

#[test]
fn inline_reasoning_text_stays_visible() {
    let raw = "answer [reasoning] is a label";
    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.as_slice(), [ParsedChatBlock::Text { content }] if content == raw)
    );
}

#[test]
fn borrowed_blocks_preserve_block_kinds() {
    let blocks = vec![
        ParsedChatBlock::Text { content: "hello".to_string() },
        ParsedChatBlock::Think { content: "why".to_string(), open: true },
    ];

    let borrowed = borrowed_blocks(&blocks).collect::<Vec<_>>();

    assert!(matches!(borrowed[0], RenderBlock::Text { content } if content == "hello"));
    assert!(
        matches!(borrowed[1], RenderBlock::Think { content, open } if content == "why" && open)
    );
}

#[test]
fn borrowed_blocks_preserve_tool_raw_content() {
    let blocks = vec![ParsedChatBlock::Tool { raw: "tool bash\n{}\n".to_string() }];

    let borrowed = borrowed_blocks(&blocks).collect::<Vec<_>>();

    assert!(
        matches!(borrowed.as_slice(), [RenderBlock::Tool { raw }] if *raw == "tool bash\n{}\n")
    );
}

#[test]
fn hash_chat_content_changes_with_content() {
    assert_eq!(hash_chat_content("same"), hash_chat_content("same"));
    assert_ne!(hash_chat_content("same"), hash_chat_content("other"));
}

#[test]
fn think_tags_allow_attributes_and_ignore_prefix_collisions() {
    let raw = "pre <thinker>not a tag</thinker><think time=\"1s\">inside</think> tail";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content.contains("<thinker>"))
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Think { content, open }) if content == "inside" && !open)
    );
    assert!(matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == " tail"));
}

#[test]
fn thinking_alias_tags_are_parsed_as_think_blocks() {
    let raw = "pre <thinking>plan</thinking> tail";

    let blocks = owned_blocks_from_raw(raw);

    assert!(matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content == "pre "));
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Think { content, open }) if content == "plan" && !open)
    );
    assert!(matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == " tail"));
}

#[test]
fn orphan_think_close_tags_are_removed_from_visible_text() {
    let raw = "answer</think>\nnext</think reason=\"done\">tail</thinker>";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.as_slice(), [ParsedChatBlock::Text { content }] if content == "answer\nnexttail</thinker>")
    );
}

#[test]
fn orphan_think_close_tag_does_not_create_an_empty_text_block() {
    let blocks = owned_blocks_from_raw("</think><think>hidden</think>answer</think>");

    assert!(matches!(blocks.as_slice(), [
            ParsedChatBlock::Think { content, open },
            ParsedChatBlock::Text { content: answer },
        ] if content == "hidden" && !open && answer == "answer"));
}

#[test]
fn unclosed_think_splits_following_tool_blocks() {
    let raw = "<think>plan\ntool bash\n{\"status\":\"completed\",\"output\":\"ok\"}\nanswer";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Think { content, open }) if content == "plan\n" && *open)
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Tool { raw }) if raw.starts_with("tool bash"))
    );
    assert!(
        matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == "answer")
    );
}

#[test]
fn invalid_tool_like_text_stays_text() {
    let raw = "tool bash\nnot json\nstill not json";

    let blocks = owned_blocks_from_raw(raw);

    assert_eq!(blocks.len(), 1);
    assert!(matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content == raw));
}

#[test]
fn tool_header_without_a_json_line_stays_text() {
    let raw = "tool bash";

    let blocks = owned_blocks_from_raw(raw);

    assert!(matches!(blocks.as_slice(), [ParsedChatBlock::Text { content }] if content == raw));
}

#[test]
fn tool_after_colon_is_parsed_as_tool_block() {
    let raw = "说明：tool file_write\n{\"status\":\"completed\",\"output\":\"ok\"}\n尾部";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content == "说明：")
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Tool { raw }) if raw.starts_with("tool file_write"))
    );
    assert!(matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == "尾部"));
}

#[test]
fn empty_and_plain_content_keep_their_natural_block_shapes() {
    assert!(owned_blocks_from_raw("").is_empty());

    let blocks = owned_blocks_from_raw("plain text");

    assert!(
        matches!(blocks.as_slice(), [ParsedChatBlock::Text { content }] if content == "plain text")
    );
}

#[test]
fn embedded_tool_like_content_without_a_boundary_stays_text() {
    let raw = "prefix tool bash\n{\"status\":\"completed\"}\n";

    let blocks = owned_blocks_from_raw(raw);

    assert!(matches!(blocks.as_slice(), [ParsedChatBlock::Text { content }] if content == raw));
}

#[test]
fn tool_after_ascii_colon_and_whitespace_is_parsed() {
    let raw = "说明: \ttool bash\n{}\n尾部";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content == "说明: \t")
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Tool { raw }) if raw == "tool bash\n{}\n")
    );
    assert!(matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == "尾部"));
}

#[test]
fn multiline_tool_json_is_accumulated_until_complete() {
    let raw = "tool bash\n{\n  \"status\": \"completed\"\n}\nanswer";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Tool { raw }) if raw == "tool bash\n{\n  \"status\": \"completed\"\n}\n")
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Text { content }) if content == "answer")
    );
}

#[test]
fn tool_json_ending_at_eof_is_parsed_without_a_trailing_newline() {
    let raw = "tool bash\n{\"status\":\"completed\"}";

    let blocks = owned_blocks_from_raw(raw);

    assert!(matches!(blocks.as_slice(), [ParsedChatBlock::Tool { raw: parsed }] if parsed == raw));
}

#[test]
fn consecutive_tool_blocks_are_kept_separate() {
    let raw = "tool first\n{}\ntool second\n{}\n";

    let blocks = owned_blocks_from_raw(raw);

    assert_eq!(blocks.len(), 2);
    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Tool { raw }) if raw == "tool first\n{}\n")
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Tool { raw }) if raw == "tool second\n{}\n")
    );
}

#[test]
fn tool_before_think_is_selected_as_the_first_special_block() {
    let raw = "tool bash\n{}\n<think>plan</think>answer";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Tool { raw }) if raw == "tool bash\n{}\n")
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Think { content, open }) if content == "plan" && !open)
    );
    assert!(
        matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == "answer")
    );
}

#[test]
fn closed_think_splits_an_embedded_tool_after_reasoning_text() {
    let raw = "<think>plan\ntool bash\n{}\n</think>answer";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Think { content, open }) if content == "plan\n" && !open)
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Tool { raw }) if raw == "tool bash\n{}\n")
    );
    assert!(
        matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == "answer")
    );
}

#[test]
fn closed_think_starting_with_a_tool_keeps_an_empty_think_boundary() {
    let raw = "<think>tool bash\n{}\n</think>answer";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Think { content, open }) if content.is_empty() && !open)
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Tool { raw }) if raw == "tool bash\n{}\n")
    );
    assert!(
        matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == "answer")
    );
}

#[test]
fn unclosed_think_starting_with_a_tool_keeps_an_open_empty_think_boundary() {
    let raw = "<think>tool bash\n{}\nanswer";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Think { content, open }) if content.is_empty() && *open)
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Tool { raw }) if raw == "tool bash\n{}\n")
    );
    assert!(
        matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == "answer")
    );
}

#[test]
fn close_tag_prefix_collision_is_skipped_before_a_valid_tag_with_attributes() {
    let raw = "<think>inside</thinker>middle</think reason=\"done\">tail";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Think { content, open }) if content == "inside</thinker>middle" && !open)
    );
    assert!(matches!(blocks.get(1), Some(ParsedChatBlock::Text { content }) if content == "tail"));
}

#[test]
fn unterminated_think_open_tag_stays_text() {
    let raw = "before <think";

    let blocks = owned_blocks_from_raw(raw);

    assert!(matches!(blocks.as_slice(), [ParsedChatBlock::Text { content }] if content == raw));
}

#[test]
fn unterminated_think_close_tag_keeps_the_think_open() {
    let blocks = owned_blocks_from_raw("<think>inside</think");

    assert!(
        matches!(blocks.as_slice(), [ParsedChatBlock::Think { content, open }] if content == "inside</think" && *open)
    );
}

#[test]
fn tool_json_after_the_sixty_four_line_limit_stays_text() {
    let mut raw = String::from("tool bash\n");
    for _ in 0..64 {
        raw.push_str("not json\n");
    }
    raw.push_str("{}\n");

    let blocks = owned_blocks_from_raw(&raw);

    assert!(matches!(blocks.as_slice(), [ParsedChatBlock::Text { content }] if content == &raw));
}

#[test]
fn think_tags_accept_non_space_whitespace_after_their_names() {
    let raw = "before<think\nsource=\"stream\">inside</think\tdone>after";

    let blocks = owned_blocks_from_raw(raw);

    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content == "before")
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Think { content, open }) if content == "inside" && !open)
    );
    assert!(matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content == "after"));
}
