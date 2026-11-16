//! 宿主契约：滚动/测量 widget Task 与兼容字段、流式事件到缓存和保存任务的映射，
//! 以及延迟消息的会话/代际校验；纯载荷和跟随期限规则已迁入 desktop-session。

//! 验证聊天模块键值编码、会话操作类型和自动滚动节流行为。
//!
//! 自动滚动测试使用明确时钟窗口，确保跟随模式、节流期限和兼容字段同步更新。

#[test]
fn tests_module_is_wired() {
    assert!(module_path!().ends_with("tests"));
}

#[test]
fn throttled_stream_autoscroll_respects_follow_mode_and_window() {
    let (mut app, _task) = crate::app::App::new();
    app.chat_view.chat_auto_scroll = false;
    assert!(super::throttled_stream_autoscroll_task(&mut app).is_none());

    app.chat_view.chat_auto_scroll = true;
    app.chat_interaction.chat_stream_autoscroll_last_ms = super::now_ms();
    assert!(super::throttled_stream_autoscroll_task(&mut app).is_none());

    app.chat_interaction.chat_stream_autoscroll_last_ms = 0;
    assert!(super::throttled_stream_autoscroll_task(&mut app).is_some());
}

#[test]
fn scroll_chat_to_bottom_task_snaps_virtualizer_and_syncs_compat_fields() {
    use crate::app::components::chat_panel::virtualizer::{ChatScrollMode, ChatVirtualizer};

    let (mut app, _task) = crate::app::App::new();
    app.chat_view.chat_auto_scroll = false;
    app.chat_view.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0, 120.0, 140.0]);
    app.chat_view.chat_virtualizer.set_viewport_height(100.0);
    app.chat_view.chat_virtualizer.set_scroll_top_px(20.0);

    let _task = super::scroll_chat_to_bottom_task(&mut app);

    assert!(app.chat_view.chat_auto_scroll);
    assert_eq!(app.chat_view.chat_virtualizer.scroll_mode(), ChatScrollMode::TailSticky);
    assert_eq!(app.chat_view.chat_virtualizer.scroll_top_px(), app.chat_view.chat_virtualizer.max_scroll_top_px());
    assert_eq!(app.chat_view.chat_scroll_offset_y, 1.0);
    assert_eq!(app.chat_view.chat_scroll_viewport_h, 100.0);
}

#[test]
fn scroll_chat_to_bottom_with_followups_snaps_virtualizer_and_syncs_compat_fields() {
    use crate::app::components::chat_panel::virtualizer::{ChatScrollMode, ChatVirtualizer};

    let (mut app, _task) = crate::app::App::new();
    app.chat_view.chat_auto_scroll = false;
    app.chat_view.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0, 120.0]);
    app.chat_view.chat_virtualizer.set_viewport_height(50.0);
    app.chat_view.chat_virtualizer.set_scroll_top_px(10.0);

    let _task = super::scroll_chat_to_bottom_with_followups(&mut app);

    assert!(app.chat_view.chat_auto_scroll);
    assert_eq!(app.chat_view.chat_virtualizer.scroll_mode(), ChatScrollMode::TailSticky);
    assert_eq!(app.chat_view.chat_virtualizer.scroll_top_px(), app.chat_view.chat_virtualizer.max_scroll_top_px());
    assert_eq!(app.chat_view.chat_scroll_offset_y, 1.0);
    assert_eq!(app.chat_view.chat_scroll_viewport_h, 50.0);
}

#[test]
fn turn_top_scroll_task_focuses_requested_user_and_requests_measurement() {
    use crate::app::components::chat_panel::virtualizer::{
        CHAT_TURN_TOP_INSET_PX, ChatScrollAnchor, ChatVirtualizer,
    };

    let (mut app, _task) = crate::app::App::new();
    app.chat_view.chat_auto_scroll = true;
    app.chat_view.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[120.0, 100.0, 0.0]);
    app.chat_view.chat_virtualizer.set_viewport_height(300.0);

    let task = super::scroll_chat_to_turn_top_task(&mut app, 1);

    assert!(!app.chat_view.chat_auto_scroll);
    assert_eq!(app.chat_view.chat_virtualizer.turn_focus_user_index(), Some(1));
    assert_eq!(
        app.chat_view.chat_virtualizer.anchor(),
        Some(ChatScrollAnchor { index: 1, offset_px: -CHAT_TURN_TOP_INSET_PX })
    );
    assert_eq!(task.units(), 2);
}

#[test]
fn measure_visible_chat_messages_task_schedules_widget_operation() {
    let task = super::measure_visible_chat_messages_task();

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}

#[test]
fn update_routes_append_text_to_context_handler() {
    let (mut app, _task) = crate::app::App::new();
    app.session_state.input_editor = iced::widget::text_editor::Content::with_text("existing");
    app.current_session_runtime_mut().input_editor =
        iced::widget::text_editor::Content::with_text("existing");

    let _task = super::update(&mut app, super::ChatMessage::AppendText("added".to_string()));

    assert_eq!(app.current_session_runtime().input_editor.text(), "existing\nadded");
    assert_eq!(app.session_state.input_editor.text(), "existing\nadded");
}

#[test]
fn stream_delta_syncs_and_prunes_think_block_state() {
    let (mut app, _task) = crate::app::App::new();
    app.session_state.active_session_id = Some("thinking-session".into());
    app.chat_view.chat_auto_scroll = false;
    app.chat_view.chat = vec![crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::Assistant,
        content: "visible <think>still working".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let content = app.chat_view.chat[0].content.clone();
    let tail = &mut app.get_session_runtime_mut("thinking-session").stream_tail_state;
    tail.clear_tail();
    tail.begin_message(0);
    tail.apply_delta(&content);
    let _task = super::update(
        &mut app,
        super::ChatMessage::AgentStreamDeltaFlush("thinking-session".into()),
    );

    assert_eq!(app.chat_interaction.chat_stream_think_msg_idx, Some(0));
    assert_eq!(app.chat_interaction.chat_stream_think_count, 1);
    assert_eq!(app.chat_interaction.chat_stream_think_open_idx, Some(0));

    let key = super::think_block_key(0, 0);
    app.chat_view.chat_think_editors.insert(key, iced::widget::text_editor::Content::with_text("stale"));
    app.chat_view.chat_think_expanded.insert(key);
    app.chat_view.chat_think_collapsed.insert(key);
    app.chat_view.chat[0].content = "visible answer".to_string();

    let content = app.chat_view.chat[0].content.clone();
    let tail = &mut app.get_session_runtime_mut("thinking-session").stream_tail_state;
    tail.clear_tail();
    tail.begin_message(0);
    tail.apply_delta(&content);
    let _task = super::update(
        &mut app,
        super::ChatMessage::AgentStreamDeltaFlush("thinking-session".into()),
    );

    assert_eq!(app.chat_interaction.chat_stream_think_count, 0);
    assert_eq!(app.chat_interaction.chat_stream_think_open_idx, None);
    assert!(!app.chat_view.chat_think_editors.contains_key(&key));
    assert!(!app.chat_view.chat_think_expanded.contains(&key));
    assert!(!app.chat_view.chat_think_collapsed.contains(&key));
}

#[test]
fn stream_flush_updates_tail_and_schedules_save_without_chunk_preparation() {
    use crate::app::components::chat_panel::virtualizer::ChatVirtualizer;
    use crate::app::models::{ChatMessage as ModelChatMessage, ChatRole};

    let (mut app, _task) = crate::app::App::new();
    let session_id = "active".to_string();
    app.session_state.active_session_id = Some(session_id.clone());
    app.chat_view.chat = (0..39)
        .map(|idx| ModelChatMessage {
            role: ChatRole::User,
            content: format!("history {idx}"),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        })
        .chain(std::iter::once(ModelChatMessage {
            role: ChatRole::Assistant,
            content: String::new(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        }))
        .collect();
    app.chat_view.chat_message_ids = vec![None; app.chat_view.chat.len()];
    app.chat_view.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&vec![100.0; app.chat_view.chat.len()]);
    app.chat_view.chat_virtualizer.set_viewport_height(240.0);
    app.chat_view.chat_virtualizer.snap_to_tail();
    app.chat_view.chat_auto_scroll = false;
    app.sync_chat_virtualizer_compat_fields();
    let runtime = app.get_session_runtime_mut(&session_id);
    runtime.pending_stream_delta = "one-second tail".to_string();
    runtime.stream_delta_flush_scheduled = true;

    let task = super::update(&mut app, super::ChatMessage::AgentStreamDeltaFlush(session_id));

    assert_eq!(app.chat_view.chat.last().unwrap().content, "one-second tail");
    assert_eq!(task.units(), 1, "stream projection is synchronous; only save is scheduled");
    let runtime = app.get_session_runtime_mut("active");
    assert!(runtime.stream_save.flush_scheduled);
    assert_eq!(runtime.stream_save.dirty_records.len(), 1);
    assert_eq!(runtime.stream_save.dirty_records[0].message.content, "one-second tail");
    assert!(app.chat_view.active_session_view_state.preparing_chat_ui_chunks.is_empty());
}

#[test]
fn stream_delta_with_new_open_think_refreshes_autoscroll_throttle() {
    let (mut app, _task) = crate::app::App::new();
    app.session_state.active_session_id = Some("thinking-session".into());
    app.chat_view.chat_auto_scroll = true;
    app.chat_interaction.chat_stream_autoscroll_last_ms = 0;
    app.chat_view.chat = vec![crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::Assistant,
        content: "visible <think>still working".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];

    let content = app.chat_view.chat[0].content.clone();
    let tail = &mut app.get_session_runtime_mut("thinking-session").stream_tail_state;
    tail.clear_tail();
    tail.begin_message(0);
    tail.apply_delta(&content);
    let _task = super::update(
        &mut app,
        super::ChatMessage::AgentStreamDeltaFlush("thinking-session".into()),
    );

    assert_eq!(app.chat_interaction.chat_stream_think_open_idx, Some(0));
    assert_ne!(app.chat_interaction.chat_stream_autoscroll_last_ms, 0);
    assert!(app.chat_interaction.chat_autoscroll_hold_until_ms >= super::AUTOSCROLL_HOLD_MS);
}

#[test]
fn composer_update_skips_chunks_but_transcript_event_prepares_visible_window() {
    let (mut app, _task) = crate::app::App::new();
    app.session_state.active_session_id = Some("session-1".to_string());
    app.chat_view.chat = vec![crate::app::models::ChatMessage {
        role: crate::app::models::ChatRole::Assistant,
        content: "<think>working</think>\nvisible answer".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    app.chat_view.chat_virtualizer.sync_len(app.chat_view.chat.len(), 100.0);
    app.chat_view.chat_virtualizer.set_viewport_height(300.0);

    let _task = super::update(&mut app, super::ChatMessage::InputChanged("typed".to_string()));

    assert!(app.chat_view.chat_render_cache.is_empty());
    assert!(!app.chat_view.active_session_view_state.prepared_chat_ui_chunks.contains(&0));
    assert_eq!(app.session_state.input_text, "typed");
    assert!(app.chat_view.active_session_view_state.preparing_chat_ui_chunks.is_empty());
    let _task = super::update(&mut app, super::ChatMessage::ThinkHover(0, 0));
    assert!(app.chat_view.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
}

#[test]
fn delayed_scroll_rejects_old_session_request_and_manual_scroll() {
    use super::ChatMessage;
    let (mut app, _) = crate::app::App::new();
    app.session_state.active_session_id = Some("original".into());
    let _ = super::scroll_chat_to_bottom_with_followups(&mut app);
    let generation = app.chat_interaction.chat_autoscroll_generation;
    let callback = ChatMessage::FollowupScrollToBottom {
        session_id: app.session_state.active_session_id.clone(),
        generation,
    };
    let task = super::input::update(&mut app, callback.clone());
    assert!(iced_runtime::task::into_stream(task).is_some());
    assert_eq!(app.chat_interaction.chat_autoscroll_generation, generation);

    app.session_state.active_session_id = Some("other".into());
    assert!(
        iced_runtime::task::into_stream(super::input::update(&mut app, callback.clone())).is_none()
    );
    app.session_state.active_session_id = Some("original".into());
    super::input::apply_chat_scroll_changed(&mut app, 0.2, 0.0, 300.0);
    assert!(!app.chat_view.chat_auto_scroll);
    assert!(
        iced_runtime::task::into_stream(super::input::update(&mut app, callback.clone())).is_none()
    );
    // Resuming follow mode must not revive the earlier callbacks.
    app.chat_view.chat_auto_scroll = true;
    assert!(
        iced_runtime::task::into_stream(super::input::update(&mut app, callback.clone())).is_none()
    );
    let _ = super::scroll_chat_to_bottom_with_followups(&mut app);
    assert!(iced_runtime::task::into_stream(super::input::update(&mut app, callback)).is_none());
}
