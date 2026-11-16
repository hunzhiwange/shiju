//! App缓存同步的组合断言；测量组件断言随组件迁移。
use super::apply_chat_message_measurements;
use crate::host_module_5_tests::fixture::RenderFixture as App;
use crate::message_measure::ChatMessageMeasurement;
use crate::virtualizer::ChatVirtualizer;
use session_ui_types::{ChatMessage, ChatRole};

fn test_app() -> App {
    App::new().0
}

fn chat_message(content: &str) -> ChatMessage {
    ChatMessage {
        role: ChatRole::Assistant,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn chat_message_measure_ignores_unchanged_heights() {
    let mut app = test_app();
    app.chat = vec![chat_message("answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0]);
    app.chat_ui().sync_chat_virtualizer_compat_fields();
    app.chat_message_measured_heights.insert(0, 100.0);
    app.chat_render_cache.insert(0, Default::default());
    app.chat_visible_text_cache = vec![Some("copy".to_string())];
    app.chat_copy_hash_cache = vec![Some(42)];

    let changed = apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 10.0, height: 100.4, target: None }],
    );

    assert!(!changed);
    assert_eq!(app.chat_message_estimated_heights, vec![100.0]);
    assert_eq!(app.chat_message_measured_heights.get(&0), Some(&100.0));
    assert!(app.chat_render_cache.contains_key(&0));
    assert_eq!(app.chat_visible_text_cache, vec![Some("copy".to_string())]);
    assert_eq!(app.chat_copy_hash_cache, vec![Some(42)]);
}

#[test]
fn chat_message_measure_updates_changed_heights_without_touching_copy_caches() {
    let mut app = test_app();
    app.chat = vec![chat_message("answer")];
    app.chat_virtualizer = ChatVirtualizer::from_estimated_heights(&[100.0]);
    app.chat_ui().sync_chat_virtualizer_compat_fields();
    app.chat_render_cache.insert(0, Default::default());
    app.chat_visible_text_cache = vec![Some("copy".to_string())];
    app.chat_copy_hash_cache = vec![Some(42)];

    let changed = apply_chat_message_measurements(
        &mut app,
        vec![ChatMessageMeasurement { message_idx: 0, y: 10.0, height: 124.0, target: None }],
    );

    assert!(changed);
    assert_eq!(app.chat_message_estimated_heights, vec![124.0]);
    assert_eq!(app.chat_message_measured_heights.get(&0), Some(&124.0));
    assert_eq!(app.chat_height_index.scroll_top_for_item(0), 0.0);
    assert!(app.chat_render_cache.contains_key(&0));
    assert_eq!(app.chat_visible_text_cache, vec![Some("copy".to_string())]);
    assert_eq!(app.chat_copy_hash_cache, vec![Some(42)]);
}
