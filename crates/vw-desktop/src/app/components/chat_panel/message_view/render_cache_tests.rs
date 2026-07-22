use super::parse::hash_chat_content;
use super::render_cache::{
    assistant_render_blocks, build_render_cache_entry, effective_assistant_render_cache,
    resolve_visible_text_and_copy_hash,
};
use crate::app::App;
use crate::app::models::ParsedChatBlock;

#[test]
fn build_render_cache_entry_marks_special_blocks() {
    let raw = "<think>plan</think>\nvisible";
    let entry = build_render_cache_entry(raw, "visible", hash_chat_content("visible"), true);

    assert!(entry.has_special_blocks);
    assert!(matches!(entry.blocks.first(), Some(ParsedChatBlock::Think { .. })));
}

#[test]
fn effective_assistant_render_cache_rebuilds_stale_cache() {
    let old = build_render_cache_entry("old", "old", hash_chat_content("old"), true);
    let rebuilt = effective_assistant_render_cache(
        "tool bash\n{\"status\":\"completed\"}",
        &old,
        "tool bash",
        hash_chat_content("tool bash"),
        false,
        true,
    );

    assert!(matches!(rebuilt.blocks.first(), Some(ParsedChatBlock::Tool { .. })));
}

#[test]
fn assistant_render_blocks_uses_cache_when_fresh() {
    let raw = "plain text";
    let entry = build_render_cache_entry(raw, raw, hash_chat_content(raw), true);

    let (blocks, has_special) = assistant_render_blocks(raw, &entry, false);

    assert!(!has_special);
    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content == "plain text")
    );
}

#[test]
fn build_render_cache_entry_merges_explore_across_hidden_think() {
    let raw = concat!(
        "tool read\n",
        "{\"tool_call_id\":\"call-1\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"completed\"}\n",
        "<think>done</think>\n",
        "tool read\n",
        "{\"tool_call_id\":\"call-2\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/b.rs\\\"}\",\"status\":\"completed\"}\n"
    );
    let entry = build_render_cache_entry(raw, raw, hash_chat_content(raw), false);

    assert_eq!(entry.explore_summary_text_blocks.len(), 1);
    assert_eq!(entry.explore_summary_text_blocks[0].1, "2 次读取");
}

#[test]
fn build_render_cache_entry_falls_back_to_tool_text_when_visible_text_is_empty() {
    let raw = "tool bash\n{\"input\":\"pwd\",\"status\":\"completed\",\"output\":\"/tmp\"}\n";
    let entry = build_render_cache_entry(raw, "", hash_chat_content(""), true);

    assert_eq!(entry.display_text, "pwd");
    assert_eq!(entry.preview_text, "pwd");
    assert!(entry.has_special_blocks);
}

#[test]
fn build_render_cache_entry_marks_large_and_foldable_by_code_blocks() {
    let visible = "```text\nx\n```\n".repeat(6);
    let entry = build_render_cache_entry(&visible, &visible, hash_chat_content(&visible), true);

    assert!(entry.is_large_message);
    assert!(entry.foldable);
    assert!(entry.estimated_expanded_height >= entry.estimated_collapsed_height);
}

#[test]
fn build_render_cache_entry_collects_text_and_tool_card_blocks() {
    let raw = concat!(
        "visible text\n",
        "tool bash\n",
        "{\"input\":\"pwd\",\"status\":\"completed\",\"output\":\"/tmp\"}\n",
        "<think>still running",
    );
    let entry = build_render_cache_entry(raw, raw, hash_chat_content(raw), true);

    assert!(entry.special_text_blocks.iter().any(|text| text.contains("visible text")));
    assert!(entry.tool_card_text_blocks.iter().flatten().any(|text| text == "pwd"));
    assert!(entry.estimated_expanded_height >= entry.estimated_collapsed_height);
}

#[test]
fn build_render_cache_entry_truncates_preview_for_long_visible_text() {
    let visible = "x".repeat(2500);
    let entry = build_render_cache_entry(&visible, &visible, hash_chat_content(&visible), true);

    assert_eq!(entry.display_text.len(), 2500);
    assert!(entry.preview_text.len() < entry.display_text.len());
    assert!(!entry.is_large_message);
}

#[test]
fn effective_assistant_render_cache_borrows_fresh_cache() {
    let raw = "fresh";
    let entry = build_render_cache_entry(raw, raw, hash_chat_content(raw), false);
    let resolved =
        effective_assistant_render_cache(raw, &entry, raw, hash_chat_content(raw), false, false);

    assert!(matches!(resolved, std::borrow::Cow::Borrowed(_)));
}

#[test]
fn effective_assistant_render_cache_rebuilds_when_reasoning_summary_flag_changes() {
    let raw = "<think>plan</think>\nanswer";
    let entry = build_render_cache_entry(raw, "answer", hash_chat_content("answer"), false);
    let resolved = effective_assistant_render_cache(
        raw,
        &entry,
        "answer",
        hash_chat_content("answer"),
        true,
        true,
    );

    assert!(matches!(resolved, std::borrow::Cow::Owned(_)));
}

#[test]
fn assistant_render_blocks_rebuilds_stale_blocks() {
    let entry = build_render_cache_entry("plain", "plain", hash_chat_content("plain"), true);

    let (blocks, has_special) =
        assistant_render_blocks("tool bash\n{\"status\":\"completed\"}", &entry, true);

    assert!(has_special);
    assert!(matches!(blocks.first(), Some(ParsedChatBlock::Tool { .. })));
}

#[test]
fn resolve_visible_text_uses_fresh_cache_and_recomputes_stale_content() {
    let mut app = App::new().0;
    app.chat_visible_text_cache = vec![Some("cached visible".to_string())];
    app.chat_copy_hash_cache = vec![Some(42)];
    let cached_content = "<think>secret</think>cached body";
    let cache = build_render_cache_entry(cached_content, "cached body", 42, true);

    let (cached_visible, cached_hash) =
        resolve_visible_text_and_copy_hash(&app, 0, cached_content, &cache);
    assert!(matches!(cached_visible, std::borrow::Cow::Borrowed("cached visible")));
    assert_eq!(cached_hash, 42);

    let stale_content = "<think>new secret</think>new body";
    let (recomputed_visible, recomputed_hash) =
        resolve_visible_text_and_copy_hash(&app, 0, stale_content, &cache);
    assert!(matches!(recomputed_visible, std::borrow::Cow::Owned(ref text) if text == "new body"));
    assert_eq!(recomputed_hash, hash_chat_content("new body"));
}

#[test]
fn resolve_visible_text_recomputes_when_fresh_cache_lacks_visible_text_or_hash() {
    let mut app = App::new().0;
    app.chat_visible_text_cache = vec![None];
    app.chat_copy_hash_cache = vec![None];
    let content = "<think>secret</think>body";
    let cache = build_render_cache_entry(content, "body", hash_chat_content("body"), true);

    let (visible, copy_hash) = resolve_visible_text_and_copy_hash(&app, 0, content, &cache);

    assert!(matches!(visible, std::borrow::Cow::Owned(ref text) if text == "body"));
    assert_eq!(copy_hash, hash_chat_content("body"));
}
