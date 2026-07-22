use super::{message_footer_padding, message_view};
use crate::app::models::{ChatMessage, ChatRenderCacheEntry, ChatRole};
use iced::Padding;
use std::borrow::Cow;

fn keep_message_view(element: iced::Element<'_, crate::app::Message>) {
    std::hint::black_box(element);
}

#[test]
fn user_message_footer_padding_has_right_offset() {
    let padding = message_footer_padding(true);
    assert_eq!(padding, Padding { top: 0.0, right: 6.0, bottom: 0.0, left: 6.0 });
}

#[test]
fn non_user_message_footer_padding_is_zero() {
    let padding = message_footer_padding(false);
    assert_eq!(padding, Padding::ZERO);
}

#[test]
fn message_view_builds_user_message_with_meta_and_branch_actions() {
    let mut app = crate::app::App::new().0;
    app.active_session_id = Some("session-1".to_string());
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::User,
        "please review",
        &[],
        Some(Cow::Borrowed("You · now")),
        &cache,
        true,
    );

    keep_message_view(view);
}

#[test]
fn message_view_builds_attachment_card_for_user_message() {
    let app = crate::app::App::new().0;
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::User,
        "please review\n\n[DOCUMENT:/tmp/spec.pdf]",
        &[],
        None,
        &cache,
        true,
    );

    keep_message_view(view);
}

#[test]
fn message_view_builds_tool_role_as_assistant_like_body() {
    let app = crate::app::App::new().0;
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::Tool,
        "tool read\n{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"src/lib.rs\\\"}\"}",
        &[],
        Some(Cow::Borrowed("tool · done")),
        &cache,
        false,
    );

    keep_message_view(view);
}

#[test]
fn message_view_builds_streaming_assistant_from_last_chat_message() {
    let mut app = crate::app::App::new().0;
    app.current_session_runtime_mut().is_requesting = true;
    app.chat.push(ChatMessage {
        role: ChatRole::Assistant,
        content: "<think>working".to_string(),
        think_timing: Vec::new(),
    });
    let cache = ChatRenderCacheEntry::default();

    let view =
        message_view(&app, 0, ChatRole::Assistant, "<think>working", &[], None, &cache, false);

    keep_message_view(view);
}
