use super::{
    active_user_question_idx, chat_message_column, chat_panel_context_menu_overlay,
    chat_panel_virtual_window, chat_todo_overlay, compute_visible_message_window_for_chat,
    is_chat_message_idx_visible, jump_to_bottom_button, live_message_meta_fallback,
    rough_message_heights, user_question_indices, user_question_nav_overlay, user_question_preview,
    view,
};
use crate::app::components::chat_panel::virtualizer::ChatVirtualizer;
use crate::app::models::{ChatMessage, ChatRole};
use crate::app::{App, Message, TodoPanelPlacement};
use vw_shared::todo::Todo;

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

fn todo(id: &str) -> Todo {
    Todo {
        id: id.to_string(),
        status: "pending".to_string(),
        content: format!("task {id}"),
        priority: "medium".to_string(),
    }
}

#[test]
fn chat_message_column_fills_available_width() {
    let element: iced::Element<'_, Message> = chat_message_column().into();

    assert_eq!(element.as_widget().size().width, iced::Length::Fill);
}

#[test]
fn chat_panel_uses_virtual_window_for_render_range() {
    let (mut app, _) = App::new();
    app.chat = (0..120).map(|idx| message(ChatRole::Assistant, &format!("msg {idx}"))).collect();
    app.chat_message_estimated_heights = vec![2000.0; app.chat.len()];
    app.chat_height_index =
        super::height_index::ChatHeightIndex::from_heights(&app.chat_message_estimated_heights);

    let virtual_heights = vec![80.0; app.chat.len()];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&virtual_heights);
    app.chat_virtualizer.set_viewport_height(180.0);
    app.chat_virtualizer.set_scroll_top_px(4200.0);

    let window = chat_panel_virtual_window(&app);

    assert_eq!(window, app.chat_virtualizer.window());
    assert!(window.render_start_idx > 0);
    assert!(window.render_end_idx < app.chat.len());
    assert!(window.top_spacer_h > 0.0);
    assert!(window.bottom_spacer_h > 0.0);
}

#[test]
fn chat_panel_virtualization_has_single_state_owner() {
    let sources = [
        ("chat_panel/mod.rs", include_str!("mod.rs")),
        ("session/chat_ui.rs", include_str!("../../session/chat_ui.rs")),
        ("message/chat/mod.rs", include_str!("../../message/chat/mod.rs")),
        ("message/chat/input.rs", include_str!("../../message/chat/input.rs")),
        ("message/chat/input/shared.rs", include_str!("../../message/chat/input/shared.rs")),
        (
            "message/chat/stream/stream_events.rs",
            include_str!("../../message/chat/stream/stream_events.rs"),
        ),
    ];

    for (path, source) in sources {
        assert!(
            !source.contains("chat_height_index.compute_window("),
            "{path} must get chat windows from ChatVirtualizer"
        );
        assert!(
            !source.contains("set_viewport_height(self.chat_scroll_viewport_h)")
                && !source.contains("set_relative_y(self.chat_scroll_offset_y)"),
            "{path} must not let legacy scroll fields drive ChatVirtualizer"
        );
        assert!(
            !source.contains("app.chat_scroll_offset_y = app.chat_virtualizer.relative_y()")
                && !source.contains(
                    "app.chat_scroll_viewport_h = app.chat_virtualizer.viewport_height_px()"
                ),
            "{path} must mirror ChatVirtualizer through the shared compat sync"
        );
        assert!(
            !source.contains("app.chat_height_index.update_height(")
                && !source.contains("app.chat_message_estimated_heights.get_mut("),
            "{path} must update measured heights through ChatVirtualizer"
        );
    }
}

#[test]
fn visible_message_window_clamps_to_available_heights() {
    let heights = vec![40.0, 50.0, 60.0];

    let (start, end) = compute_visible_message_window_for_chat(10, &heights, 0.0, 70.0);

    assert_eq!(start, 0);
    assert!(end <= heights.len());
    assert!(end > start);
}

#[test]
fn visible_message_window_returns_empty_without_messages_or_heights() {
    assert_eq!(compute_visible_message_window_for_chat(0, &[40.0], 0.0, 100.0), (0, 0));
    assert_eq!(compute_visible_message_window_for_chat(2, &[], 0.0, 100.0), (0, 0));
}

#[test]
fn visible_message_idx_requires_positive_non_empty_window() {
    assert!(is_chat_message_idx_visible(1, 0, 2, 200.0));
    assert!(!is_chat_message_idx_visible(2, 0, 2, 200.0));
    assert!(!is_chat_message_idx_visible(1, 0, 2, 0.0));
    assert!(!is_chat_message_idx_visible(1, 2, 2, 200.0));
}

#[test]
fn user_question_indices_only_returns_user_roles() {
    let chat = vec![
        message(ChatRole::System, "system"),
        message(ChatRole::User, "first"),
        message(ChatRole::Assistant, "answer"),
        message(ChatRole::User, "second"),
    ];

    assert_eq!(user_question_indices(&chat), vec![1, 3]);
}

#[test]
fn rough_message_heights_grow_for_rich_messages() {
    let chat = vec![
        message(ChatRole::User, "short"),
        message(ChatRole::Assistant, "<think>x</think>\ntool bash\n{}\nanswer"),
    ];

    let heights = rough_message_heights(&chat);

    assert_eq!(heights.len(), 2);
    assert!(heights[1] > heights[0]);
}

#[test]
fn live_message_meta_fallback_only_applies_to_requesting_tail_chat_roles() {
    assert_eq!(
        live_message_meta_fallback(ChatRole::Assistant, true, true, "gpt"),
        Some("gpt · 刚刚".to_string())
    );
    assert_eq!(
        live_message_meta_fallback(ChatRole::User, true, true, "gpt"),
        Some("gpt · 刚刚".to_string())
    );
    assert_eq!(live_message_meta_fallback(ChatRole::Assistant, false, true, "new-model"), None);
    assert_eq!(live_message_meta_fallback(ChatRole::User, true, false, "new-model"), None);
    assert_eq!(live_message_meta_fallback(ChatRole::System, true, true, "gpt"), None);
}

#[test]
fn active_user_question_returns_first_visible_question() {
    let indices = vec![1, 3, 5];
    assert_eq!(active_user_question_idx(&indices, 0, 4, 200.0), Some(1));
}

#[test]
fn active_user_question_returns_none_when_outside_range() {
    let indices = vec![5, 7];
    assert_eq!(active_user_question_idx(&indices, 0, 4, 200.0), None);
}

#[test]
fn active_user_question_returns_none_with_empty_indices() {
    assert_eq!(active_user_question_idx(&[], 0, 4, 200.0), None);
}

#[test]
fn active_user_question_returns_first_when_multiple_visible() {
    let indices = vec![2, 5, 8];
    assert_eq!(active_user_question_idx(&indices, 2, 9, 200.0), Some(2));
}

#[test]
fn active_user_question_returns_none_with_zero_viewport() {
    let indices = vec![1, 3];
    assert_eq!(active_user_question_idx(&indices, 0, 4, 0.0), None);
}

#[test]
fn user_question_preview_truncates_long_content() {
    let long = "a".repeat(200);
    assert_eq!(user_question_preview(&long).chars().count(), 101);
}

#[test]
fn user_question_preview_flattens_newlines() {
    let result = user_question_preview("hello\nworld\nfoo");
    assert!(!result.contains('\n'));
    assert!(result.contains(' '));
}

#[test]
fn user_question_preview_returns_placeholder_for_empty() {
    assert_eq!(user_question_preview(""), "空提问");
}

#[test]
fn user_question_preview_short_text_passes_through() {
    assert_eq!(user_question_preview("short question"), "short question");
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn chat_panel_view_builds_empty_and_loading_session_states() {
    let (mut app, _) = App::new();
    keep_element(view(&app));

    app.active_session_id = Some("loading".to_string());
    app.active_session_view_state.ui_preparing = true;
    app.active_session_view_state.base_ready = false;
    keep_element(view(&app));
}

#[test]
fn chat_panel_view_builds_started_message_window() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat = vec![message(ChatRole::User, "question"), message(ChatRole::Assistant, "answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0, 80.0]);
    app.chat_virtualizer.set_viewport_height(200.0);

    keep_element(view(&app));
}

#[test]
fn chat_panel_context_menu_overlay_builds_closed_and_open_states() {
    let (mut app, _) = App::new();
    keep_element(chat_panel_context_menu_overlay(&app, iced::widget::text("content")));

    app.chat_context_menu_target =
        Some(crate::app::components::chat_panel::utils::chat_context_target_key(0, None));
    app.chat_context_menu_pos = Some((12.0, 24.0));
    keep_element(chat_panel_context_menu_overlay(&app, iced::widget::text("content")));
}

#[test]
fn chat_panel_context_menu_has_a_single_overlay_owner() {
    assert!(include_str!("mod.rs").contains("PointBelowOverlay::new(content, menu)"));
    for source in [
        include_str!("message_view.rs"),
        include_str!("tools/advanced_view.rs"),
        include_str!("tools/apply_patch_view.rs"),
        include_str!("tools/files_view/common_view.rs"),
        include_str!("tools/files_view/write_view.rs"),
        include_str!("tools/text_view.rs"),
        include_str!("tools/todo_views.rs"),
        include_str!("tools/web_view.rs"),
    ] {
        assert!(!source.contains("PointBelowOverlay"));
    }
}

#[test]
fn jump_button_and_question_overlay_cover_hidden_and_visible_states() {
    let (mut app, _) = App::new();
    keep_element(jump_to_bottom_button(&app));
    keep_element(user_question_nav_overlay(&app, &[], 0, 0));

    app.chat_auto_scroll = false;
    app.chat_scroll_viewport_h = 200.0;
    app.chat = vec![message(ChatRole::User, "first\nquestion")];
    keep_element(jump_to_bottom_button(&app));
    keep_element(user_question_nav_overlay(&app, &[0], 0, 1));
}

#[test]
fn chat_todo_overlay_builds_only_for_matching_top_right_session_with_items() {
    let (mut app, _) = App::new();
    app.active_session_id = Some("active".to_string());
    app.chat_todo_session_id = Some("other".to_string());
    app.chat_todo_items = vec![todo("1")];

    keep_element(chat_todo_overlay(&app));

    app.chat_todo_session_id = Some("active".to_string());
    app.chat_todo_placement = TodoPanelPlacement::InputBottom;
    keep_element(chat_todo_overlay(&app));

    app.chat_todo_placement = TodoPanelPlacement::ChatTopRight;
    keep_element(chat_todo_overlay(&app));
}
