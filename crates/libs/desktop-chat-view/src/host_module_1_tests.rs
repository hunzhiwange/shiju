use super::*;
use desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState;
#[test]
fn chunk_scheduler_retains_anchor_and_expanded_text() {
    let mut app = Fixture::default();
    app.chat = (0..260)
        .map(|idx| ChatMessage {
            role: if idx == 259 { ChatRole::Assistant } else { ChatRole::User },
            content: format!("message {idx}"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .collect();
    app.chat_virtualizer.sync_len(app.chat.len(), 100.0);
    app.chat_virtualizer.set_viewport_height(300.0);
    app.chat_virtualizer.scroll_to_item_anchor(170, 0.0, 0.0);
    app.is_requesting = true;

    let anchor_key = (170_u64) << 32;
    let expanded_key = (210_u64) << 32;
    let think_key = (211_u64) << 32;
    let tail_key = (259_u64) << 32;
    let evicted_key = (96_u64) << 32;
    app.chat_message_expanded.insert(210);
    app.chat_think_expanded.insert(think_key);
    app.chat_special_text_editors.insert(anchor_key, text_editor::Content::with_text("anchor"));
    app.chat_special_text_editors.insert(expanded_key, text_editor::Content::with_text("expanded"));
    app.chat_think_editors.insert(think_key, text_editor::Content::with_text("think"));
    app.chat_special_text_editors.insert(tail_key, text_editor::Content::with_text("tail"));
    app.chat_special_text_editors.insert(evicted_key, text_editor::Content::with_text("evicted"));

    app.ui().prune_chat_heavy_editor_caches(64, 96);

    assert!(app.chat_special_text_editors.contains_key(&anchor_key));
    assert!(app.chat_special_text_editors.contains_key(&expanded_key));
    assert!(app.chat_think_editors.contains_key(&think_key));
    assert!(app.chat_special_text_editors.contains_key(&tail_key));
    assert!(!app.chat_special_text_editors.contains_key(&evicted_key));
}

#[test]
fn apply_prepared_chat_ui_phase_preserves_explore_summary_animation_history() {
    let mut app = Fixture::default();
    let first = concat!(
        "tool read\n",
        "{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\",\\\"offset\\\":0,\\\"limit\\\":10}\",\"output\":\"ok\"}\n"
    );
    let second = concat!(
        "tool read\n",
        "{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/a.rs\\\",\\\"offset\\\":0,\\\"limit\\\":10}\",\"output\":\"ok\"}\n",
        "tool read\n",
        "{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"/tmp/b.rs\\\",\\\"offset\\\":0,\\\"limit\\\":10}\",\"output\":\"ok\"}\n"
    );

    app.chat = vec![ChatMessage {
        role: ChatRole::Assistant,
        content: first.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    let phase = prepare_chat_ui_chunk_phase(&app.chat, 0, true, false);
    app.ui().apply_prepared_chat_ui_phase(phase);

    let key = explore_summary_animation_key(0, 0);
    let initial_state = app
        .chat_explore_summary_animations
        .get(&key)
        .expect("initial explore summary animation state should exist");
    assert_eq!(initial_state.previous_summary_text, "1 次读取");
    assert_eq!(initial_state.current_summary_text, "1 次读取");
    assert_eq!(initial_state.changed_at_ms, None);

    app.chat[0].content = second.to_string();
    let phase = prepare_chat_ui_chunk_phase(&app.chat, 0, true, false);
    app.ui().apply_prepared_chat_ui_phase(phase);

    let updated_state = app
        .chat_explore_summary_animations
        .get(&key)
        .expect("updated explore summary animation state should exist");
    assert_eq!(updated_state.previous_summary_text, "1 次读取");
    assert_eq!(updated_state.current_summary_text, "2 次读取");
    assert!(updated_state.changed_at_ms.is_some());
}

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn invalidate_chat_ui_state_clears_user_question_index_for_history_replacement() {
    let mut app = Fixture::default();
    app.chat = vec![message(ChatRole::User, "old question")];
    app.user_question_markers =
        desktop_session::session::active::build_user_question_markers(&app.chat);
    app.user_question_indexed_chat_len = app.chat.len();

    app.ui().invalidate_chat_ui_state();

    assert!(app.user_question_markers.is_empty());
    assert_eq!(app.user_question_indexed_chat_len, 0);
}

#[test]
fn resolve_render_cache_height_requires_matching_message_content_hash() {
    let mut app = Fixture::default();
    app.chat = vec![message(ChatRole::Assistant, "cached")];
    let content_hash = crate::message_view::parse::hash_chat_content("cached");
    app.chat_render_cache.insert(
        0,
        ChatRenderCacheEntry {
            content_hash,
            estimated_expanded_height: 240.0,
            ..ChatRenderCacheEntry::default()
        },
    );

    assert_eq!(cached_message_height(&app.chat, &app.chat_render_cache, 0), Some(240.0));

    app.chat[0].content = "changed".to_string();

    assert_eq!(cached_message_height(&app.chat, &app.chat_render_cache, 0), None);
    assert_eq!(cached_message_height(&app.chat, &app.chat_render_cache, 1), None);
}
#[test]
fn animation_helpers_detect_future_change_and_wrap_frame_without_expiring_idle_state() {
    let mut states = std::collections::HashMap::new();
    let mut frame;
    states.clear();
    assert!(!crate::stream_render::has_active_explore_summary_animation(
        &states,
        2_000,
        desktop_ui_controls::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS
    ));

    states.insert(
        1,
        ExploreSummaryAnimationState {
            previous_summary_text: "previous".to_string(),
            current_summary_text: "current".to_string(),
            changed_at_ms: Some(u64::MAX),
        },
    );
    assert!(crate::stream_render::has_active_explore_summary_animation(
        &states,
        2_000,
        desktop_ui_controls::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS
    ));

    states.insert(
        2,
        ExploreSummaryAnimationState {
            previous_summary_text: "stable".to_string(),
            current_summary_text: "stable".to_string(),
            changed_at_ms: None,
        },
    );
    frame = usize::MAX;
    crate::stream_render::advance_status_animation_frame(
        &mut frame,
        &mut states,
        2_000,
        desktop_ui_controls::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS,
    );

    assert_eq!(frame, 0);
    let stable = states.get(&2).expect("stable animation state");
    assert_eq!(stable.previous_summary_text, "stable");
    assert_eq!(stable.changed_at_ms, None);
}

#[test]
fn advance_status_animation_frame_expires_old_summary_animation() {
    let mut states = std::collections::HashMap::new();
    let mut frame = 0usize;
    states.clear();
    states.insert(
        7,
        ExploreSummaryAnimationState {
            previous_summary_text: "old".to_string(),
            current_summary_text: "new".to_string(),
            changed_at_ms: Some(0),
        },
    );

    crate::stream_render::advance_status_animation_frame(
        &mut frame,
        &mut states,
        2_000,
        desktop_ui_controls::status_animation::EXPLORE_SUMMARY_FLIP_DURATION_MS,
    );

    let state = states.get(&7).expect("animation state");
    assert_eq!(state.previous_summary_text, "new");
    assert_eq!(state.current_summary_text, "new");
    assert_eq!(state.changed_at_ms, None);
}

#[test]
fn cold_session_load_prepares_base_chunk_off_ui_thread() {
    let messages =
        (0..33).map(|index| message(ChatRole::Assistant, &format!("message {index}"))).collect();
    let prepared = desktop_session::session::loading::prepare_session_open(
        "session-1".to_string(),
        messages,
        (0..33).map(|index| Some(format!("message-{index}"))).collect(),
        session_ui_types::TokenUsage::default(),
        Vec::new(),
        Vec::new(),
        20,
        None,
        |messages| prepare_initial_session_chunk(messages, false),
    );

    let chunk = prepared.base_ui_chunk.expect("first UI chunk should be prepared");
    assert_eq!(chunk.chunk_start_idx, 0);
    assert_eq!(chunk.chunk_end_idx, 32);
    assert_eq!(chunk.render_cache.len(), 32);
    assert_eq!(prepared.messages.len(), 33);
}
