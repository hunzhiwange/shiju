//! 验证不同消息角色、附件与处理时长在消息视图中的呈现契约。
//! 时间 fixture 区分进行中、已完成和暂停区间，防止历史耗时遗漏非模型活动。

use super::fixture::{Message, message};
use super::{
    copy_chat_message_action, message_view, open_chat_message_context_menu_action,
    processing_duration_label,
};
use crate::render_cache::ChatRenderCacheEntry;
use session_ui_types::ProcessingTiming;
use session_ui_types::{ChatMessage, ChatRole};
use std::borrow::Cow;

fn keep_message_view(element: iced::Element<'_, super::fixture::Message>) {
    std::hint::black_box(element);
}

#[test]
fn message_view_does_not_hash_full_content() {
    let mut app = super::fixture::RenderFixture::new().0;
    app.chat.push(ChatMessage {
        role: ChatRole::Assistant,
        content: "cached assistant response".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    let cache = super::build_render_cache_entry(
        &app.chat[0].content,
        &app.chat[0].content,
        super::hash_chat_content(&app.chat[0].content),
        false,
    );
    app.chat_visible_text_cache = vec![Some(app.chat[0].content.clone())];
    app.chat_copy_hash_cache = vec![cache.copy_content_hash];
    crate::performance::reset();

    keep_message_view(message_view(
        &app,
        0,
        ChatRole::Assistant,
        &app.chat[0].content,
        &[],
        None,
        None,
        &cache,
        true,
    ));

    assert_eq!(crate::performance::snapshot().full_content_hashes, 0);
}

#[test]
fn message_view_actions_carry_identity_without_message_text() {
    let copy = copy_chat_message_action("session-1", 42);
    assert!(matches!(
        copy,
        Message::CopyChatMessage { session_id, message_idx }
            if session_id == "session-1" && message_idx == 42
    ));

    let context =
        open_chat_message_context_menu_action(7, iced::Point::new(10.0, 20.0), "session-1", 42);
    assert!(matches!(
        context,
        Message::Chat(message::ChatMessage::OpenChatMessageContextMenu {
            target: 7,
            x: 10.0,
            y: 20.0,
            session_id,
            message_idx: 42,
        }) if session_id == "session-1"
    ));
}

#[test]
fn message_view_builds_completed_processing_label() {
    let app = super::fixture::RenderFixture::new().0;
    let cache = ChatRenderCacheEntry::default();
    let timing = ProcessingTiming {
        started_ms: 1_000,
        completed_ms: Some(26_000),
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    };

    let view =
        message_view(&app, 0, ChatRole::Assistant, "done", &[], Some(&timing), None, &cache, true);

    keep_message_view(view);
}

#[test]
fn message_view_hides_open_processing_label() {
    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            None,
            None
        ),
        None
    );
    let app = super::fixture::RenderFixture::new().0;
    let cache = ChatRenderCacheEntry::default();
    let timing = ProcessingTiming {
        started_ms: 1_000,
        completed_ms: None,
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    };

    assert_eq!(
        processing_duration_label(
            desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese),
            Some(&timing),
            None
        ),
        None
    );
    let view = message_view(
        &app,
        0,
        ChatRole::Assistant,
        "still running",
        &[],
        Some(&timing),
        None,
        &cache,
        true,
    );

    keep_message_view(view);
}

#[test]
fn message_view_builds_user_message_with_meta_and_branch_actions() {
    let mut app = super::fixture::RenderFixture::new().0;
    app.active_session_id = Some("session-1".to_string());
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::User,
        "please review",
        &[],
        None,
        Some(Cow::Borrowed("You · now")),
        &cache,
        true,
    );

    keep_message_view(view);
}


#[test]
fn message_view_builds_tool_role_as_assistant_like_body() {
    let app = super::fixture::RenderFixture::new().0;
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::Tool,
        "tool read\n{\"status\":\"completed\",\"input\":\"{\\\"filePath\\\":\\\"src/lib.rs\\\"}\"}",
        &[],
        None,
        Some(Cow::Borrowed("tool · done")),
        &cache,
        false,
    );

    keep_message_view(view);
}

#[test]
fn message_view_builds_streaming_assistant_from_last_chat_message() {
    let mut app = super::fixture::RenderFixture::new().0;
    app.is_requesting = true;
    app.chat.push(ChatMessage {
        role: ChatRole::Assistant,
        content: "<think>working".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    let cache = ChatRenderCacheEntry::default();

    let view = message_view(
        &app,
        0,
        ChatRole::Assistant,
        "<think>working",
        &[],
        None,
        None,
        &cache,
        false,
    );

    keep_message_view(view);
}
