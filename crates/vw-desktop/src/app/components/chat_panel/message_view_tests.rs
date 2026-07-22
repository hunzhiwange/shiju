//! 验证聊天消息视图构建。
//! 测试聚焦消息类型与渲染输出的映射，防止 UI 回归影响可读性。

use super::message_view::{
    assistant_render_blocks, deduped_tool_last_indices, effective_assistant_render_cache,
    explore_summary_text_blocks, hash_chat_content, message_footer_padding, message_view,
    should_highlight_pending_permission_tool, should_prefer_plain_think_body,
    should_render_think_block, summarize_explore_items, think_block_default_expanded,
    think_block_resolved_expanded, tool_card_text_blocks, trailing_tool_tail_text_source_block_idx,
};
use super::tools::{EXPLORE_GROUP_TOOL_IDX, tool_name_from_raw};
use super::utils::strip_internal_tool_trace;
use crate::app::App;
use crate::app::models::{ChatRenderCacheEntry, ChatRole, ParsedChatBlock, ThinkTiming};
use crate::app::ui::chat::split_think;
use std::borrow::Cow;
use std::collections::HashSet;

fn stale_plain_text_cache() -> ChatRenderCacheEntry {
    ChatRenderCacheEntry {
        content_hash: hash_chat_content("plain text cache"),
        blocks: vec![ParsedChatBlock::Text { content: "plain text cache".to_string() }],
        has_special_blocks: false,
        ..ChatRenderCacheEntry::default()
    }
}

#[test]
fn streaming_assistant_uses_live_blocks_when_cache_is_stale() {
    let content = "<think>planning</think>\nvisible answer";
    let render_cache = stale_plain_text_cache();

    let (blocks, has_special_blocks) = assistant_render_blocks(content, &render_cache, true);

    assert!(matches!(blocks.first(), Some(ParsedChatBlock::Think { .. })));
    assert!(matches!(blocks.get(1), Some(ParsedChatBlock::Text { .. })));
    assert!(has_special_blocks);
}

#[test]
fn non_streaming_assistant_rebuilds_blocks_when_cache_is_stale() {
    let content = "<think>planning</think>\nvisible answer";
    let render_cache = stale_plain_text_cache();

    let (blocks, has_special_blocks) = assistant_render_blocks(content, &render_cache, false);

    assert!(matches!(blocks.first(), Some(ParsedChatBlock::Think { .. })));
    assert!(matches!(blocks.get(1), Some(ParsedChatBlock::Text { .. })));
    assert!(has_special_blocks);
}

#[test]
fn non_streaming_assistant_rebuilds_all_think_blocks_when_cache_is_stale() {
    let content = concat!(
        "<think>first pass</think>\n",
        "visible answer\n",
        "<think>second pass</think>\n",
        "final answer"
    );
    let render_cache = stale_plain_text_cache();

    let (blocks, has_special_blocks) = assistant_render_blocks(content, &render_cache, false);

    assert!(has_special_blocks);
    assert_eq!(blocks.len(), 4);
    assert!(matches!(
        blocks.first(),
        Some(ParsedChatBlock::Think { content, open }) if content == "first pass" && !open
    ));
    assert!(matches!(
        blocks.get(2),
        Some(ParsedChatBlock::Think { content, open }) if content == "second pass" && !open
    ));
}

#[test]
fn streaming_assistant_rebuilds_special_text_blocks_when_cache_is_stale() {
    let content = "tool read\n{\"status\":\"completed\",\"output\":\"ok\"}\n总结";
    let render_cache = stale_plain_text_cache();

    let live_cache = effective_assistant_render_cache(
        content,
        &render_cache,
        content,
        hash_chat_content(content),
        true,
        true,
    );

    assert!(matches!(live_cache.blocks.first(), Some(ParsedChatBlock::Tool { .. })));
    assert_eq!(live_cache.special_text_blocks, vec!["总结".to_string()]);
}

#[test]
fn non_streaming_assistant_rebuilds_special_text_blocks_when_cache_is_stale() {
    let content = "tool read\n{\"status\":\"completed\",\"output\":\"ok\"}\n总结";
    let render_cache = stale_plain_text_cache();

    let live_cache = effective_assistant_render_cache(
        content,
        &render_cache,
        content,
        hash_chat_content(content),
        false,
        true,
    );

    assert!(matches!(live_cache.blocks.first(), Some(ParsedChatBlock::Tool { .. })));
    assert_eq!(live_cache.special_text_blocks, vec!["总结".to_string()]);
}

#[test]
fn assistant_render_blocks_extracts_inline_file_write_tool_after_colon() {
    let content = concat!(
        "好的，这是 `compat-cleanup` 技能的中文翻译：tool file_write\n",
        "{\"status\":\"completed\",\"input\":\"{\\\"path\\\":\\\"/tmp/demo.md\\\",\\\"content\\\":\\\"hello\\\\n\\\"}\",\"metadata\":{\"path\":\"demo.md\",\"operation\":\"update\",\"additions\":1,\"deletions\":1,\"changed\":true},\"output\":\"<file_link>\\npath: demo.md\\nopen: file:////tmp/demo.md\\n</file_link>\\nOverwrote demo.md\"}\n\n",
        "`compat-cleanup` 技能已翻译完成。"
    );
    let render_cache = stale_plain_text_cache();

    let (blocks, has_special_blocks) = assistant_render_blocks(content, &render_cache, false);

    assert!(has_special_blocks);
    assert!(
        matches!(blocks.first(), Some(ParsedChatBlock::Text { content }) if content.contains("中文翻译："))
    );
    assert!(
        matches!(blocks.get(1), Some(ParsedChatBlock::Tool { raw }) if raw.starts_with("tool file_write\n"))
    );
    assert!(
        matches!(blocks.get(2), Some(ParsedChatBlock::Text { content }) if content.contains("翻译完成"))
    );
}

#[test]
fn deduped_tool_last_indices_keeps_all_explore_tools() {
    let blocks = vec![
        ParsedChatBlock::Tool {
            raw: "tool read\n{\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\",\\\"offset\\\":0,\\\"limit\\\":10}\",\"status\":\"completed\",\"output\":\"...\"}"
                .to_string(),
        },
        ParsedChatBlock::Tool {
            raw: "tool read\n{\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\",\\\"offset\\\":0,\\\"limit\\\":10}\",\"status\":\"completed\",\"output\":\"...\"}"
                .to_string(),
        },
        ParsedChatBlock::Tool {
            raw: "tool bash\n{\"input\":\"echo 1\",\"status\":\"completed\",\"output\":\"1\"}"
                .to_string(),
        },
        ParsedChatBlock::Tool {
            raw: "tool shell\n{\"input\":\"echo 1\",\"status\":\"completed\",\"output\":\"1\"}"
                .to_string(),
        },
    ];

    let tool_last = deduped_tool_last_indices(&blocks);

    assert_eq!(tool_last.len(), 1);
    assert_eq!(tool_last.get("bash:echo 1"), Some(&3));
    assert!(!tool_last.keys().any(|key| key.starts_with("read:")));
}

#[test]
fn deduped_tool_last_indices_collapses_write_tools_by_path() {
    let blocks = vec![
        ParsedChatBlock::Tool {
            raw: "tool write\n{\"input\":\"{\\\"path\\\":\\\"docs/iced/adding_widgets.md\\\",\\\"content\\\":\\\"<omitted 812 chars>\\\"}\",\"status\":\"completed\",\"output\":\"Updated\"}"
                .to_string(),
        },
        ParsedChatBlock::Tool {
            raw: "tool file_write\n{\"input\":\"{\\\"path\\\":\\\"docs/iced/adding_widgets.md\\\",\\\"content\\\":\\\"real content\\\"}\",\"status\":\"completed\",\"output\":\"Updated\",\"result\":{\"content\":[{\"type\":\"structured_patch\",\"hunks\":[{\"path\":\"docs/iced/adding_widgets.md\",\"header\":\"@@ -1 +1 @@\",\"lines\":[\"-old\",\"+new\"]}]}]}}"
                .to_string(),
        },
    ];

    let tool_last = deduped_tool_last_indices(&blocks);

    assert_eq!(tool_last.len(), 1);
    assert_eq!(tool_last.get("file_write:docs/iced/adding_widgets.md"), Some(&1));
}

#[test]
fn completed_think_blocks_hide_even_when_multiple() {
    assert!(!should_render_think_block(false, 2, false, None));
}

#[test]
fn completed_think_blocks_show_with_reasoning_summary_setting() {
    let timing = ThinkTiming { start_ms: 10, end_ms: Some(20), last_update_ms: 20 };

    assert!(should_render_think_block(true, 1, false, Some(&timing)));
}

#[test]
fn running_think_blocks_show_when_reasoning_summary_is_disabled() {
    let timing = ThinkTiming { start_ms: 10, end_ms: None, last_update_ms: 10 };

    assert!(should_render_think_block(false, 1, true, Some(&timing)));
}

#[test]
fn running_think_block_defaults_collapsed_and_manual_collapse_still_wins() {
    let timing = ThinkTiming { start_ms: 10, end_ms: None, last_update_ms: 10 };
    let key = 42_u64;

    let default_expanded = think_block_default_expanded(false, true, Some(&timing));
    let resolved = think_block_resolved_expanded(
        default_expanded,
        key,
        &HashSet::new(),
        &HashSet::from([key]),
    );

    assert!(!default_expanded);
    assert!(!resolved);
}

#[test]
fn finished_think_block_defaults_collapsed_but_manual_expand_wins() {
    let timing = ThinkTiming { start_ms: 10, end_ms: Some(20), last_update_ms: 20 };
    let key = 7_u64;

    let default_expanded = think_block_default_expanded(false, false, Some(&timing));
    let resolved = think_block_resolved_expanded(
        default_expanded,
        key,
        &HashSet::from([key]),
        &HashSet::new(),
    );

    assert!(!default_expanded);
    assert!(resolved);
}

#[test]
fn finished_think_block_defaults_collapsed_even_when_reasoning_summary_is_enabled() {
    let timing = ThinkTiming { start_ms: 10, end_ms: Some(20), last_update_ms: 20 };

    assert!(!think_block_default_expanded(true, false, Some(&timing)));
}

#[test]
fn running_think_block_defaults_collapsed_when_reasoning_summary_is_enabled() {
    let timing = ThinkTiming { start_ms: 10, end_ms: None, last_update_ms: 10 };

    assert!(!think_block_default_expanded(true, true, Some(&timing)));
}

#[test]
fn running_think_blocks_prefer_plain_text_body() {
    assert!(should_prefer_plain_think_body(true, false));
    assert!(should_prefer_plain_think_body(false, true));
    assert!(!should_prefer_plain_think_body(false, false));
}

#[test]
fn trailing_tool_tail_text_source_block_idx_ignores_blank_tail_after_tools() {
    let content = "项目概览\n\ntool read\n{\"status\":\"completed\",\"output\":\"ok\"}\n\n";
    let render_cache = stale_plain_text_cache();

    let live_cache = effective_assistant_render_cache(
        content,
        &render_cache,
        content,
        hash_chat_content(content),
        true,
        true,
    );

    assert_eq!(live_cache.special_text_blocks, vec!["项目概览".to_string()]);
    assert_eq!(trailing_tool_tail_text_source_block_idx(&live_cache.blocks), Some(0));
}

#[test]
fn trailing_tool_tail_text_source_block_idx_skips_when_text_already_follows_tools() {
    let content = "tool read\n{\"status\":\"completed\",\"output\":\"ok\"}\n项目概览";
    let render_cache = stale_plain_text_cache();

    let live_cache = effective_assistant_render_cache(
        content,
        &render_cache,
        content,
        hash_chat_content(content),
        true,
        true,
    );

    assert_eq!(trailing_tool_tail_text_source_block_idx(&live_cache.blocks), None);
}

#[test]
fn trailing_tool_tail_text_source_block_idx_handles_non_explore_tool_before_explore_suffix() {
    let content = concat!(
        "这是说明文本\n",
        "tool skill\n",
        "{\"status\":\"error\",\"error\":\"未找到技能 \\\"brainstorming\\\"\"}\n",
        "tool read\n",
        "{\"status\":\"completed\",\"output\":\"ok\"}\n"
    );
    let render_cache = stale_plain_text_cache();

    let live_cache = effective_assistant_render_cache(
        content,
        &render_cache,
        content,
        hash_chat_content(content),
        true,
        true,
    );

    assert_eq!(trailing_tool_tail_text_source_block_idx(&live_cache.blocks), Some(0));
}

#[test]
fn trailing_tool_tail_text_source_block_idx_skips_non_explore_tool_suffix() {
    let content = concat!(
        "这是说明文本\n",
        "tool skill\n",
        "{\"status\":\"error\",\"error\":\"未找到技能 \\\"brainstorming\\\"\"}\n"
    );
    let render_cache = stale_plain_text_cache();

    let live_cache = effective_assistant_render_cache(
        content,
        &render_cache,
        content,
        hash_chat_content(content),
        true,
        true,
    );

    assert_eq!(trailing_tool_tail_text_source_block_idx(&live_cache.blocks), None);
}

#[test]
fn split_think_keeps_open_state_when_tool_arrives_before_close_tag() {
    let raw = "<think>分析仓库\ntool read\n{\"status\":\"running\",\"output\":\"\"}";

    let (thinks, visible, thinking_open) = split_think(raw);

    assert_eq!(thinks, vec!["分析仓库\n".to_string()]);
    assert!(visible.is_empty());
    assert!(thinking_open);
}

#[test]
fn summarize_explore_items_uses_latest_status_for_same_tool_call() {
    let items = [
        "tool read\n{\"tool_call_id\":\"call-1\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"running\"}",
        "tool read\n{\"tool_call_id\":\"call-1\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"result\":{\"success\":true}}",
    ];

    let summary = summarize_explore_items(items.iter().copied(), 0, false)
        .expect("explore summary should exist");

    assert_eq!(summary.0, EXPLORE_GROUP_TOOL_IDX);
    assert_eq!(summary.1, "1 次读取");
}

#[test]
fn summarize_explore_items_falls_back_to_identity_when_call_id_missing() {
    let items = [
        "tool grep\n{\"input\":\"{\\\"pattern\\\":\\\"foo\\\",\\\"include\\\":\\\"src/**\\\"}\",\"status\":\"running\"}",
        "tool grep\n{\"input\":\"{\\\"pattern\\\":\\\"foo\\\",\\\"include\\\":\\\"src/**\\\"}\",\"status\":\"completed\"}",
    ];

    let summary = summarize_explore_items(items.iter().copied(), 0, false)
        .expect("explore summary should exist");

    assert_eq!(summary.0, EXPLORE_GROUP_TOOL_IDX);
    assert_eq!(summary.1, "1 次搜索");
}

#[test]
fn summarize_explore_items_keeps_distinct_explore_calls() {
    let items = [
        "tool read\n{\"tool_call_id\":\"call-1\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"completed\"}",
        "tool read\n{\"tool_call_id\":\"call-2\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/b.rs\\\"}\",\"status\":\"completed\"}",
    ];

    let summary = summarize_explore_items(items.iter().copied(), 0, false)
        .expect("explore summary should exist");

    assert_eq!(summary.0, EXPLORE_GROUP_TOOL_IDX);
    assert_eq!(summary.1, "2 次读取");
}

#[test]
fn summarize_explore_items_uses_running_slot_when_forced() {
    let items = [
        "tool read\n{\"tool_call_id\":\"call-1\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"completed\"}",
    ];

    let summary = summarize_explore_items(items.iter().copied(), 0, true)
        .expect("explore summary should exist");

    assert_eq!(summary.0, EXPLORE_GROUP_TOOL_IDX - 1);
    assert_eq!(summary.1, "1 次读取");
}

#[test]
fn explore_summary_text_blocks_merge_on_hidden_think_boundary() {
    let raw = concat!(
        "tool read\n",
        "{\"tool_call_id\":\"call-1\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"completed\"}\n",
        "<think>done</think>\n",
        "tool read\n",
        "{\"tool_call_id\":\"call-2\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/b.rs\\\"}\",\"status\":\"completed\"}\n"
    );

    let summaries = explore_summary_text_blocks(raw, false);

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].1, "2 次读取");
}

#[test]
fn explore_summary_text_blocks_split_on_visible_think_boundary() {
    let raw = concat!(
        "tool read\n",
        "{\"tool_call_id\":\"call-1\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\"}\",\"status\":\"completed\"}\n",
        "<think>done</think>\n",
        "tool read\n",
        "{\"tool_call_id\":\"call-2\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/b.rs\\\"}\",\"status\":\"completed\"}\n"
    );

    let summaries = explore_summary_text_blocks(raw, true);

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].1, "1 次读取");
    assert_eq!(summaries[1].1, "1 次读取");
}

#[test]
fn split_think_counts_empty_open_think_before_tool_block() {
    let raw = "<think>tool grep\n{\"status\":\"running\",\"output\":\"\"}";

    let (thinks, visible, thinking_open) = split_think(raw);

    assert_eq!(thinks, vec![String::new()]);
    assert!(visible.is_empty());
    assert!(thinking_open);
}

#[test]
fn tool_card_text_blocks_use_bash_command_title() {
    let blocks = tool_card_text_blocks(
        "tool shell\n{\"input\":\"cargo check -p vw-desktop --quiet\",\"output\":\"ok\"}",
    );

    assert_eq!(blocks, vec![vec!["cargo check -p vw-desktop --quiet".to_string()]]);
}

#[test]
fn assistant_footer_padding_keeps_left_edge_flush() {
    let padding = message_footer_padding(false);

    assert_eq!(padding.top, 0.0);
    assert_eq!(padding.right, 0.0);
    assert_eq!(padding.bottom, 0.0);
    assert_eq!(padding.left, 0.0);
}

#[test]
fn user_footer_padding_preserves_meta_breathing_room() {
    let padding = message_footer_padding(true);

    assert_eq!(padding.top, 0.0);
    assert_eq!(padding.right, 6.0);
    assert_eq!(padding.bottom, 0.0);
    assert_eq!(padding.left, 6.0);
}

#[test]
fn tool_name_from_raw_canonicalizes_ask_user_question() {
    let raw = "tool AskUserQuestion\n{\"status\":\"completed\",\"output\":\"ok\"}";

    assert_eq!(tool_name_from_raw(raw), Some("question".to_string()));
}

#[test]
fn tool_card_text_blocks_use_question_summary_instead_of_raw_answers() {
    let blocks = tool_card_text_blocks(
        "tool AskUserQuestion\n{\"input\":\"{\\\"questions\\\":[{\\\"header\\\":\\\"确认部署\\\",\\\"question\\\":\\\"现在发布吗？\\\"}]}\",\"output\":\"[[\\\"yes\\\"]]\",\"status\":\"completed\",\"render_hint\":{\"summary\":\"Collected 1 answer(s)\",\"kind\":\"ask_user_question\"}}",
    );

    assert_eq!(blocks, vec![vec!["Collected 1 answer(s)".to_string()]]);
}

#[test]
fn tool_card_text_blocks_use_error_text_for_apply_patch_denial() {
    let blocks = tool_card_text_blocks(
        "tool apply_patch\n{\"input\":\"*** Begin Patch\",\"status\":\"denied\",\"error\":\"Denied by user.\"}",
    );

    assert_eq!(blocks, vec![vec!["Denied by user.".to_string()]]);
}

#[test]
fn tool_card_text_blocks_include_permission_request_details_in_error_body() {
    let blocks = tool_card_text_blocks(
        "tool file_write\n{\"status\":\"denied\",\"error\":\"Request blocked\",\"permission_request\":{\"reason\":\"Approval required for file write\",\"updated_input\":{\"path\":\"src/main.rs\"}}}",
    );

    assert_eq!(
        blocks,
        vec![vec![
            "原因：Approval required for file write\n目标：src/main.rs\nRequest blocked"
                .to_string()
        ]]
    );
}

#[test]
fn tool_card_text_blocks_support_file_write_alias_preview() {
    let blocks = tool_card_text_blocks(
        "tool file_write\n{\"input\":\"{\\\"filePath\\\":\\\"src/main.rs\\\",\\\"content\\\":\\\"fn main() {}\\n\\\"}\",\"status\":\"completed\",\"render_hint\":{\"summary\":\"Created src/main.rs\",\"kind\":\"file_write\"}}",
    );

    assert_eq!(blocks, vec![vec!["Created src/main.rs".to_string(), "fn main() {}\n".to_string()]]);
}

#[test]
fn strip_internal_tool_trace_removes_standalone_ask_user_question_trace() {
    let raw = "AskUserQuestion\n{\"status\":\"completed\",\"output\":\"ok\"}\n最终回答";

    assert_eq!(strip_internal_tool_trace(raw), "最终回答");
}

#[test]
fn strip_internal_tool_trace_removes_toolshell_compact_trace() {
    let raw = "toolshell({\"command\":\"pwd\"})\n最终回答";

    assert_eq!(strip_internal_tool_trace(raw), "最终回答");
}

#[test]
fn tool_card_text_blocks_use_read_summary_parts() {
    let blocks = tool_card_text_blocks(
        "tool read\n{\"input\":\"{\\\"filePath\\\":\\\"/tmp/demo.txt\\\",\\\"offset\\\":10,\\\"limit\\\":5}\",\"output\":\"body\"}",
    );

    assert_eq!(
        blocks,
        vec![vec!["demo.txt".to_string(), "offset=10 limit=5 (line 11-15)".to_string(),]]
    );
}

#[test]
fn tool_card_text_blocks_support_file_edit_preview() {
    let blocks = tool_card_text_blocks(
        "tool edit\n{\"input\":\"{\\\"filePath\\\":\\\"src/lib.rs\\\",\\\"old_string\\\":\\\"old\\\",\\\"new_string\\\":\\\"fn run() {}\\n\\\"}\",\"status\":\"completed\",\"render_hint\":{\"summary\":\"Updated src/lib.rs\",\"kind\":\"file_edit\"}}",
    );

    assert_eq!(blocks, vec![vec!["Updated src/lib.rs".to_string(), "fn run() {}\n".to_string()]]);
}

#[test]
fn tool_card_text_blocks_support_notebook_edit_preview() {
    let blocks = tool_card_text_blocks(
        "tool notebook_edit\n{\"input\":\"{\\\"path\\\":\\\"demo.ipynb\\\",\\\"edit_type\\\":\\\"edit\\\",\\\"new_code\\\":[\\\"print(1)\\\",\\\"print(2)\\\"]}\",\"status\":\"completed\",\"render_hint\":{\"summary\":\"edit cell 3\",\"kind\":\"notebook_edit\"}}",
    );

    assert_eq!(blocks, vec![vec!["edit cell 3".to_string(), "print(1)\nprint(2)".to_string()]]);
}

#[test]
fn tool_card_text_blocks_use_web_fetch_body_preview() {
    let blocks = tool_card_text_blocks(
        "tool web_fetch\n{\"status\":\"completed\",\"output\":\"Example page body\",\"render_hint\":{\"summary\":\"Fetched https://example.com\",\"kind\":\"web_fetch\"}}",
    );

    assert_eq!(blocks, vec![vec!["Example page body".to_string()]]);
}

#[test]
fn tool_card_text_blocks_extract_brief_message_and_attachments() {
    let blocks = tool_card_text_blocks(
        "tool Brief\n{\"status\":\"completed\",\"result\":{\"data\":{\"message\":\"## 已完成\\n请查看附件。\",\"attachments\":[{\"path\":\"/tmp/reports/chart.png\",\"size\":2048,\"isImage\":true}],\"status\":\"proactive\",\"sentAt\":\"2026-04-29T10:20:30Z\"},\"model_result\":\"Message delivered to user. (1 attachment included)\"},\"renderHint\":{\"kind\":\"brief\",\"summary\":\"## 已完成 请查看附件。\",\"metadata\":{\"canonical_tool_id\":\"brief\",\"attachment_count\":1,\"status\":\"proactive\"}}}",
    );

    assert_eq!(
        blocks,
        vec![vec![
            "## 已完成\n请查看附件。".to_string(),
            "[image] reports/chart.png (2.0 KB)".to_string(),
        ]]
    );
}

#[test]
fn tool_card_text_blocks_use_agenttool_summary_before_output() {
    let blocks = tool_card_text_blocks(
        "tool AgentTool\n{\"input\":\"{\\\"agent\\\":\\\"reviewer\\\",\\\"prompt\\\":\\\"Review auth flow for regressions\\\"}\",\"status\":\"completed\",\"output\":\"[Agent 'reviewer' (openai/gpt-4)]\\nLooks good.\"}",
    );

    assert_eq!(
        blocks,
        vec![vec![
            "reviewer · Review auth flow for regressions".to_string(),
            "[Agent 'reviewer' (openai/gpt-4)]\nLooks good.".to_string(),
        ]]
    );
}

#[test]
fn tool_card_text_blocks_extract_agenttool_background_message() {
    let blocks = tool_card_text_blocks(
        "tool AgentTool\n{\"input\":\"{\\\"agent\\\":\\\"coder\\\",\\\"task\\\":\\\"Refactor query engine\\\"}\",\"status\":\"completed\",\"output\":\"{\\\"session_id\\\":\\\"sub-1\\\",\\\"agent\\\":\\\"coder\\\",\\\"status\\\":\\\"running\\\",\\\"message\\\":\\\"AgentTool launched in background. Use AgentTool with action='get' or action='list' to inspect progress.\\\"}\"}",
    );

    assert_eq!(
        blocks,
        vec![vec![
            "coder · Refactor query engine".to_string(),
            "AgentTool launched in background. Use AgentTool with action='get' or action='list' to inspect progress."
                .to_string(),
        ]]
    );
}

#[test]
fn tool_card_text_blocks_extract_browser_snapshot_preview() {
    let blocks = tool_card_text_blocks(
        "tool browser\n{\"status\":\"completed\",\"output\":\"{\\\"backend\\\":\\\"agent-browser\\\",\\\"title\\\":\\\"Example Domain\\\",\\\"url\\\":\\\"https://example.com\\\"}\",\"result\":{\"data\":{\"action\":\"snapshot\",\"backend\":\"agent-browser\",\"result\":{\"title\":\"Example Domain\",\"url\":\"https://example.com\"}},\"model_result\":\"{\\\"backend\\\":\\\"agent-browser\\\",\\\"title\\\":\\\"Example Domain\\\",\\\"url\\\":\\\"https://example.com\\\"}\"},\"render_hint\":{\"summary\":\"Captured page snapshot\",\"kind\":\"browser\",\"metadata\":{\"action\":\"snapshot\",\"backend\":\"agent-browser\"}}}",
    );

    assert_eq!(
        blocks,
        vec![vec![
            "Captured page snapshot".to_string(),
            "Example Domain".to_string(),
            "https://example.com".to_string(),
        ]]
    );
}

#[test]
fn tool_card_text_blocks_extract_tool_search_matches() {
    let blocks = tool_card_text_blocks(
        "tool tool_search\n{\"status\":\"completed\",\"result\":{\"data\":{\"query\":\"plan\",\"count\":2,\"items\":[{\"id\":\"plan_enter\",\"display_name\":\"EnterPlanMode\",\"reason\":\"name match\"},{\"id\":\"verify_plan_execution\",\"display_name\":\"VerifyPlanExecution\",\"reason\":\"description keyword match\"}]},\"model_result\":\"Found 2 matching tool(s)\"},\"render_hint\":{\"summary\":\"Found 2 matching tool(s)\",\"kind\":\"tool_search\",\"metadata\":{\"query\":\"plan\"}}}",
    );

    assert_eq!(
        blocks,
        vec![vec![
            "状态: available".to_string(),
            "EnterPlanMode: name match".to_string(),
            "VerifyPlanExecution: description keyword match".to_string(),
        ]]
    );
}

#[test]
fn tool_card_text_blocks_emit_explicit_available_status_for_plan_mode() {
    let blocks = tool_card_text_blocks("tool plan_enter\n{\"status\":\"completed\"}");

    assert_eq!(
        blocks,
        vec![vec![
            "状态: available".to_string(),
            "进入规划模式 已接入当前会话工具面。".to_string(),
        ]]
    );
}

#[test]
fn tool_card_text_blocks_emit_plan_mode_guidance_from_result_text() {
    let blocks = tool_card_text_blocks(
        "tool plan_enter\n{\"status\":\"completed\",\"result\":{\"data\":{\"message\":\"Entered plan mode. You should now focus on exploring the codebase and designing an implementation approach.\",\"instructions\":[\"Thoroughly explore the codebase to understand existing patterns.\",\"Design a concrete implementation strategy.\"]},\"content\":[{\"type\":\"text\",\"text\":\"Entered plan mode. You should now focus on exploring the codebase and designing an implementation approach.\\n\\nIn plan mode, you should:\\n1. Thoroughly explore the codebase to understand existing patterns.\\n2. Design a concrete implementation strategy.\"}]}}",
    );

    assert_eq!(
        blocks,
        vec![vec![
            "状态: available".to_string(),
            "Entered plan mode. You should now focus on exploring the codebase and designing an implementation approach.\n\nIn plan mode, you should:\n1. Thoroughly explore the codebase to understand existing patterns.\n2. Design a concrete implementation strategy."
                .to_string(),
        ]]
    );
}

#[test]
fn tool_card_text_blocks_emit_planned_status_for_mcp_surface() {
    let blocks = tool_card_text_blocks("tool mcp_example\n{\"status\":\"completed\"}");

    assert_eq!(
        blocks,
        vec![vec![
            "状态: planned".to_string(),
            "mcp_* 集成 当前已明确标记为 planned。".to_string(),
        ]]
    );
}

#[test]
fn should_highlight_pending_permission_tool_only_for_active_request() {
    assert!(should_highlight_pending_permission_tool(Some("perm-2"), Some("perm-2"), false,));
    assert!(should_highlight_pending_permission_tool(None, Some("perm-2"), true));
    assert!(!should_highlight_pending_permission_tool(Some("perm-1"), Some("perm-2"), false,));
}

#[test]
fn message_view_builds_user_message_with_metadata_and_attachment() {
    let app = App::new().0;
    let content = "please review\n\n[DOCUMENT:/tmp/spec.pdf]";
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::User,
        content,
        &[],
        Some(Cow::Borrowed("You · now")),
        &cache,
        true,
    );

    std::hint::black_box(view);
}

#[test]
fn message_view_builds_assistant_special_blocks_without_metadata() {
    let app = App::new().0;
    let content = "<think>inspect</think>\ntool read\n{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"src/lib.rs\\\"}\"}\nanswer";
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(&app, 0, ChatRole::Assistant, content, &[], None, &cache, false);

    std::hint::black_box(view);
}

#[test]
fn message_view_builds_tool_role_with_metadata() {
    let app = App::new().0;
    let content = "tool shell\n{\"status\":\"completed\",\"input\":\"pwd\",\"output\":\"/tmp\"}";
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::Tool,
        content,
        &[],
        Some(Cow::Borrowed("Tool")),
        &cache,
        true,
    );

    std::hint::black_box(view);
}
