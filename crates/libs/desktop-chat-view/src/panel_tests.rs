use crate::panel::*;
use session_ui_types::{ChatMessage, ChatRole};
use desktop_session::session::active::UserQuestionMarker;
fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage { role, content: content.to_string(), model: None, think_timing: Vec::new(), processing_timing: None }
}
fn question_marker(message_idx: usize) -> UserQuestionMarker {
    UserQuestionMarker { message_idx, preview: format!("question {message_idx}") }
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
        live_message_meta_fallback(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            ChatRole::Assistant,
            true,
            true,
            "gpt"
        ),
        Some("gpt · 刚刚".to_string())
    );
    assert_eq!(
        live_message_meta_fallback(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            ChatRole::User,
            true,
            true,
            "gpt"
        ),
        Some("gpt · 刚刚".to_string())
    );
    assert_eq!(
        live_message_meta_fallback(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            ChatRole::Assistant,
            false,
            true,
            "new-model"
        ),
        None
    );
    assert_eq!(
        live_message_meta_fallback(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            ChatRole::User,
            true,
            false,
            "new-model"
        ),
        None
    );
    assert_eq!(
        live_message_meta_fallback(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            ChatRole::System,
            true,
            true,
            "gpt"
        ),
        None
    );
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
fn chat_render_window_iterates_only_render_slice() {
    let chat = vec![(); 10_000];
    let mut visits = 0;

    for (idx, _) in chat_render_window(&chat, 9_980, 10_000) {
        assert!((9_980..10_000).contains(&idx));
        visits += 1;
    }

    assert_eq!(visits, 20);
}

#[test]
fn question_marker_projection_is_bounded_to_40_and_keeps_active_edges() {
    let markers = (0..10_000).map(question_marker).collect::<Vec<_>>();
    let active_position = 5_000;
    let projected = project_user_question_marker_positions(markers.len(), Some(active_position));

    assert!(projected.len() <= 40);
    assert!(projected.contains(&0));
    assert!(projected.contains(&(markers.len() - 1)));
    assert!(projected.contains(&active_position));
    assert!(projected.contains(&(active_position - 1)));
    assert!(projected.contains(&(active_position + 1)));
}

#[test]
fn chat_panel_widget_tree_size_is_independent_of_total_history() {
    let small_history = vec![(); 100];
    let large_history = vec![(); 10_000];
    let small_rows = chat_render_window(&small_history, 80, 100).count();
    let large_rows = chat_render_window(&large_history, 9_980, 10_000).count();
    let small_markers = project_user_question_marker_positions(100, Some(90));
    let large_markers = project_user_question_marker_positions(10_000, Some(9_990));

    assert_eq!(small_rows, large_rows);
    assert!(small_markers.len() <= 40);
    assert!(large_markers.len() <= 40);
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
    assert_eq!(user_question_preview(""), "");
}

#[test]
fn user_question_preview_short_text_passes_through() {
    assert_eq!(user_question_preview("short question"), "short question");
}
