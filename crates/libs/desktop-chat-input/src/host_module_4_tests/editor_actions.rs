use desktop_session::message::chat::input::editor_actions::handle_markdown_block_editor_action;
use iced::widget::text_editor;
#[test]
fn markdown_block_editor_ignores_edit_actions_and_applies_navigation_actions() {
    let mut editors = std::collections::HashMap::new();
    let block_key = desktop_chat_view::message_view::markdown_block_key(3, 4, 5);
    editors.insert(block_key, text_editor::Content::with_text("markdown block"));

    // Edit 动作被忽略。
    handle_markdown_block_editor_action(
        &mut editors,
        block_key,
        text_editor::Action::Edit(text_editor::Edit::Paste(std::sync::Arc::new(
            " changed".to_string(),
        ))),
    );
    assert_eq!(editors[&block_key].text(), "markdown block");

    // 非 Edit 动作（SelectAll）生效。
    handle_markdown_block_editor_action(&mut editors, block_key, text_editor::Action::SelectAll);
    assert_eq!(
        editors.get(&block_key).and_then(text_editor::Content::selection).as_deref(),
        Some("markdown block")
    );

    // 缺失键返回 None 分支。
    handle_markdown_block_editor_action(
        &mut editors,
        desktop_chat_view::message_view::markdown_block_key(9, 9, 9),
        text_editor::Action::SelectAll,
    );
}

#[test]
fn editor_actions_tests_module_is_wired() {
    assert!(module_path!().ends_with("editor_actions_tests"));
}
