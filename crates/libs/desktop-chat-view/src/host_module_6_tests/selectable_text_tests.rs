use super::text_support::{selected_chat_text_for_message, tool_text_key};
use crate::chat_ui::prepare_chat_ui_chunk_phase;
use crate::host_module_5_tests::fixture::RenderFixture as App;
use crate::selectable_text::{
    SelectableTextPolicy, read_only_editor_content, selectable_text_policy,
};
use iced::widget::text_editor;
use session_ui_types::{ChatMessage, ChatRole};

#[test]
fn selectable_text_policy_uses_preview_for_unsafe_text() {
    let unsafe_text = "x".repeat(2_001);

    assert_eq!(
        selectable_text_policy(&unsafe_text, false),
        SelectableTextPolicy::PreviewWithCopyFallback
    );
    assert!(read_only_editor_content(&unsafe_text).is_none());

    let (mut app, _task) = App::new();
    app.chat = vec![ChatMessage {
        role: ChatRole::User,
        content: unsafe_text,
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }];
    let phase = prepare_chat_ui_chunk_phase(&app.chat, 0, true, false);
    app.chat_ui().apply_prepared_chat_ui_phase(phase);

    assert!(app.chat_message_editors[0].text().is_empty());
}

#[test]
fn selectable_text_reads_all_aux_editors() {
    let (mut app, _task) = App::new();
    app.chat_message_editors.push(text_editor::Content::with_text("message"));

    let special_key = 0_u64;
    let tool_key = tool_text_key(0, 1, 0);
    let think_key = 0_u64;
    app.chat_special_text_editors.insert(special_key, text_editor::Content::with_text("special"));
    app.chat_tool_text_editors.insert(tool_key, text_editor::Content::with_text("tool"));
    app.chat_think_editors.insert(think_key, text_editor::Content::with_text("think"));

    app.chat_special_text_editors
        .get_mut(&special_key)
        .expect("special editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&app, 0).as_deref(), Some("special"));

    app.chat_special_text_editors.clear();
    app.chat_tool_text_editors
        .get_mut(&tool_key)
        .expect("tool editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&app, 0).as_deref(), Some("tool"));

    app.chat_tool_text_editors.clear();
    app.chat_think_editors
        .get_mut(&think_key)
        .expect("think editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&app, 0).as_deref(), Some("think"));
}

#[test]
fn selectable_text_reads_markdown_block_editor_selection() {
    use crate::message_view::markdown::markdown_block_key;

    let (mut app, _task) = App::new();
    let block_key = markdown_block_key(0, 0, 1);
    app.chat_markdown_block_editors
        .insert(block_key, text_editor::Content::with_text("markdown 块文本"));

    // 未选中时不应捕获到文本。
    assert_eq!(selected_chat_text_for_message(&app, 0), None);

    app.chat_markdown_block_editors
        .get_mut(&block_key)
        .expect("markdown block editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&app, 0).as_deref(), Some("markdown 块文本"));
}
