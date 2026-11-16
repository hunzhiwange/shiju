use crate::chat_ui::*;
use crate::render_cache::ChatRenderCacheEntry;
use iced::widget::text_editor;
use session_ui_types::{ChatMessage, ChatRole};
use std::collections::{HashMap, HashSet};

// Own only the state borrowed by ChatUi. No desktop runtime or startup tasks.
#[derive(Default)]
struct Fixture {
    chat: Vec<session_ui_types::ChatMessage>,
    chat_copy_hash_cache: Vec<Option<u64>>,
    chat_explore_expanded: HashSet<u64>,
    chat_explore_summary_animations:
        HashMap<u128, desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState>,
    chat_height_index: crate::height_index::ChatHeightIndex,
    chat_markdown_block_editors: HashMap<u128, text_editor::Content>,
    chat_markdown_blocks: HashMap<u64, Vec<crate::message_view::markdown::MarkdownBlock>>,
    chat_markdown_selection: Option<ChatMarkdownSelection>,
    chat_message_editors: Vec<text_editor::Content>,
    chat_message_estimated_heights: Vec<f32>,
    chat_message_expanded: HashSet<usize>,
    chat_message_measured_heights: HashMap<usize, f32>,
    chat_render_cache: HashMap<usize, ChatRenderCacheEntry>,
    chat_reset_menu_idx: Option<usize>,
    chat_scroll_offset_y: f32,
    chat_scroll_viewport_h: f32,
    chat_special_text_editors: HashMap<u64, text_editor::Content>,
    chat_think_collapsed: HashSet<u64>,
    chat_think_editors: HashMap<u64, text_editor::Content>,
    chat_think_expanded: HashSet<u64>,
    chat_think_hovered_idx: Option<u64>,
    chat_think_scroll_ids: HashMap<u64, iced::widget::Id>,
    chat_tool_expanded: HashSet<u64>,
    chat_tool_file_expanded: HashSet<String>,
    chat_tool_file_hovered: Option<String>,
    chat_tool_hovered_idx: Option<u64>,
    chat_tool_text_editors: HashMap<u128, text_editor::Content>,
    chat_virtualizer: crate::virtualizer::ChatVirtualizer,
    chat_visible_text_cache: Vec<Option<String>>,
    tool_detail_dialog:
        Option<desktop_session::message::chat::input::tool_detail::ToolDetailDialog>,
    message_meta_texts: Vec<Option<desktop_i18n::chat::MessageMeta>>,
    user_question_markers: Vec<desktop_session::session::active::UserQuestionMarker>,
    user_question_indexed_chat_len: usize,
    prepared_chat_ui_chunks: HashSet<usize>,
    preparing_chat_ui_chunks: HashSet<usize>,
    dirty_preparing_chat_ui_chunks: HashSet<usize>,
    last_visited_chat_ui_chunk_start: Option<usize>,
    pinned_chat_ui_chunk_start: Option<usize>,
    desktop_locale: config_ui::DesktopLocale,
    dialogue_flow_show_reasoning_summary: bool,
    is_requesting: bool,
    window_width: f32,
    is_dark: bool,
}
impl Fixture {
    fn ui(&mut self) -> ChatUi<'_> {
        ChatUi {
            chat: &self.chat,
            chat_copy_hash_cache: &mut self.chat_copy_hash_cache,
            chat_explore_expanded: &mut self.chat_explore_expanded,
            chat_explore_summary_animations: &mut self.chat_explore_summary_animations,
            chat_height_index: &mut self.chat_height_index,
            chat_markdown_block_editors: &mut self.chat_markdown_block_editors,
            chat_markdown_blocks: &mut self.chat_markdown_blocks,
            chat_markdown_selection: &mut self.chat_markdown_selection,
            chat_message_editors: &mut self.chat_message_editors,
            chat_message_estimated_heights: &mut self.chat_message_estimated_heights,
            chat_message_expanded: &mut self.chat_message_expanded,
            chat_message_measured_heights: &mut self.chat_message_measured_heights,
            chat_render_cache: &mut self.chat_render_cache,
            chat_reset_menu_idx: &mut self.chat_reset_menu_idx,
            chat_scroll_offset_y: &mut self.chat_scroll_offset_y,
            chat_scroll_viewport_h: &mut self.chat_scroll_viewport_h,
            chat_special_text_editors: &mut self.chat_special_text_editors,
            chat_think_collapsed: &mut self.chat_think_collapsed,
            chat_think_editors: &mut self.chat_think_editors,
            chat_think_expanded: &mut self.chat_think_expanded,
            chat_think_hovered_idx: &mut self.chat_think_hovered_idx,
            chat_think_scroll_ids: &mut self.chat_think_scroll_ids,
            chat_tool_expanded: &mut self.chat_tool_expanded,
            chat_tool_file_expanded: &mut self.chat_tool_file_expanded,
            chat_tool_file_hovered: &mut self.chat_tool_file_hovered,
            chat_tool_hovered_idx: &mut self.chat_tool_hovered_idx,
            chat_tool_text_editors: &mut self.chat_tool_text_editors,
            chat_virtualizer: &mut self.chat_virtualizer,
            chat_visible_text_cache: &mut self.chat_visible_text_cache,
            tool_detail_dialog: &mut self.tool_detail_dialog,
            message_meta_texts: &mut self.message_meta_texts,
            user_question_markers: &mut self.user_question_markers,
            user_question_indexed_chat_len: &mut self.user_question_indexed_chat_len,
            prepared_chat_ui_chunks: &mut self.prepared_chat_ui_chunks,
            preparing_chat_ui_chunks: &mut self.preparing_chat_ui_chunks,
            dirty_preparing_chat_ui_chunks: &mut self.dirty_preparing_chat_ui_chunks,
            last_visited_chat_ui_chunk_start: &mut self.last_visited_chat_ui_chunk_start,
            pinned_chat_ui_chunk_start: &mut self.pinned_chat_ui_chunk_start,
            desktop_locale: self.desktop_locale,
            dialogue_flow_show_reasoning_summary: self.dialogue_flow_show_reasoning_summary,
            is_requesting: self.is_requesting,
            timeline: None,
            window_width: self.window_width,
            is_dark: self.is_dark,
        }
    }
    fn schedule(&self) -> ChatUiSchedule<'_> {
        ChatUiSchedule {
            chat: &self.chat,
            chat_virtualizer: &self.chat_virtualizer,
            pinned_chat_ui_chunk_start: &self.pinned_chat_ui_chunk_start,
            last_visited_chat_ui_chunk_start: &self.last_visited_chat_ui_chunk_start,
            chat_message_expanded: &self.chat_message_expanded,
            chat_think_expanded: &self.chat_think_expanded,
            is_requesting: self.is_requesting,
        }
    }
}
fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.into(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
#[test]
fn chat_virtualizer_state_initializes_from_chat() {
    let mut app = Fixture::default();
    app.chat = vec![
        ChatMessage {
            role: ChatRole::User,
            content: "hello".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "hi there".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    app.ui().sync_chat_message_estimated_heights_len();

    assert_eq!(app.chat_virtualizer.len(), app.chat.len());
    assert_eq!(app.chat_virtualizer.len(), app.chat_message_estimated_heights.len());
    assert_eq!(app.chat_virtualizer.total_height_px(), app.chat_height_index.total_height());
}

#[test]
fn rebuild_chat_message_estimated_heights_uses_uniform_initial_height() {
    let mut app = Fixture::default();
    app.chat = vec![
        message(ChatRole::User, "short"),
        message(ChatRole::Assistant, &"very long\n".repeat(20_000)),
    ];

    app.ui().rebuild_chat_message_estimated_heights();

    assert_eq!(
        app.chat_message_estimated_heights,
        vec![CHAT_INITIAL_MESSAGE_HEIGHT; app.chat.len()]
    );
    assert_eq!(app.chat_virtualizer.len(), app.chat.len());
}

#[test]
fn chat_virtualizer_state_resyncs_after_prepared_chunk() {
    let mut app = Fixture::default();
    app.chat = vec![
        ChatMessage {
            role: ChatRole::User,
            content: "one".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        ChatMessage {
            role: ChatRole::Assistant,
            content: "two".repeat(120),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    app.ui().sync_chat_message_estimated_heights_len();

    let phase = prepare_chat_ui_chunk_phase(&app.chat, 0, true, false);
    app.ui().apply_prepared_chat_ui_phase(phase);

    assert_eq!(app.chat_virtualizer.len(), app.chat.len());
    assert_eq!(app.chat_virtualizer.total_height_px(), app.chat_height_index.total_height());
}

#[test]
fn chat_scroll_compat_sync_does_not_rebuild_height_caches() {
    let mut app = Fixture::default();
    app.chat_virtualizer =
        crate::virtualizer::ChatVirtualizer::from_estimated_heights(&[80.0, 120.0, 160.0]);
    app.chat_virtualizer.set_viewport_height(100.0);
    app.chat_virtualizer.set_relative_y(0.5);
    app.chat_message_estimated_heights = vec![999.0];
    let height_index_total = app.chat_height_index.total_height();

    app.ui().sync_chat_scroll_compat_fields();

    assert_eq!(app.chat_message_estimated_heights, vec![999.0]);
    assert_eq!(app.chat_height_index.total_height(), height_index_total);
    assert_eq!(app.chat_scroll_viewport_h, 100.0);
    assert_eq!(app.chat_scroll_offset_y, app.chat_virtualizer.relative_y());
}

#[test]
fn stale_prepared_chunk_clears_preparing_state() {
    let mut app = Fixture::default();
    app.chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: "draft".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    let phase = prepare_chat_ui_chunk_phase(&app.chat, 0, false, false);
    app.ui().mark_chat_ui_chunks_preparing(&[0]);
    app.dirty_preparing_chat_ui_chunks.insert(0);

    app.chat[0].content.push_str(" updated");
    app.ui().apply_prepared_chat_ui_phase(phase);

    assert!(!app.preparing_chat_ui_chunks.contains(&0));
    assert!(!app.dirty_preparing_chat_ui_chunks.contains(&0));
    assert!(!app.prepared_chat_ui_chunks.contains(&0));
}

#[test]
fn pin_chat_ui_chunk_rejects_empty_chat_and_clamps_to_last_chunk() {
    let mut app = Fixture::default();

    app.ui().pin_chat_ui_chunk(Some(40));
    assert_eq!(app.pinned_chat_ui_chunk_start, None);

    app.chat = (0..70).map(|idx| message(ChatRole::User, &idx.to_string())).collect();
    app.ui().pin_chat_ui_chunk(Some(usize::MAX));
    assert_eq!(app.pinned_chat_ui_chunk_start, Some(64));

    app.ui().pin_chat_ui_chunk(None);
    assert_eq!(app.pinned_chat_ui_chunk_start, None);
}

#[test]
fn protected_chat_ui_chunk_starts_deduplicates_pinned_and_expanded_messages() {
    let mut app = Fixture::default();
    app.chat = (0..70).map(|idx| message(ChatRole::User, &idx.to_string())).collect();
    app.ui().pin_chat_ui_chunk(Some(35));
    app.chat_message_expanded.extend([1, 35, 100]);
    app.chat_think_expanded.extend([(35_u64) << 32, (65_u64) << 32]);

    let mut protected = app.schedule().protected_chat_ui_chunk_starts();
    protected.sort_unstable();
    assert_eq!(protected, vec![0, 32, 64]);
}

#[test]
fn protected_chat_ui_chunk_starts_keeps_requesting_tail_assistant_chunk() {
    let mut app = Fixture::default();
    app.chat = (0..70)
        .map(|idx| {
            message(if idx == 69 { ChatRole::Assistant } else { ChatRole::User }, &idx.to_string())
        })
        .collect();
    app.is_requesting = true;

    let protected = app.schedule().protected_chat_ui_chunk_starts();

    assert_eq!(protected, vec![64]);
}

#[test]
fn unload_chat_ui_chunk_clears_derived_caches_but_preserves_interaction_state() {
    let mut app = Fixture::default();
    app.chat = vec![message(ChatRole::Assistant, "answer")];
    let PreparedChatUiPhase::Base(chunk) = prepare_chat_ui_chunk_phase(&app.chat, 0, true, false)
    else {
        panic!("base preparation should return a base phase");
    };
    app.chat_render_cache = chunk.render_cache;
    app.chat_visible_text_cache = chunk.visible_texts;
    app.chat_copy_hash_cache = chunk.copy_hashes;
    app.prepared_chat_ui_chunks.insert(0);
    let key = 0_u64;
    app.chat_think_expanded.insert(key);
    app.chat_think_collapsed.insert(key);
    app.chat_explore_expanded.insert(key);
    app.chat_tool_expanded.insert(key);

    app.ui().unload_chat_ui_chunk(0);

    assert!(!app.prepared_chat_ui_chunks.contains(&0));
    assert!(!app.chat_render_cache.contains_key(&0));
    assert_eq!(app.chat_visible_text_cache[0], None);
    assert_eq!(app.chat_copy_hash_cache[0], None);
    assert!(app.chat_think_expanded.contains(&key));
    assert!(app.chat_think_collapsed.contains(&key));
    assert!(app.chat_explore_expanded.contains(&key));
    assert!(app.chat_tool_expanded.contains(&key));
}

#[test]
fn invalidate_chat_ui_chunk_drops_interaction_state_in_target_chunk_only() {
    let mut app = Fixture::default();
    app.chat = (0..40).map(|idx| message(ChatRole::User, &idx.to_string())).collect();
    let target_key = (1_u64) << 32;
    let retained_key = (33_u64) << 32;
    app.chat_think_expanded.extend([target_key, retained_key]);
    app.chat_think_collapsed.extend([target_key, retained_key]);
    app.chat_explore_expanded.extend([target_key, retained_key]);
    app.chat_tool_expanded.extend([target_key, retained_key]);

    app.ui().invalidate_chat_ui_chunk(0);

    assert!(!app.chat_think_expanded.contains(&target_key));
    assert!(!app.chat_think_collapsed.contains(&target_key));
    assert!(!app.chat_explore_expanded.contains(&target_key));
    assert!(!app.chat_tool_expanded.contains(&target_key));
    assert!(app.chat_think_expanded.contains(&retained_key));
    assert!(app.chat_think_collapsed.contains(&retained_key));
    assert!(app.chat_explore_expanded.contains(&retained_key));
    assert!(app.chat_tool_expanded.contains(&retained_key));
}

#[test]
fn invalidate_chat_ui_for_message_idx_ignores_out_of_range_message() {
    let mut app = Fixture::default();
    app.chat = vec![message(ChatRole::User, "only")];
    app.prepared_chat_ui_chunks.insert(0);

    app.ui().invalidate_chat_ui_for_message_idx(1);

    assert!(app.prepared_chat_ui_chunks.contains(&0));
}

#[test]
fn sync_chat_message_estimated_heights_len_truncates_removed_measurements() {
    let mut app = Fixture::default();
    app.chat = vec![message(ChatRole::User, "one"), message(ChatRole::Assistant, "two")];
    app.ui().sync_chat_message_estimated_heights_len();
    app.chat_message_measured_heights.insert(1, 123.0);

    app.chat.truncate(1);
    app.ui().sync_chat_message_estimated_heights_len();

    assert_eq!(app.chat_virtualizer.len(), 1);
    assert_eq!(app.chat_message_estimated_heights.len(), 1);
    assert!(!app.chat_message_measured_heights.contains_key(&1));
}

#[test]
fn preferred_base_chat_ui_chunk_start_uses_visible_window_or_tail_fallback() {
    let mut app = Fixture::default();

    assert_eq!(app.schedule().preferred_base_chat_ui_chunk_start(), 0);

    app.chat = (0..70).map(|idx| message(ChatRole::User, &idx.to_string())).collect();
    assert_eq!(app.schedule().preferred_base_chat_ui_chunk_start(), 64);

    app.chat_virtualizer.sync_len(app.chat.len(), 100.0);
    app.chat_virtualizer.set_viewport_height(100.0);
    app.chat_virtualizer.scroll_to_item_anchor(40, 0.0, 0.0);

    assert_eq!(app.schedule().preferred_base_chat_ui_chunk_start(), 32);
}

#[test]
fn pending_chat_ui_chunk_starts_filters_loaded_chunks_and_tracks_anchor() {
    let mut app = Fixture::default();
    app.chat = (0..100).map(|idx| message(ChatRole::User, &idx.to_string())).collect();
    app.prepared_chat_ui_chunks.insert(32);
    app.preparing_chat_ui_chunks.insert(64);

    let pending = app.ui().pending_chat_ui_chunk_starts(32, 64, true);

    assert_eq!(app.last_visited_chat_ui_chunk_start, Some(32));
    assert!(!pending.contains(&32));
    assert!(!pending.contains(&64));
    assert!(pending.contains(&0));
}

#[test]
fn apply_prepared_chat_ui_phase_missing_cache_entry_marks_chunk_stale() {
    let mut app = Fixture::default();
    app.chat = vec![message(ChatRole::Assistant, "live")];
    app.chat_render_cache.insert(0, ChatRenderCacheEntry::default());
    app.prepared_chat_ui_chunks.insert(0);
    app.preparing_chat_ui_chunks.insert(0);
    app.dirty_preparing_chat_ui_chunks.insert(0);

    app.ui().apply_prepared_chat_ui_phase(PreparedChatUiPhase::Chunk(PreparedChatUiChunk {
        chunk_start_idx: 0,
        chunk_end_idx: 1,
        render_cache: std::collections::HashMap::new(),
        visible_texts: vec![Some("stale".to_string())],
        copy_hashes: vec![Some(1)],
        message_editor_texts: vec![Some("stale".to_string())],
    }));

    assert!(!app.prepared_chat_ui_chunks.contains(&0));
    assert!(!app.preparing_chat_ui_chunks.contains(&0));
    assert!(!app.dirty_preparing_chat_ui_chunks.contains(&0));
    assert!(!app.chat_render_cache.contains_key(&0));
    assert_eq!(app.chat_visible_text_cache[0], None);
}

#[test]
fn prepared_stream_message_updates_only_target_cache() {
    let mut app = Fixture::default();
    app.chat = vec![
        message(ChatRole::Assistant, "first"),
        message(ChatRole::Assistant, "old tail"),
        message(ChatRole::Assistant, "neighbor"),
    ];
    let phase = prepare_chat_ui_chunk_phase(&app.chat, 0, true, false);
    app.ui().apply_prepared_chat_ui_phase(phase);
    let first_hash = app.chat_render_cache[&0].content_hash;
    let neighbor_hash = app.chat_render_cache[&2].content_hash;
    let target_interaction_key = (1_u64) << 32;
    let neighbor_interaction_key = (2_u64) << 32;
    app.chat_tool_expanded.extend([target_interaction_key, neighbor_interaction_key]);

    app.chat[1].content = "new streamed tail".to_string();
    let phase = prepare_chat_ui_message_phase(&app.chat[1], 1, false);
    app.ui().apply_prepared_chat_ui_phase(phase);

    assert_eq!(app.chat_render_cache[&0].content_hash, first_hash);
    assert_eq!(app.chat_render_cache[&2].content_hash, neighbor_hash);
    assert_eq!(
        app.chat_render_cache[&1].content_hash,
        crate::message_view::hash_chat_content("new streamed tail")
    );
    assert_eq!(app.chat_visible_text_cache[1].as_deref(), Some("new streamed tail"));
    assert!(app.chat_tool_expanded.contains(&target_interaction_key));
    assert!(app.chat_tool_expanded.contains(&neighbor_interaction_key));
}

#[test]
fn stale_prepared_stream_message_is_ignored() {
    let mut app = Fixture::default();
    app.chat = vec![message(ChatRole::Assistant, "old")];
    app.chat_render_cache.insert(
        0,
        ChatRenderCacheEntry {
            content_hash: 77,
            display_text: "kept".to_string(),
            ..ChatRenderCacheEntry::default()
        },
    );
    app.chat_visible_text_cache = vec![Some("kept".to_string())];
    let stale_phase = prepare_chat_ui_message_phase(&app.chat[0], 0, false);
    app.chat[0].content = "newer".to_string();

    app.ui().apply_prepared_chat_ui_phase(stale_phase);

    assert_eq!(app.chat_render_cache[&0].content_hash, 77);
    assert_eq!(app.chat_visible_text_cache[0].as_deref(), Some("kept"));
}

#[test]
fn markdown_cache_reuses_unchanged_message() {
    let mut app = Fixture::default();
    app.chat = vec![message(ChatRole::Assistant, "第一段\n\n第二段")];
    let first = prepare_chat_ui_message_phase(&app.chat[0], 0, false);
    app.ui().apply_prepared_chat_ui_phase(first);

    let first_key = crate::message_view::markdown_block_key(0, 0, 0);
    app.chat_markdown_block_editors
        .get_mut(&first_key)
        .expect("first markdown editor")
        .perform(iced::widget::text_editor::Action::SelectAll);

    let unchanged = prepare_chat_ui_message_phase(&app.chat[0], 0, false);
    app.ui().apply_prepared_chat_ui_phase(unchanged);

    assert_eq!(app.chat_markdown_blocks[&0].len(), 2);
    assert_eq!(
        app.chat_markdown_block_editors[&first_key].selection().as_deref(),
        Some("第一段\n\n第二段"),
        "unchanged content should retain the existing editor instead of rebuilding it"
    );
}

#[test]
fn markdown_cache_prunes_offscreen_blocks() {
    let mut app = Fixture::default();
    app.chat = (0..500).map(|idx| message(ChatRole::Assistant, &format!("# 消息 {idx}"))).collect();
    app.ui().sync_chat_message_estimated_heights_len();
    app.chat_virtualizer.set_viewport_height(2_000.0);
    app.chat_virtualizer.scroll_to_item_anchor(250, 0.0, 0.0);

    for idx in 0..app.chat.len() {
        app.chat_markdown_blocks.insert(
            (idx as u64) << 32,
            crate::message_view::parse_markdown_blocks(&format!("# 消息 {idx}")),
        );
        app.chat_markdown_block_editors.insert(
            crate::message_view::markdown_block_key(idx, 0, 0),
            iced::widget::text_editor::Content::with_text(&format!("消息 {idx}")),
        );
    }

    app.ui().prune_chat_heavy_editor_caches(240, 260);

    assert!(!app.chat_markdown_blocks.contains_key(&0));
    assert!(!app.chat_markdown_blocks.contains_key(&((499_u64) << 32)));
    assert!(app.chat_markdown_blocks.contains_key(&((250_u64) << 32)));
    eprintln!(
        "markdown history cache: messages=500, retained_blocks={}, retained_editors={}",
        app.chat_markdown_blocks.len(),
        app.chat_markdown_block_editors.len()
    );
    assert_eq!(app.chat_markdown_blocks.len(), app.chat_markdown_block_editors.len());
    assert!(app.chat_markdown_blocks.len() < 128);
    assert!(app.chat_markdown_block_editors.keys().all(|key| {
        let block_key = ((*key >> 64) as u64) << 32;
        app.chat_markdown_blocks.contains_key(&block_key)
    }));
}

#[test]
fn chat_cache_retention_is_bounded_by_visible_pages() {
    let mut app = Fixture::default();
    app.chat =
        (0..10_000).map(|idx| message(ChatRole::Assistant, &format!("message {idx}"))).collect();
    app.ui().sync_chat_message_estimated_heights_len();
    app.chat_virtualizer.set_viewport_height(2_000.0);
    app.chat_virtualizer.scroll_to_item_anchor(5_000, 0.0, 0.0);
    seed_markdown_cache(&mut app);

    app.ui().prune_chat_heavy_editor_caches(4_992, 5_024);

    assert!(app.chat_markdown_blocks.len() <= 80);
    assert!(app.chat_markdown_blocks.contains_key(&((5_000_u64) << 32)));
}

#[test]
fn chat_cache_retention_pins_active_entry() {
    let mut app = Fixture::default();
    app.chat =
        (0..10_000).map(|idx| message(ChatRole::Assistant, &format!("message {idx}"))).collect();
    app.ui().sync_chat_message_estimated_heights_len();
    app.chat_virtualizer.set_viewport_height(2_000.0);
    app.chat_virtualizer.scroll_to_item_anchor(0, 0.0, 0.0);
    app.is_requesting = true;
    seed_markdown_cache(&mut app);

    app.ui().prune_chat_heavy_editor_caches(0, 32);

    assert!(app.chat_markdown_blocks.contains_key(&((9_999_u64) << 32)));
    assert!(app.chat_markdown_blocks.len() <= 160);
}

#[test]
fn chat_cache_eviction_preserves_canonical_message() {
    let mut app = Fixture::default();
    app.chat =
        (0..256).map(|idx| message(ChatRole::Assistant, &format!("canonical {idx}"))).collect();
    app.ui().sync_chat_message_estimated_heights_len();
    app.chat_virtualizer.set_viewport_height(2_000.0);
    app.chat_virtualizer.scroll_to_item_anchor(144, 0.0, 0.0);
    seed_markdown_cache(&mut app);

    app.ui().prune_chat_heavy_editor_caches(128, 160);
    assert!(!app.chat_markdown_blocks.contains_key(&0));
    assert_eq!(app.chat[0].content, "canonical 0");

    let rebuilt = prepare_chat_ui_message_phase(&app.chat[0], 0, false);
    app.ui().apply_prepared_chat_ui_phase(rebuilt);
    assert!(app.chat_markdown_blocks.contains_key(&0));
    assert_eq!(app.chat[0].content, "canonical 0");
}

#[test]
fn markdown_cache_reuse_performance_probe() {
    let mut app = Fixture::default();
    let mut content = "长回复正文。".repeat(2_000);
    for idx in 0..200 {
        content.push_str(&format!("\n\n- 列表项 {idx}\n\n```rust\nlet item_{idx} = {idx};\n```"));
    }
    app.chat = vec![message(ChatRole::Assistant, &content)];

    let warmup = prepare_chat_ui_message_phase(&app.chat[0], 0, false);
    app.ui().apply_prepared_chat_ui_phase(warmup);

    let mut rebuild_micros = Vec::with_capacity(3);
    for _ in 0..3 {
        app.chat_markdown_blocks.clear();
        app.chat_markdown_block_editors.clear();
        let phase = prepare_chat_ui_message_phase(&app.chat[0], 0, false);
        let started = std::time::Instant::now();
        app.ui().apply_prepared_chat_ui_phase(phase);
        rebuild_micros.push(started.elapsed().as_micros());
    }

    let mut reuse_micros = Vec::with_capacity(3);
    for _ in 0..3 {
        let phase = prepare_chat_ui_message_phase(&app.chat[0], 0, false);
        let started = std::time::Instant::now();
        app.ui().apply_prepared_chat_ui_phase(phase);
        reuse_micros.push(started.elapsed().as_micros());
    }

    eprintln!(
        "markdown performance: bytes={}, rebuild_us={rebuild_micros:?}, reuse_us={reuse_micros:?}, blocks={}, editors={}",
        content.len(),
        app.chat_markdown_blocks[&0].len(),
        app.chat_markdown_block_editors.len()
    );
    assert_eq!(app.chat_markdown_blocks[&0].len(), 401);
    assert!(app.chat_markdown_block_editors.len() <= 1);
}
fn seed_markdown_cache(app: &mut Fixture) {
    for idx in 0..app.chat.len() {
        app.chat_markdown_blocks
            .insert((idx as u64) << 32, crate::message_view::parse_markdown_blocks("cached"));
    }
}
#[test]
fn prepare_chat_ui_chunk_phase_uses_display_text_for_tool_editor_cache() {
    let chat = vec![ChatMessage {
        role: ChatRole::Tool,
        content: concat!(
            "tool grep\n",
            "{\"status\":\"completed\",\"input\":\"selectors\",\"output\":\"2 matches\"}\n"
        )
        .to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let PreparedChatUiPhase::Base(chunk) = prepare_chat_ui_chunk_phase(&chat, 0, true, false)
    else {
        panic!("base phase should be returned");
    };

    assert_eq!(chunk.message_editor_texts[0].as_deref(), Some("2 matches"));
    assert_eq!(chunk.visible_texts[0].as_deref(), Some(chat[0].content.as_str()));
}
