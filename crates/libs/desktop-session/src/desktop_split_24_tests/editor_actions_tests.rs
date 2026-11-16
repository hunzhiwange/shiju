use crate::message::chat::input::editor_actions::*;
use iced::widget::text_editor;
#[test]
fn message_editor_ignores_edit_actions_and_applies_navigation_actions() {
    let mut chat_message_editors = Vec::new();

    chat_message_editors.push(text_editor::Content::with_text("hello"));

    handle_message_editor_action(
        &mut chat_message_editors,
        0,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            " world".to_string(),
        ))),
    );
    assert_eq!(chat_message_editors[0].text(), "hello");

    handle_message_editor_action(&mut chat_message_editors, 0, text_editor::Action::SelectAll);
    assert_eq!(chat_message_editors[0].selection().as_deref(), Some("hello"));

    let _task =
        handle_message_editor_action(&mut chat_message_editors, 99, text_editor::Action::SelectAll);
}

#[test]
fn special_tool_and_think_editors_use_stable_composite_keys() {
    let mut chat_special_text_editors = std::collections::HashMap::new();
    let mut chat_think_editors = std::collections::HashMap::new();
    let mut chat_tool_text_editors = std::collections::HashMap::new();
    let special_key = ((2u64) << 32) | 3u64;
    let think_key = ((4u64) << 32) | 5u64;
    let tool_key = ((6u128) << 64) | ((7u128) << 32) | 8u128;

    chat_special_text_editors.insert(special_key, text_editor::Content::with_text("special"));
    chat_think_editors.insert(think_key, text_editor::Content::with_text("think"));
    chat_tool_text_editors.insert(tool_key, text_editor::Content::with_text("tool"));

    handle_special_text_editor_action(
        &mut chat_special_text_editors,
        2,
        3,
        text_editor::Action::SelectAll,
    );
    handle_think_editor_action(&mut chat_think_editors, 4, 5, text_editor::Action::SelectAll);
    handle_tool_text_editor_action(
        &mut chat_tool_text_editors,
        6,
        7,
        8,
        text_editor::Action::SelectAll,
    );

    assert_eq!(
        chat_special_text_editors
            .get(&special_key)
            .and_then(text_editor::Content::selection)
            .as_deref(),
        Some("special")
    );
    assert_eq!(
        chat_think_editors.get(&think_key).and_then(text_editor::Content::selection).as_deref(),
        Some("think")
    );
    assert_eq!(
        chat_tool_text_editors.get(&tool_key).and_then(text_editor::Content::selection).as_deref(),
        Some("tool")
    );

    handle_special_text_editor_action(
        &mut chat_special_text_editors,
        2,
        3,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            " changed".to_string(),
        ))),
    );
    assert_eq!(chat_special_text_editors[&special_key].text(), "special");

    let _task = handle_special_text_editor_action(
        &mut chat_special_text_editors,
        9,
        9,
        text_editor::Action::SelectAll,
    );
    let _task =
        handle_think_editor_action(&mut chat_think_editors, 9, 9, text_editor::Action::SelectAll);
    let _task = handle_tool_text_editor_action(
        &mut chat_tool_text_editors,
        9,
        9,
        9,
        text_editor::Action::SelectAll,
    );
}

#[test]
fn tool_and_think_editors_reject_edit_actions() {
    let mut chat_think_editors = std::collections::HashMap::new();
    let mut chat_tool_text_editors = std::collections::HashMap::new();
    let think_key = ((4u64) << 32) | 5u64;
    let tool_key = ((6u128) << 64) | ((7u128) << 32) | 8u128;
    chat_think_editors.insert(think_key, text_editor::Content::with_text("think"));
    chat_tool_text_editors.insert(tool_key, text_editor::Content::with_text("tool"));

    handle_think_editor_action(
        &mut chat_think_editors,
        4,
        5,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            " changed".to_string(),
        ))),
    );
    handle_tool_text_editor_action(
        &mut chat_tool_text_editors,
        6,
        7,
        8,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            " changed".to_string(),
        ))),
    );

    assert_eq!(chat_think_editors[&think_key].text(), "think");
    assert_eq!(chat_tool_text_editors[&tool_key].text(), "tool");
}
