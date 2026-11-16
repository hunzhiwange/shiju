use super::fixture::{Message, RenderFixture};
use super::think_block::think_block_view;
use iced::widget::text_editor;
use session_ui_types::{ChatMessage, ChatRole, ThinkTiming};
fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn think_block_view_builds_collapsed_and_empty_expanded_states() {
    let (mut app, _) = RenderFixture::new();

    keep_element(think_block_view(&app, 1, 2, "collapsed".to_string(), false, None));

    let key = (1_u64 << 32) | 2;
    app.chat_think_expanded.insert(key);
    keep_element(think_block_view(&app, 1, 2, " \n\r ".to_string(), false, None));
}

#[test]
fn think_block_view_uses_editor_for_completed_short_content() {
    let (mut app, _) = RenderFixture::new();
    let key = (3_u64 << 32) | 4;
    app.chat_think_expanded.insert(key);
    app.chat_think_editors.insert(key, text_editor::Content::with_text("reasoning"));
    let done = ThinkTiming { start_ms: 1, end_ms: Some(2_001), last_update_ms: 2_001 };

    keep_element(think_block_view(&app, 3, 4, "reasoning".to_string(), false, Some(&done)));
}

#[test]
fn think_block_view_uses_text_fallback_for_completed_short_content_without_editor() {
    let (mut app, _) = RenderFixture::new();
    let key = (4_u64 << 32) | 5;
    app.chat_think_expanded.insert(key);
    let done = ThinkTiming { start_ms: 1_000, end_ms: Some(3_000), last_update_ms: 3_000 };

    keep_element(think_block_view(&app, 4, 5, "short reasoning".to_string(), false, Some(&done)));
}

#[test]
fn think_block_view_uses_segmented_body_for_long_completed_content() {
    let (mut app, _) = RenderFixture::new();
    let key = (5_u64 << 32) | 6;
    app.chat_think_expanded.insert(key);
    app.chat_think_hovered_idx = Some(key);
    let done = ThinkTiming { start_ms: 0, end_ms: Some(5_000), last_update_ms: 5_000 };

    keep_element(think_block_view(&app, 5, 6, "long reasoning\n".repeat(110), false, Some(&done)));
}

#[test]
fn think_block_view_builds_running_body() {
    let (mut app, _) = RenderFixture::new();
    app.chat.push(ChatMessage {
        role: ChatRole::Assistant,
        content: "answer".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    app.chat_think_expanded.insert(0);
    let running = ThinkTiming { start_ms: 1, end_ms: None, last_update_ms: 1 };

    keep_element(think_block_view(
        &app,
        0,
        0,
        "streaming reasoning".to_string(),
        true,
        Some(&running),
    ));
}

#[test]
fn think_block_running_header_builds_chinese_across_animation_frames() {
    let (mut app, _) = RenderFixture::new();
    app.chat.push(ChatMessage {
        role: ChatRole::Assistant,
        content: "<think>分析中文路径</think>".to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    });
    let running = ThinkTiming { start_ms: 1, end_ms: None, last_update_ms: 1 };

    for frame in 0..24 {
        app.status_animation_frame = frame;
        keep_element(think_block_view(
            &app,
            0,
            0,
            "正在分析 含中文/很长的工具摘要.rs".to_string(),
            true,
            Some(&running),
        ));
    }
}
