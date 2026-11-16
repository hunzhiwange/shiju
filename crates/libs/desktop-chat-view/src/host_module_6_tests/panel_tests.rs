use crate::host_module_5_tests::fixture::RenderFixture as App;
use crate::virtualizer::ChatVirtualizer;
use session_ui_types::{ChatMessage, ChatRole};
fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.to_owned(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}
fn keep_element(element: iced::Element<'_, crate::panel::Message>) {
    std::hint::black_box(element);
}
#[test]
fn chat_panel_uses_virtual_window_for_render_range() {
    let (mut app, _) = App::new();
    app.chat = (0..120).map(|idx| message(ChatRole::Assistant, &format!("msg {idx}"))).collect();
    app.chat_message_estimated_heights = vec![2000.0; app.chat.len()];
    app.chat_height_index =
        crate::height_index::ChatHeightIndex::from_heights(&app.chat_message_estimated_heights);

    let virtual_heights = vec![80.0; app.chat.len()];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&virtual_heights);
    app.chat_virtualizer.set_viewport_height(180.0);
    app.chat_virtualizer.set_scroll_top_px(4200.0);

    let window = app.chat_virtualizer.window();

    assert_eq!(window, app.chat_virtualizer.window());
    assert!(window.render_start_idx > 0);
    assert!(window.render_end_idx < app.chat.len());
    assert!(window.top_spacer_h > 0.0);
    assert!(window.bottom_spacer_h > 0.0);
}
#[test]
fn chat_panel_context_menu_overlay_builds_closed_and_open_states() {
    for (open, anchor) in [(false, None), (true, Some((12.0, 24.0)))] {
        keep_element(crate::panel::context_menu_overlay(
            iced::widget::text("content"),
            iced::widget::text("menu").into(),
            open,
            anchor,
            crate::panel::Message::ScrollToBottom,
        ));
    }
}
#[test]
fn jump_button_and_question_overlay_cover_hidden_and_visible_states() {
    let i18n = desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese);
    keep_element(crate::panel::jump_to_bottom_button(true, 0.0));
    keep_element(crate::panel::user_question_nav_overlay(i18n, 0.0, &[], 0, 0));
    let messages = vec![message(ChatRole::User, "first\nquestion")];
    let markers = desktop_session::session::active::build_user_question_markers(&messages);
    keep_element(crate::panel::jump_to_bottom_button(false, 200.0));
    keep_element(crate::panel::user_question_nav_overlay(i18n, 200.0, &markers, 0, 1));
}
#[test]
fn chat_todo_overlay_builds_only_for_matching_top_right_session_with_items() {
    assert!(!crate::panel::todo_is_visible(true, Some("other"), Some("active"), 1));
    assert!(!crate::panel::todo_is_visible(false, Some("active"), Some("active"), 1));
    assert!(crate::panel::todo_is_visible(true, Some("active"), Some("active"), 1));
    keep_element(crate::panel::todo_overlay(iced::widget::text("task 1").into()));
}
