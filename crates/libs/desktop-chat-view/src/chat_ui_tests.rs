fn message(role: super::ChatRole, content: &str) -> super::ChatMessage {
    super::ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("chat_ui_tests"));
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
fn prioritize_chat_ui_chunk_starts_keeps_base_first_on_initial_plan() {
    let (chunk_starts, anchor_chunk_start) =
        super::prioritize_chat_ui_chunk_starts(160, 64, 96, true, None, 32);

    assert_eq!(anchor_chunk_start, Some(64));
    assert_eq!(chunk_starts, vec![32, 64, 96]);
}

#[test]
fn prioritize_chat_ui_chunk_starts_prefers_forward_chunks_when_scrolling_down() {
    let (chunk_starts, anchor_chunk_start) =
        super::prioritize_chat_ui_chunk_starts(192, 72, 104, true, Some(32), 32);

    assert_eq!(anchor_chunk_start, Some(64));
    assert_eq!(chunk_starts, vec![96, 64, 128, 32]);
}

#[test]
fn prioritize_chat_ui_chunk_starts_prefers_backward_chunks_when_scrolling_up() {
    let (chunk_starts, anchor_chunk_start) =
        super::prioritize_chat_ui_chunk_starts(192, 40, 72, true, Some(96), 32);

    assert_eq!(anchor_chunk_start, Some(32));
    assert_eq!(chunk_starts, vec![32, 64, 0, 96]);
}
