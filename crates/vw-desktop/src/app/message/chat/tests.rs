#[test]
fn tests_module_is_wired() {
    assert!(module_path!().ends_with("tests"));
}

#[test]
fn think_block_key_packs_message_and_think_indexes() {
    assert_eq!(super::think_block_key(0, 7), 7);
    assert_eq!(super::think_block_key(2, 3), (2_u64 << 32) | 3);
}

#[test]
fn fork_session_target_and_actions_keep_expected_equality() {
    assert_eq!(super::ForkSessionTarget::Local, super::ForkSessionTarget::Local);
    assert_ne!(super::ForkSessionTarget::Local, super::ForkSessionTarget::NewWorktree);

    assert_eq!(
        super::MessageIndexedSessionAction::Reset { revert_code: true },
        super::MessageIndexedSessionAction::Reset { revert_code: true }
    );
    assert_ne!(
        super::MessageIndexedSessionAction::Reset { revert_code: true },
        super::MessageIndexedSessionAction::Reset { revert_code: false }
    );
}

#[test]
fn clipboard_payload_variants_are_constructible() {
    let payloads = [
        super::ClipboardPastePayload::Text("text".to_string()),
        super::ClipboardPastePayload::AttachmentPath("/tmp/a.png".to_string()),
        super::ClipboardPastePayload::Empty,
        super::ClipboardPastePayload::Error("boom".to_string()),
    ];

    assert_eq!(payloads.len(), 4);
}

#[test]
fn arm_autoscroll_hold_enables_follow_mode_and_extends_deadline() {
    let (mut app, _task) = crate::app::App::new();
    app.chat_auto_scroll = false;
    app.chat_autoscroll_hold_until_ms = 0;

    super::arm_autoscroll_hold(&mut app);

    assert!(app.chat_auto_scroll);
    assert!(app.chat_autoscroll_hold_until_ms >= super::AUTOSCROLL_HOLD_MS);
}

#[test]
fn throttled_stream_autoscroll_respects_follow_mode_and_window() {
    let (mut app, _task) = crate::app::App::new();
    app.chat_auto_scroll = false;
    assert!(super::throttled_stream_autoscroll_task(&mut app).is_none());

    app.chat_auto_scroll = true;
    app.chat_stream_autoscroll_last_ms = super::now_ms();
    assert!(super::throttled_stream_autoscroll_task(&mut app).is_none());

    app.chat_stream_autoscroll_last_ms = 0;
    assert!(super::throttled_stream_autoscroll_task(&mut app).is_some());
}

#[test]
fn scroll_chat_to_bottom_task_snaps_virtualizer_and_syncs_compat_fields() {
    use crate::app::components::chat_panel::virtualizer::{ChatScrollMode, ChatVirtualizer};

    let (mut app, _task) = crate::app::App::new();
    app.chat_auto_scroll = false;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0, 120.0, 140.0]);
    app.chat_virtualizer.set_viewport_height(100.0);
    app.chat_virtualizer.set_scroll_top_px(20.0);

    let _task = super::scroll_chat_to_bottom_task(&mut app);

    assert!(app.chat_auto_scroll);
    assert_eq!(app.chat_virtualizer.scroll_mode(), ChatScrollMode::TailSticky);
    assert_eq!(app.chat_virtualizer.scroll_top_px(), app.chat_virtualizer.max_scroll_top_px());
    assert_eq!(app.chat_scroll_offset_y, 1.0);
    assert_eq!(app.chat_scroll_viewport_h, 100.0);
}

#[test]
fn scroll_chat_to_bottom_with_followups_snaps_virtualizer_and_syncs_compat_fields() {
    use crate::app::components::chat_panel::virtualizer::{ChatScrollMode, ChatVirtualizer};

    let (mut app, _task) = crate::app::App::new();
    app.chat_auto_scroll = false;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0, 120.0]);
    app.chat_virtualizer.set_viewport_height(50.0);
    app.chat_virtualizer.set_scroll_top_px(10.0);

    let _task = super::scroll_chat_to_bottom_with_followups(&mut app);

    assert!(app.chat_auto_scroll);
    assert_eq!(app.chat_virtualizer.scroll_mode(), ChatScrollMode::TailSticky);
    assert_eq!(app.chat_virtualizer.scroll_top_px(), app.chat_virtualizer.max_scroll_top_px());
    assert_eq!(app.chat_scroll_offset_y, 1.0);
    assert_eq!(app.chat_scroll_viewport_h, 50.0);
}

#[test]
fn measure_visible_chat_messages_task_schedules_widget_operation() {
    let task = super::measure_visible_chat_messages_task();

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn update_routes_append_text_to_context_handler() {
    let (mut app, _task) = crate::app::App::new();
    app.input_editor = iced::widget::text_editor::Content::with_text("existing");
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("existing");

    let _task = super::update(&mut app, super::ChatMessage::AppendText("added".to_string()));

    assert_eq!(app.current_session_runtime().input_editor.text(), "existing\nadded");
    assert_eq!(app.input_editor.text(), "existing\nadded");
}

#[test]
fn stream_delta_syncs_and_prunes_think_block_state() {
    let (mut app, _task) = crate::app::App::new();
    app.chat_auto_scroll = false;
    app.chat = vec![crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::Assistant,
        content: "visible <think>still working".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let _task =
        super::update(&mut app, super::ChatMessage::AgentStreamDelta(u64::MAX, String::new()));

    assert_eq!(app.chat_stream_think_msg_idx, Some(0));
    assert_eq!(app.chat_stream_think_count, 1);
    assert_eq!(app.chat_stream_think_open_idx, Some(0));

    let key = super::think_block_key(0, 0);
    app.chat_think_editors.insert(key, iced::widget::text_editor::Content::with_text("stale"));
    app.chat_think_expanded.insert(key);
    app.chat_think_collapsed.insert(key);
    app.chat[0].content = "visible answer".to_string();

    let _task =
        super::update(&mut app, super::ChatMessage::AgentStreamDelta(u64::MAX, String::new()));

    assert_eq!(app.chat_stream_think_count, 0);
    assert_eq!(app.chat_stream_think_open_idx, None);
    assert!(!app.chat_think_editors.contains_key(&key));
    assert!(!app.chat_think_expanded.contains(&key));
    assert!(!app.chat_think_collapsed.contains(&key));
}

#[test]
fn stream_delta_with_new_open_think_refreshes_autoscroll_throttle() {
    let (mut app, _task) = crate::app::App::new();
    app.chat_auto_scroll = true;
    app.chat_stream_autoscroll_last_ms = 0;
    app.chat = vec![crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::Assistant,
        content: "visible <think>still working".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let _task =
        super::update(&mut app, super::ChatMessage::AgentStreamDelta(u64::MAX, String::new()));

    assert_eq!(app.chat_stream_think_open_idx, Some(0));
    assert_ne!(app.chat_stream_autoscroll_last_ms, 0);
    assert!(app.chat_autoscroll_hold_until_ms >= super::AUTOSCROLL_HOLD_MS);
}

#[test]
fn non_stream_update_schedules_visible_chunk_preparation() {
    let (mut app, _task) = crate::app::App::new();
    app.active_session_id = Some("session-1".to_string());
    app.chat = vec![crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::Assistant,
        content: "<think>working</think>\nvisible answer".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    app.chat_virtualizer.sync_len(app.chat.len(), 100.0);
    app.chat_virtualizer.set_viewport_height(300.0);

    let _task = super::update(&mut app, super::ChatMessage::InputChanged("typed".to_string()));

    assert!(app.chat_render_cache.is_empty());
    assert!(!app.active_session_view_state.prepared_chat_ui_chunks.contains(&0));
    assert!(app.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
}
