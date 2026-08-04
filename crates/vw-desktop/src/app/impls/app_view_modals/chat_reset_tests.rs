use super::*;
use crate::app::models::{ChatMessage, ChatRole};

fn test_app() -> App {
    App::new().0
}

fn root() -> iced::Element<'static, Message> {
    iced::widget::container(iced::widget::text("root")).into()
}

fn user_message() -> ChatMessage {
    ChatMessage { role: ChatRole::User, content: "hello".to_string(), model: None, think_timing: Vec::new(), processing_timing: None }
}

#[test]
fn action_text_styles_use_white_foreground() {
    assert_eq!(action_button_title_text_style(&iced::Theme::Dark).color, Some(iced::Color::WHITE));
    assert_eq!(
        action_button_detail_text_style(&iced::Theme::Dark).color,
        Some(iced::Color::WHITE.scale_alpha(0.82))
    );
}

#[test]
fn with_chat_reset_dialog_returns_root_when_no_index_or_missing_message() {
    let mut app = test_app();
    let _element: iced::Element<'_, Message> = with_chat_reset_dialog(&app, root());

    app.chat_reset_menu_idx = Some(1);
    app.chat.push(user_message());
    let _element: iced::Element<'_, Message> = with_chat_reset_dialog(&app, root());
}

#[test]
fn with_chat_reset_dialog_builds_modal_for_existing_message() {
    let mut app = test_app();
    app.chat.push(user_message());
    app.chat_reset_menu_idx = Some(0);

    let _element: iced::Element<'_, Message> = with_chat_reset_dialog(&app, root());
}
