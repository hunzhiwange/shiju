use super::{message_footer_padding, message_view, processing_duration_label};
use crate::app::models::{ChatMessage, ChatRenderCacheEntry, ChatRole};
use iced::Padding;
use std::borrow::Cow;
use vw_shared::session::ui_types::ProcessingTiming;

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
fn processing_duration_formats_seconds_minutes_and_hours() {
    assert_eq!(
        processing_duration_label(Some(&ProcessingTiming {
            started_ms: 1_000,
            completed_ms: Some(60_000),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        }), None)
        .as_deref(),
        Some("已处理 59s")
    );
    assert_eq!(
        processing_duration_label(Some(&ProcessingTiming {
            started_ms: 1_000,
            completed_ms: Some(61_000),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        }), None)
        .as_deref(),
        Some("已处理 1m 0s")
    );
    assert_eq!(
        processing_duration_label(Some(&ProcessingTiming {
            started_ms: 1_000,
            completed_ms: Some(1_518_000),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        }), None)
        .as_deref(),
        Some("已处理 25m 17s")
    );
    assert_eq!(
        processing_duration_label(Some(&ProcessingTiming {
            started_ms: 1_000,
            completed_ms: Some(3_601_000),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        }), None)
        .as_deref(),
        Some("已处理 1h 0m 0s")
    );
    assert_eq!(
        processing_duration_label(Some(&ProcessingTiming {
            started_ms: 1_000,
            completed_ms: Some(3_724_000),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        }), None)
        .as_deref(),
        Some("已处理 1h 2m 3s")
    );
    assert_eq!(
        processing_duration_label(Some(&ProcessingTiming {
            started_ms: 2_000,
            completed_ms: Some(1_000),
            processing_elapsed_ms: None,
            llm_elapsed_ms: None,
        }), None)
        .as_deref(),
        Some("已处理 0s")
    );
}

#[test]
fn message_view_builds_completed_processing_label() {
    let app = crate::app::App::new().0;
    let cache = ChatRenderCacheEntry::default();
    let timing = ProcessingTiming {
        started_ms: 1_000,
        completed_ms: Some(26_000),
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    };

    let view = message_view(
        &app,
        0,
        ChatRole::Assistant,
        "done",
        &[],
        Some(&timing),
        None,
        &cache,
        true,
    );

    keep_message_view(view);
}

#[test]
fn message_view_hides_open_processing_label() {
    assert_eq!(processing_duration_label(None, None), None);
    let app = crate::app::App::new().0;
    let cache = ChatRenderCacheEntry::default();
    let timing = ProcessingTiming {
        started_ms: 1_000,
        completed_ms: None,
        processing_elapsed_ms: None,
        llm_elapsed_ms: None,
    };

    assert_eq!(processing_duration_label(Some(&timing), None), None);
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
fn processing_label_shows_zero_for_active_open_cycle() {
    let timing = ProcessingTiming {
        started_ms: 1_000,
        completed_ms: None,
        processing_elapsed_ms: Some(0),
        llm_elapsed_ms: Some(0),
    };

    assert_eq!(
        processing_duration_label(Some(&timing), Some(0)).as_deref(),
        Some("已处理 0s")
    );
}

#[test]
fn historical_processing_label_includes_pauses_outside_llm_activity() {
    let timing = ProcessingTiming {
        started_ms: 1_000,
        completed_ms: Some(61_000),
        processing_elapsed_ms: Some(60_000),
        llm_elapsed_ms: Some(2_750),
    };

    assert_eq!(
        processing_duration_label(Some(&timing), None).as_deref(),
        Some("已处理 1m 0s")
    );
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
        None,
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
        None,
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
