#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("chat_ui_tests"));
}

#[test]
fn chat_virtualizer_state_initializes_from_chat() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![
        super::ChatMessage {
            role: super::ChatRole::User,
            content: "hello".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        super::ChatMessage {
            role: super::ChatRole::Assistant,
            content: "hi there".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    app.sync_chat_message_estimated_heights_len();

    assert_eq!(app.chat_virtualizer.len(), app.chat.len());
    assert_eq!(app.chat_virtualizer.len(), app.chat_message_estimated_heights.len());
    assert_eq!(app.chat_virtualizer.total_height_px(), app.chat_height_index.total_height());
}

#[test]
fn rebuild_chat_message_estimated_heights_uses_uniform_initial_height() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![
        message(super::ChatRole::User, "short"),
        message(super::ChatRole::Assistant, &"very long\n".repeat(20_000)),
    ];

    app.rebuild_chat_message_estimated_heights();

    assert_eq!(
        app.chat_message_estimated_heights,
        vec![super::CHAT_INITIAL_MESSAGE_HEIGHT; app.chat.len()]
    );
    assert_eq!(app.chat_virtualizer.len(), app.chat.len());
}

#[test]
fn chat_virtualizer_state_resyncs_after_prepared_chunk() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![
        super::ChatMessage {
            role: super::ChatRole::User,
            content: "one".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        super::ChatMessage {
            role: super::ChatRole::Assistant,
            content: "two".repeat(120),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];
    app.sync_chat_message_estimated_heights_len();

    let phase = super::prepare_chat_ui_chunk_phase(&app.chat, 0, true, false);
    app.apply_prepared_chat_ui_phase(phase);

    assert_eq!(app.chat_virtualizer.len(), app.chat.len());
    assert_eq!(app.chat_virtualizer.total_height_px(), app.chat_height_index.total_height());
}

#[test]
fn chat_scroll_compat_sync_does_not_rebuild_height_caches() {
    let (mut app, _task) = super::App::new();
    app.chat_virtualizer =
        crate::app::components::chat_panel::virtualizer::ChatVirtualizer::from_estimated_heights(
            &[80.0, 120.0, 160.0],
        );
    app.chat_virtualizer.set_viewport_height(100.0);
    app.chat_virtualizer.set_relative_y(0.5);
    app.chat_message_estimated_heights = vec![999.0];
    let height_index_total = app.chat_height_index.total_height();

    app.sync_chat_scroll_compat_fields();

    assert_eq!(app.chat_message_estimated_heights, vec![999.0]);
    assert_eq!(app.chat_height_index.total_height(), height_index_total);
    assert_eq!(app.chat_scroll_viewport_h, 100.0);
    assert_eq!(app.chat_scroll_offset_y, app.chat_virtualizer.relative_y());
}

#[test]
fn stale_prepared_chunk_clears_preparing_state() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![super::ChatMessage {
        role: super::ChatRole::Assistant,
        content: "draft".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    let phase = super::prepare_chat_ui_chunk_phase(&app.chat, 0, false, false);
    app.mark_chat_ui_chunks_preparing(&[0]);
    app.active_session_view_state.dirty_preparing_chat_ui_chunks.insert(0);

    app.chat[0].content.push_str(" updated");
    app.apply_prepared_chat_ui_phase(phase);

    assert!(!app.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
    assert!(!app.active_session_view_state.dirty_preparing_chat_ui_chunks.contains(&0));
    assert!(!app.active_session_view_state.prepared_chat_ui_chunks.contains(&0));
}

fn message(role: super::ChatRole, content: &str) -> super::ChatMessage {
    super::ChatMessage { role, content: content.to_string(), model: None, think_timing: Vec::new(), processing_timing: None }
}

#[test]
fn parse_chat_ui_artifacts_keeps_vector_slots_aligned_and_hides_assistant_thinking() {
    let chat = vec![
        message(super::ChatRole::User, "question"),
        message(super::ChatRole::Assistant, "<think>private</think>answer"),
        message(super::ChatRole::Tool, "tool output"),
    ];

    let parsed = super::parse_chat_ui_artifacts(&chat, false);

    assert_eq!(parsed.visible_texts.len(), chat.len());
    assert_eq!(parsed.copy_hashes.len(), chat.len());
    assert_eq!(parsed.message_editor_texts.len(), chat.len());
    assert_eq!(parsed.render_cache.len(), chat.len());
    assert_eq!(parsed.visible_texts[0].as_deref(), Some("question"));
    assert_eq!(parsed.visible_texts[1].as_deref(), Some("answer"));
    assert_eq!(parsed.visible_texts[2].as_deref(), Some("tool output"));
}

#[test]
fn chat_ui_chunk_start_idx_aligns_boundary_and_tail_indices() {
    assert_eq!(super::chat_ui_chunk_start_idx(0), 0);
    assert_eq!(super::chat_ui_chunk_start_idx(super::CHAT_UI_CHUNK_SIZE - 1), 0);
    assert_eq!(
        super::chat_ui_chunk_start_idx(super::CHAT_UI_CHUNK_SIZE),
        super::CHAT_UI_CHUNK_SIZE
    );
    assert_eq!(
        super::chat_ui_chunk_start_idx(super::CHAT_UI_CHUNK_SIZE * 2 + 7),
        super::CHAT_UI_CHUNK_SIZE * 2
    );
}

#[test]
fn chat_ui_chunk_bounds_clamps_empty_partial_and_out_of_range_chunks() {
    assert_eq!(super::chat_ui_chunk_bounds(0, 0), (0, 0));
    assert_eq!(super::chat_ui_chunk_bounds(70, 0), (0, 32));
    assert_eq!(super::chat_ui_chunk_bounds(70, 64), (64, 70));
    assert_eq!(super::chat_ui_chunk_bounds(70, usize::MAX), (70, 70));
}

#[test]
fn visible_window_bounds_reject_empty_chat_and_normalize_reversed_ranges() {
    assert_eq!(super::chat_ui_visible_window_bounds(0, 0, 0), None);
    assert_eq!(super::chat_ui_visible_window_bounds(10, 3, 3), Some((3, 4)));
    assert_eq!(super::chat_ui_visible_window_bounds(10, 20, 0), Some((9, 10)));
}

#[test]
fn visible_anchor_uses_middle_message_and_chunk_alignment() {
    assert_eq!(super::chat_ui_visible_anchor_chunk_start(0, 0, 0), None);
    assert_eq!(super::chat_ui_visible_anchor_chunk_start(100, 32, 96), Some(32));
    assert_eq!(super::chat_ui_visible_anchor_chunk_start(100, 64, 96), Some(64));
}

#[test]
fn prioritize_chat_ui_chunk_starts_handles_empty_and_neighbor_free_windows() {
    assert_eq!(super::prioritize_chat_ui_chunk_starts(0, 0, 0, true, None, 0), (Vec::new(), None));

    let (chunks, anchor) = super::prioritize_chat_ui_chunk_starts(96, 40, 72, false, Some(32), 32);
    assert_eq!(anchor, Some(32));
    assert_eq!(chunks, vec![32, 64]);
}

#[test]
fn prepare_chat_ui_chunk_phase_offsets_cache_keys_and_strips_assistant_thinking() {
    let chat = vec![
        message(super::ChatRole::Assistant, "<think>hidden</think>visible"),
        message(super::ChatRole::User, "question"),
    ];

    let super::PreparedChatUiPhase::Chunk(chunk) =
        super::prepare_chat_ui_chunk_phase(&chat, 32, false, false)
    else {
        panic!("non-base preparation should return a chunk phase");
    };

    assert_eq!((chunk.chunk_start_idx, chunk.chunk_end_idx), (32, 34));
    assert_eq!(chunk.visible_texts[0].as_deref(), Some("visible"));
    assert_eq!(chunk.visible_texts[1].as_deref(), Some("question"));
    assert!(chunk.copy_hashes.iter().all(Option::is_some));
    assert_eq!(chunk.render_cache.len(), 2);
    assert!(chunk.render_cache.contains_key(&32));
    assert!(chunk.render_cache.contains_key(&33));
}

#[test]
fn explore_summary_animation_key_keeps_message_and_group_components_distinct() {
    let key = super::explore_summary_animation_key(7, 11);

    assert_eq!((key >> 64) as usize, 7);
    assert_eq!((key & u64::MAX as u128) as usize, 11);
    assert_ne!(key, super::explore_summary_animation_key(11, 7));
}

#[test]
fn pin_chat_ui_chunk_rejects_empty_chat_and_clamps_to_last_chunk() {
    let (mut app, _task) = super::App::new();

    app.pin_chat_ui_chunk(Some(40));
    assert_eq!(app.active_session_view_state.pinned_chat_ui_chunk_start, None);

    app.chat = (0..70).map(|idx| message(super::ChatRole::User, &idx.to_string())).collect();
    app.pin_chat_ui_chunk(Some(usize::MAX));
    assert_eq!(app.active_session_view_state.pinned_chat_ui_chunk_start, Some(64));

    app.pin_chat_ui_chunk(None);
    assert_eq!(app.active_session_view_state.pinned_chat_ui_chunk_start, None);
}

#[test]
fn protected_chat_ui_chunk_starts_deduplicates_pinned_and_expanded_messages() {
    let (mut app, _task) = super::App::new();
    app.chat = (0..70).map(|idx| message(super::ChatRole::User, &idx.to_string())).collect();
    app.pin_chat_ui_chunk(Some(35));
    app.chat_message_expanded.extend([1, 35, 100]);
    app.chat_think_expanded.extend([(35_u64) << 32, (65_u64) << 32]);

    let mut protected = app.protected_chat_ui_chunk_starts();
    protected.sort_unstable();
    assert_eq!(protected, vec![0, 32, 64]);
}

#[test]
fn protected_chat_ui_chunk_starts_keeps_requesting_tail_assistant_chunk() {
    let (mut app, _task) = super::App::new();
    app.active_session_id = Some("active".to_string());
    app.chat = (0..70)
        .map(|idx| {
            message(
                if idx == 69 { super::ChatRole::Assistant } else { super::ChatRole::User },
                &idx.to_string(),
            )
        })
        .collect();
    app.current_session_runtime_mut().is_requesting = true;

    let protected = app.protected_chat_ui_chunk_starts();

    assert_eq!(protected, vec![64]);
}

#[test]
fn unload_chat_ui_chunk_clears_derived_caches_but_preserves_interaction_state() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![message(super::ChatRole::Assistant, "answer")];
    let super::PreparedChatUiPhase::Base(chunk) =
        super::prepare_chat_ui_chunk_phase(&app.chat, 0, true, false)
    else {
        panic!("base preparation should return a base phase");
    };
    app.chat_render_cache = chunk.render_cache;
    app.chat_visible_text_cache = chunk.visible_texts;
    app.chat_copy_hash_cache = chunk.copy_hashes;
    app.active_session_view_state.prepared_chat_ui_chunks.insert(0);
    let key = 0_u64;
    app.chat_think_expanded.insert(key);
    app.chat_think_collapsed.insert(key);
    app.chat_explore_expanded.insert(key);
    app.chat_tool_expanded.insert(key);

    app.unload_chat_ui_chunk(0);

    assert!(!app.active_session_view_state.prepared_chat_ui_chunks.contains(&0));
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
    let (mut app, _task) = super::App::new();
    app.chat = (0..40).map(|idx| message(super::ChatRole::User, &idx.to_string())).collect();
    let target_key = (1_u64) << 32;
    let retained_key = (33_u64) << 32;
    app.chat_think_expanded.extend([target_key, retained_key]);
    app.chat_think_collapsed.extend([target_key, retained_key]);
    app.chat_explore_expanded.extend([target_key, retained_key]);
    app.chat_tool_expanded.extend([target_key, retained_key]);

    app.invalidate_chat_ui_chunk(0);

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
    let (mut app, _task) = super::App::new();
    app.chat = vec![message(super::ChatRole::User, "only")];
    app.active_session_view_state.prepared_chat_ui_chunks.insert(0);

    app.invalidate_chat_ui_for_message_idx(1);

    assert!(app.active_session_view_state.prepared_chat_ui_chunks.contains(&0));
}

#[test]
fn sync_chat_message_estimated_heights_len_truncates_removed_measurements() {
    let (mut app, _task) = super::App::new();
    app.chat =
        vec![message(super::ChatRole::User, "one"), message(super::ChatRole::Assistant, "two")];
    app.sync_chat_message_estimated_heights_len();
    app.chat_message_measured_heights.insert(1, 123.0);

    app.chat.truncate(1);
    app.sync_chat_message_estimated_heights_len();

    assert_eq!(app.chat_virtualizer.len(), 1);
    assert_eq!(app.chat_message_estimated_heights.len(), 1);
    assert!(!app.chat_message_measured_heights.contains_key(&1));
}

#[test]
fn resolve_render_cache_height_requires_matching_message_content_hash() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![message(super::ChatRole::Assistant, "cached")];
    let content_hash =
        crate::app::components::chat_panel::message_view::hash_chat_content("cached");
    app.chat_render_cache.insert(
        0,
        super::ChatRenderCacheEntry {
            content_hash,
            estimated_expanded_height: 240.0,
            ..super::ChatRenderCacheEntry::default()
        },
    );

    assert_eq!(super::resolve_render_cache_height(&app, 0), Some(240.0));

    app.chat[0].content = "changed".to_string();

    assert_eq!(super::resolve_render_cache_height(&app, 0), None);
    assert_eq!(super::resolve_render_cache_height(&app, 1), None);
}

#[test]
fn preferred_base_chat_ui_chunk_start_uses_visible_window_or_tail_fallback() {
    let (mut app, _task) = super::App::new();

    assert_eq!(app.preferred_base_chat_ui_chunk_start(), 0);

    app.chat = (0..70).map(|idx| message(super::ChatRole::User, &idx.to_string())).collect();
    assert_eq!(app.preferred_base_chat_ui_chunk_start(), 64);

    app.chat_virtualizer.sync_len(app.chat.len(), 100.0);
    app.chat_virtualizer.set_viewport_height(100.0);
    app.chat_virtualizer.scroll_to_item_anchor(40, 0.0, 0.0);

    assert_eq!(app.preferred_base_chat_ui_chunk_start(), 32);
}

#[test]
fn pending_chat_ui_chunk_starts_filters_loaded_chunks_and_tracks_anchor() {
    let (mut app, _task) = super::App::new();
    app.chat = (0..100).map(|idx| message(super::ChatRole::User, &idx.to_string())).collect();
    app.active_session_view_state.prepared_chat_ui_chunks.insert(32);
    app.active_session_view_state.preparing_chat_ui_chunks.insert(64);

    let pending = app.pending_chat_ui_chunk_starts(32, 64, true);

    assert_eq!(app.active_session_view_state.last_visited_chat_ui_chunk_start, Some(32));
    assert!(!pending.contains(&32));
    assert!(!pending.contains(&64));
    assert!(pending.contains(&0));
}

#[test]
fn apply_prepared_chat_ui_phase_missing_cache_entry_marks_chunk_stale() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![message(super::ChatRole::Assistant, "live")];
    app.chat_render_cache.insert(0, super::ChatRenderCacheEntry::default());
    app.active_session_view_state.prepared_chat_ui_chunks.insert(0);
    app.active_session_view_state.preparing_chat_ui_chunks.insert(0);
    app.active_session_view_state.dirty_preparing_chat_ui_chunks.insert(0);

    app.apply_prepared_chat_ui_phase(super::PreparedChatUiPhase::Chunk(
        super::PreparedChatUiChunk {
            chunk_start_idx: 0,
            chunk_end_idx: 1,
            render_cache: std::collections::HashMap::new(),
            visible_texts: vec![Some("stale".to_string())],
            copy_hashes: vec![Some(1)],
            message_editor_texts: vec![Some("stale".to_string())],
        },
    ));

    assert!(!app.active_session_view_state.prepared_chat_ui_chunks.contains(&0));
    assert!(!app.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
    assert!(!app.active_session_view_state.dirty_preparing_chat_ui_chunks.contains(&0));
    assert!(!app.chat_render_cache.contains_key(&0));
    assert_eq!(app.chat_visible_text_cache[0], None);
}

#[test]
fn prepared_stream_message_updates_only_target_cache() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![
        message(super::ChatRole::Assistant, "first"),
        message(super::ChatRole::Assistant, "old tail"),
        message(super::ChatRole::Assistant, "neighbor"),
    ];
    app.apply_prepared_chat_ui_phase(super::prepare_chat_ui_chunk_phase(
        &app.chat,
        0,
        true,
        false,
    ));
    let first_hash = app.chat_render_cache[&0].content_hash;
    let neighbor_hash = app.chat_render_cache[&2].content_hash;
    let target_interaction_key = (1_u64) << 32;
    let neighbor_interaction_key = (2_u64) << 32;
    app.chat_tool_expanded.extend([target_interaction_key, neighbor_interaction_key]);

    app.chat[1].content = "new streamed tail".to_string();
    let phase = super::prepare_chat_ui_message_phase(&app.chat[1], 1, false);
    app.apply_prepared_chat_ui_phase(phase);

    assert_eq!(app.chat_render_cache[&0].content_hash, first_hash);
    assert_eq!(app.chat_render_cache[&2].content_hash, neighbor_hash);
    assert_eq!(
        app.chat_render_cache[&1].content_hash,
        crate::app::components::chat_panel::message_view::hash_chat_content("new streamed tail")
    );
    assert_eq!(app.chat_visible_text_cache[1].as_deref(), Some("new streamed tail"));
    assert!(app.chat_tool_expanded.contains(&target_interaction_key));
    assert!(app.chat_tool_expanded.contains(&neighbor_interaction_key));
}

#[test]
fn stale_prepared_stream_message_is_ignored() {
    let (mut app, _task) = super::App::new();
    app.chat = vec![message(super::ChatRole::Assistant, "old")];
    app.chat_render_cache.insert(
        0,
        super::ChatRenderCacheEntry {
            content_hash: 77,
            display_text: "kept".to_string(),
            ..super::ChatRenderCacheEntry::default()
        },
    );
    app.chat_visible_text_cache = vec![Some("kept".to_string())];
    let stale_phase = super::prepare_chat_ui_message_phase(&app.chat[0], 0, false);
    app.chat[0].content = "newer".to_string();

    app.apply_prepared_chat_ui_phase(stale_phase);

    assert_eq!(app.chat_render_cache[&0].content_hash, 77);
    assert_eq!(app.chat_visible_text_cache[0].as_deref(), Some("kept"));
}
