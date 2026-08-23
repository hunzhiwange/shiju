//! 验证聊天输入滚动事件会安排可见消息测量。
//!
//! 滚动夹具创建足量虚拟消息，确认一次滚动只安排可见范围测量而不直接改写内容。

use crate::app::components::chat_panel::virtualizer::ChatVirtualizer;
use crate::app::models::{ChatMessage as ChatEntry, ChatRole};

fn chat_entry(idx: usize) -> ChatEntry {
    ChatEntry {
        role: ChatRole::Assistant,
        content: format!("message {idx}"),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn input_tests_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("input_tests"));
}

#[test]
fn chat_panel_scroll_schedules_visible_measurement() {
    let (mut app, _) = crate::app::App::new();
    app.chat = (0..120).map(chat_entry).collect();
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&vec![80.0; app.chat.len()]);

    let outcome = super::apply_chat_scroll_changed(&mut app, 0.5, 200.0, 240.0);

    assert!(outcome.measure_visible_messages);
    assert_eq!(app.chat_virtualizer.viewport_height_px(), 240.0);
    assert_eq!(app.visible_chat_message_window(), outcome.visible_range);
    assert!(outcome.visible_range.0 > 0);
    assert!(outcome.visible_range.1 < app.chat.len());
}
