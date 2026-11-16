#![allow(unused_must_use)]
use super::handle_markdown_block_editor_action;
use std::collections::HashMap;
#[derive(Default)]
struct Editors {
    chat_markdown_block_editors: HashMap<u128, text_editor::Content>,
}

use iced::widget::text_editor;

fn test_app() -> Editors {
    Editors::default()
}

#[test]
fn editor_actions_tests_module_is_wired() {
    assert!(module_path!().ends_with("editor_actions_tests"));
}

#[test]
fn markdown_block_editor_ignores_edit_actions_and_applies_navigation_actions() {
    let mut app = test_app();
    let block_key = ((3u128) << 64) | ((4u128) << 32) | 5u128;
    app.chat_markdown_block_editors
        .insert(block_key, text_editor::Content::with_text("markdown block"));

    // Edit 动作被忽略。
    handle_markdown_block_editor_action(
        &mut app.chat_markdown_block_editors,
        block_key,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            " changed".to_string(),
        ))),
    );
    assert_eq!(app.chat_markdown_block_editors[&block_key].text(), "markdown block");

    // 非 Edit 动作（SelectAll）生效。
    handle_markdown_block_editor_action(
        &mut app.chat_markdown_block_editors,
        block_key,
        text_editor::Action::SelectAll,
    );
    assert_eq!(
        app.chat_markdown_block_editors
            .get(&block_key)
            .and_then(text_editor::Content::selection)
            .as_deref(),
        Some("markdown block")
    );

    // 缺失键返回 None 分支。
    let _task = handle_markdown_block_editor_action(
        &mut app.chat_markdown_block_editors,
        u128::MAX,
        text_editor::Action::SelectAll,
    );
}
