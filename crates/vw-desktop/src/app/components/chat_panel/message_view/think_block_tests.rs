use super::think_block::{
    should_render_think_block, think_block_default_expanded, think_block_is_running,
    think_block_resolved_expanded, think_block_view,
};
use crate::app::models::{ChatMessage, ChatRole, ThinkTiming};
use crate::app::{App, Message};
use iced::widget::text_editor;
use std::collections::HashSet;

#[test]
fn think_block_is_running_uses_open_or_missing_end() {
    let running = ThinkTiming { start_ms: 1, end_ms: None, last_update_ms: 2 };
    let done = ThinkTiming { start_ms: 1, end_ms: Some(2), last_update_ms: 2 };

    assert!(think_block_is_running(false, Some(&running)));
    assert!(think_block_is_running(true, Some(&done)));
    assert!(!think_block_is_running(false, Some(&done)));
}

#[test]
fn closed_think_block_without_timing_is_not_running() {
    assert!(!think_block_is_running(false, None));
}

#[test]
fn manual_expand_and_collapse_override_default() {
    let expanded = HashSet::from([9]);
    let collapsed = HashSet::from([10]);

    assert!(think_block_resolved_expanded(false, 9, &expanded, &collapsed));
    assert!(!think_block_resolved_expanded(true, 10, &expanded, &collapsed));
}

#[test]
fn completed_hidden_when_reasoning_summary_disabled() {
    let done = ThinkTiming { start_ms: 1, end_ms: Some(2), last_update_ms: 2 };

    assert!(!think_block_default_expanded(false, false, Some(&done)));
    assert!(!should_render_think_block(false, 1, false, Some(&done)));
}

#[test]
fn default_expanded_stays_collapsed_without_timing() {
    assert!(!think_block_default_expanded(false, false, None));
    assert!(!think_block_default_expanded(true, true, None));
}

#[test]
fn running_think_block_stays_collapsed_until_manual_expand() {
    let running = ThinkTiming { start_ms: 1, end_ms: None, last_update_ms: 2 };

    assert!(!think_block_default_expanded(true, false, Some(&running)));
    assert!(!think_block_default_expanded(true, true, Some(&running)));
    assert!(!think_block_resolved_expanded(false, 7, &HashSet::new(), &HashSet::new()));
}

#[test]
fn render_decision_shows_running_without_summary() {
    let running = ThinkTiming { start_ms: 10, end_ms: None, last_update_ms: 10 };

    assert!(should_render_think_block(false, 3, false, Some(&running)));
    assert!(should_render_think_block(false, 3, true, None));
}

#[test]
fn render_decision_hides_finished_without_summary_and_shows_with_summary() {
    let done = ThinkTiming { start_ms: 10, end_ms: Some(40), last_update_ms: 40 };

    assert!(!should_render_think_block(false, 3, false, Some(&done)));
    assert!(should_render_think_block(true, 3, false, Some(&done)));
}

#[test]
fn collapsed_manual_state_wins_even_when_expanded_set_contains_key() {
    let expanded = HashSet::from([5]);
    let collapsed = HashSet::from([5]);

    assert!(!think_block_resolved_expanded(true, 5, &expanded, &collapsed));
}

#[test]
fn resolved_expanded_uses_default_when_key_has_no_manual_state() {
    let expanded = HashSet::from([8]);
    let collapsed = HashSet::from([9]);

    assert!(think_block_resolved_expanded(true, 7, &expanded, &collapsed));
    assert!(!think_block_resolved_expanded(false, 7, &expanded, &collapsed));
}

#[test]
fn render_decision_without_timing_follows_open_or_summary_setting() {
    assert!(!should_render_think_block(false, 0, false, None));
    assert!(should_render_think_block(false, 0, true, None));
    assert!(should_render_think_block(true, 0, false, None));
}

#[test]
fn render_decision_does_not_depend_on_total_think_block_count() {
    let done = ThinkTiming { start_ms: 10, end_ms: Some(40), last_update_ms: 40 };

    assert!(!should_render_think_block(false, 0, false, Some(&done)));
    assert!(!should_render_think_block(false, usize::MAX, false, Some(&done)));
    assert!(should_render_think_block(true, 0, false, Some(&done)));
    assert!(should_render_think_block(true, usize::MAX, false, Some(&done)));
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn think_block_view_builds_collapsed_and_empty_expanded_states() {
    let (mut app, _) = App::new();

    keep_element(think_block_view(&app, 1, 2, "collapsed".to_string(), false, None));

    let key = (1_u64 << 32) | 2;
    app.chat_think_expanded.insert(key);
    keep_element(think_block_view(&app, 1, 2, " \n\r ".to_string(), false, None));
}

#[test]
fn think_block_view_uses_editor_for_completed_short_content() {
    let (mut app, _) = App::new();
    let key = (3_u64 << 32) | 4;
    app.chat_think_expanded.insert(key);
    app.chat_think_editors.insert(key, text_editor::Content::with_text("reasoning"));
    let done = ThinkTiming { start_ms: 1, end_ms: Some(2_001), last_update_ms: 2_001 };

    keep_element(think_block_view(&app, 3, 4, "reasoning".to_string(), false, Some(&done)));
}

#[test]
fn think_block_view_uses_text_fallback_for_completed_short_content_without_editor() {
    let (mut app, _) = App::new();
    let key = (4_u64 << 32) | 5;
    app.chat_think_expanded.insert(key);
    let done = ThinkTiming { start_ms: 1_000, end_ms: Some(3_000), last_update_ms: 3_000 };

    keep_element(think_block_view(&app, 4, 5, "short reasoning".to_string(), false, Some(&done)));
}

#[test]
fn think_block_view_uses_segmented_body_for_long_completed_content() {
    let (mut app, _) = App::new();
    let key = (5_u64 << 32) | 6;
    app.chat_think_expanded.insert(key);
    app.chat_think_hovered_idx = Some(key);
    let done = ThinkTiming { start_ms: 0, end_ms: Some(5_000), last_update_ms: 5_000 };

    keep_element(think_block_view(&app, 5, 6, "long reasoning\n".repeat(110), false, Some(&done)));
}

#[test]
fn think_block_view_builds_running_body() {
    let (mut app, _) = App::new();
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
    let (mut app, _) = App::new();
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
