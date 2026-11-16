//! 处理聊天输入区的局部消息。
//! 本模块将编辑器操作、文件检索和工具细节限制在输入面板边界内。

use super::shared::{
    close_input_context_menu, locate_chat_message, locate_chat_message_index,
    sync_global_input_editor_if_needed,
};
use crate::app::App;
use crate::app::components::chat_panel::height_index::CHAT_MESSAGE_GAP;
use crate::app::components::chat_panel::virtualizer::ChatVirtualizer;
use crate::app::models::{ChatMessage, ChatRole};
use iced::widget::text_editor;

fn test_app() -> App {
    App::new().0
}

fn chat_message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn close_input_context_menu_clears_open_state_and_position() {
    let mut app = test_app();
    app.input_context_menu.open = true;
    app.input_context_menu.position = Some((12.0, 24.0));

    close_input_context_menu(&mut app);

    assert!(!app.input_context_menu.open);
    assert_eq!(app.input_context_menu.position, None);
}

#[test]
fn sync_global_input_editor_copies_only_without_active_session() {
    let mut app = test_app();
    app.input_editor = text_editor::Content::with_text("stale");
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("draft");

    sync_global_input_editor_if_needed(&mut app);

    assert_eq!(app.input_editor.text(), "draft");

    app.active_session_id = Some("session-1".to_string());
    app.input_editor = text_editor::Content::with_text("visible");
    app.current_session_runtime_mut().input_editor = text_editor::Content::with_text("hidden");

    sync_global_input_editor_if_needed(&mut app);

    assert_eq!(app.input_editor.text(), "visible");
}

#[test]
fn locate_chat_message_returns_none_for_missing_message_id() {
    let mut app = test_app();
    app.chat = vec![chat_message(ChatRole::Assistant, "answer")];
    app.chat_message_ids = vec![Some("msg-1".to_string())];
    app.chat_auto_scroll = true;

    let _task = locate_chat_message(&mut app, "missing");

    assert!(app.chat_auto_scroll);
}

#[test]
fn locate_chat_message_index_disables_auto_scroll_for_valid_index() {
    let mut app = test_app();
    app.chat =
        vec![chat_message(ChatRole::User, "question"), chat_message(ChatRole::Assistant, "answer")];
    app.chat_message_ids = vec![Some("msg-1".to_string()), Some("msg-2".to_string())];
    app.chat_auto_scroll = true;
    app.chat_virtualizer.set_viewport_height(200.0);
    app.sync_chat_virtualizer_compat_fields();

    let _task = locate_chat_message_index(&mut app, 1);

    assert!(!app.chat_auto_scroll);
    assert_eq!(app.chat_message_estimated_heights.len(), app.chat.len());
}

#[test]
fn locate_chat_message_index_preserves_scroll_state_for_out_of_bounds_index() {
    let mut app = test_app();
    app.chat = vec![chat_message(ChatRole::Assistant, "answer")];
    app.chat_auto_scroll = true;

    let _task = locate_chat_message_index(&mut app, 1);

    assert!(app.chat_auto_scroll);
    assert!(app.chat_message_estimated_heights.is_empty());
}

#[test]
fn locate_chat_message_index_schedules_missing_chunk_for_active_session() {
    let mut app = test_app();
    app.active_session_id = Some("session-1".to_string());
    app.chat = vec![chat_message(ChatRole::Assistant, "answer")];

    let _task = locate_chat_message_index(&mut app, 0);

    assert!(app.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
}

#[test]
fn locate_chat_message_index_does_not_reschedule_preparing_chunk() {
    let mut app = test_app();
    app.active_session_id = Some("session-1".to_string());
    app.chat = vec![chat_message(ChatRole::Assistant, "answer")];
    app.mark_chat_ui_chunks_preparing(&[0]);

    let _task = locate_chat_message_index(&mut app, 0);

    assert!(app.active_session_view_state.preparing_chat_ui_chunks.contains(&0));
    assert!(!app.active_session_view_state.dirty_preparing_chat_ui_chunks.contains(&0));
}

#[test]
fn chat_scroll_semantics_locate_message_uses_virtualizer_anchor() {
    let mut app = test_app();
    app.chat =
        (0..8).map(|idx| chat_message(ChatRole::Assistant, &format!("answer {idx}"))).collect();
    app.chat_message_ids = (0..8).map(|idx| Some(format!("msg-{idx}"))).collect();
    app.chat_auto_scroll = true;
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0; 8]);
    app.chat_virtualizer.set_viewport_height(220.0);
    app.chat_virtualizer.focus_turn_on_user(6);
    app.sync_chat_virtualizer_compat_fields();
    for idx in 0..app.chat.len() {
        app.chat_message_measured_heights.insert(idx, 100.0);
    }

    let _task = locate_chat_message_index(&mut app, 4);

    let expected_scroll_top = 4.0 * (100.0 + CHAT_MESSAGE_GAP) + 100.0 * 0.50 - 220.0 * 0.18;
    assert!(!app.chat_auto_scroll);
    assert_eq!(app.chat_virtualizer.turn_focus_user_index(), None);
    assert!((app.chat_virtualizer.scroll_top_px() - expected_scroll_top).abs() < 0.01);
    assert_eq!(app.chat_scroll_offset_y, app.chat_virtualizer.relative_y());
}
