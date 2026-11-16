use crate::tool_text_support::{ToolTextTarget, chat_text_font_name, selected_chat_text_for_message, selected_chat_text_for_target, selected_chat_text_for_tool, tool_inline_text_editor, tool_text_editor, tool_text_key};
use super::TextEditors;
use iced::widget::text_editor;

fn selected_editor(text: &str) -> text_editor::Content {
    let mut editor = text_editor::Content::with_text(text);
    editor.perform(text_editor::Action::SelectAll);
    editor
}

#[test]
fn text_editor_lookup_returns_none_for_missing_targets_and_some_for_present_targets() {
    let mut app = TextEditors::default();

    assert!(
        tool_text_editor(
            &app.state(),
            ToolTextTarget::ToolCardText { msg_idx: 1, tool_idx: 2, text_idx: 3 },
            "JetBrains Mono",
            12.0,
            false,
            false,
        )
        .is_none()
    );
    app.chat_tool_text_editors
        .insert(tool_text_key(1, 2, 3), text_editor::Content::with_text("hello"));
    app.chat_special_text_editors
        .insert((7_u64 << 32) | 8, text_editor::Content::with_text("special"));

    assert!(
        tool_text_editor(
            &app.state(),
            ToolTextTarget::ToolCardText { msg_idx: 1, tool_idx: 2, text_idx: 3 },
            "JetBrains Mono",
            12.0,
            true,
            false,
        )
        .is_some()
    );
    assert!(
        tool_inline_text_editor(
            &app.state(),
            ToolTextTarget::SpecialMessageText { msg_idx: 7, text_idx: 8 },
            chat_text_font_name(),
            12.0,
            |theme| theme.palette().text,
        )
        .is_some()
    );
    assert_eq!(selected_chat_text_for_target(&app.state(), 1_u64 << 32), None);
}

#[test]
fn oversized_text_targets_fall_back_to_preview_elements() {
    let mut app = TextEditors::default();
    app.chat_tool_text_editors
        .insert(tool_text_key(0, 0, 0), text_editor::Content::with_text(&"x".repeat(20_001)));

    assert!(
        tool_text_editor(
            &app.state(),
            ToolTextTarget::ToolCardText { msg_idx: 0, tool_idx: 0, text_idx: 0 },
            "JetBrains Mono",
            12.0,
            false,
            true,
        )
        .is_some()
    );
    assert!(
        tool_inline_text_editor(
            &app.state(),
            ToolTextTarget::ToolCardText { msg_idx: 0, tool_idx: 0, text_idx: 0 },
            "JetBrains Mono",
            12.0,
            |theme| theme.palette().text,
        )
        .is_some()
    );
}

#[test]
fn tool_text_editor_returns_none_for_missing_special_message_text() {
    let app = TextEditors::default();
    assert!(
        tool_text_editor(
            &app.state(),
            ToolTextTarget::SpecialMessageText { msg_idx: 0, text_idx: 0 },
            "JetBrains Mono",
            12.0,
            false,
            false,
        )
        .is_none()
    );
}

#[test]
fn tool_text_editor_renders_special_message_text_with_and_without_highlight() {
    let mut app = TextEditors::default();
    app.chat_special_text_editors.insert(0u64, text_editor::Content::with_text("special text"));

    assert!(
        tool_text_editor(
            &app.state(),
            ToolTextTarget::SpecialMessageText { msg_idx: 0, text_idx: 0 },
            "JetBrains Mono",
            12.0,
            true,
            false,
        )
        .is_some()
    );
    assert!(
        tool_text_editor(
            &app.state(),
            ToolTextTarget::SpecialMessageText { msg_idx: 0, text_idx: 0 },
            "JetBrains Mono",
            12.0,
            false,
            true,
        )
        .is_some()
    );
}

#[test]
fn tool_text_editor_falls_back_to_preview_for_oversized_special_message_text() {
    let mut app = TextEditors::default();
    app.chat_special_text_editors
        .insert(0u64, text_editor::Content::with_text(&"x".repeat(20_001)));

    assert!(
        tool_text_editor(
            &app.state(),
            ToolTextTarget::SpecialMessageText { msg_idx: 0, text_idx: 0 },
            "JetBrains Mono",
            12.0,
            true,
            true,
        )
        .is_some()
    );
}

#[test]
fn tool_inline_text_editor_returns_none_and_some_for_tool_card_text() {
    let mut app = TextEditors::default();
    assert!(
        tool_inline_text_editor(
            &app.state(),
            ToolTextTarget::ToolCardText { msg_idx: 0, tool_idx: 1, text_idx: 2 },
            "JetBrains Mono",
            12.0,
            |theme| theme.palette().text,
        )
        .is_none()
    );

    app.chat_tool_text_editors
        .insert(tool_text_key(0, 1, 2), text_editor::Content::with_text("tool card"));
    assert!(
        tool_inline_text_editor(
            &app.state(),
            ToolTextTarget::ToolCardText { msg_idx: 0, tool_idx: 1, text_idx: 2 },
            "JetBrains Mono",
            12.0,
            |theme| theme.palette().text,
        )
        .is_some()
    );
}

#[test]
fn selected_chat_text_for_message_returns_none_when_no_selection() {
    let mut app = TextEditors::default();
    app.chat_message_editors.push(text_editor::Content::with_text("hello"));
    assert!(selected_chat_text_for_message(&app.state(), 0).is_none());
}

#[test]
fn selected_chat_text_for_message_prefers_visible_markdown_over_message_editor() {
    let mut app = TextEditors::default();
    app.chat_message_editors.push(text_editor::Content::with_text("message selected"));
    app.chat_message_editors[0].perform(text_editor::Action::SelectAll);

    let block_key = 0_u128 << 64;
    app.chat_markdown_block_editors
        .insert(block_key, text_editor::Content::with_text("block selected"));
    app.chat_markdown_block_editors
        .get_mut(&block_key)
        .expect("block editor")
        .perform(text_editor::Action::SelectAll);

    assert_eq!(selected_chat_text_for_message(&app.state(), 0).as_deref(), Some("block selected"));
}

#[test]
fn formatted_markdown_selection_can_cross_blocks() {
    let mut app = TextEditors::default();
    app.chat_message_editors.push(selected_editor("# hidden source"));
    for (block_idx, text) in ["标题", "第一段", "列表项"].into_iter().enumerate() {
        app.chat_markdown_block_editors.insert(block_idx as u128, selected_editor(text));
    }

    assert_eq!(
        selected_chat_text_for_message(&app.state(), 0).as_deref(),
        Some("标题\n\n第一段\n\n列表项")
    );
}

#[test]
fn selected_chat_text_for_message_falls_back_to_block_special_tool_and_think_sources() {
    let mut block_app = TextEditors::default();
    let block_key = 2_u128 << 64;
    block_app
        .chat_markdown_block_editors
        .insert(block_key, text_editor::Content::with_text("block selected"));
    block_app
        .chat_markdown_block_editors
        .get_mut(&block_key)
        .expect("block editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&block_app.state(), 2).as_deref(), Some("block selected"));

    let mut special_app = TextEditors::default();
    let special_key = (3_u64 << 32) | 1;
    special_app
        .chat_special_text_editors
        .insert(special_key, text_editor::Content::with_text("special selected"));
    special_app
        .chat_special_text_editors
        .get_mut(&special_key)
        .expect("special editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(
        selected_chat_text_for_message(&special_app.state(), 3).as_deref(),
        Some("special selected")
    );

    let mut tool_app = TextEditors::default();
    let tool_key = tool_text_key(4, 1, 0);
    tool_app
        .chat_tool_text_editors
        .insert(tool_key, text_editor::Content::with_text("tool selected"));
    tool_app
        .chat_tool_text_editors
        .get_mut(&tool_key)
        .expect("tool editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&tool_app.state(), 4).as_deref(), Some("tool selected"));

    let mut think_app = TextEditors::default();
    let think_key = 5_u64 << 32;
    think_app
        .chat_think_editors
        .insert(think_key, text_editor::Content::with_text("think selected"));
    think_app
        .chat_think_editors
        .get_mut(&think_key)
        .expect("think editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&think_app.state(), 5).as_deref(), Some("think selected"));
}

#[test]
fn tool_inline_text_editor_handles_missing_and_oversized_special_message_text() {
    let mut app = TextEditors::default();
    assert!(
        tool_inline_text_editor(
            &app.state(),
            ToolTextTarget::SpecialMessageText { msg_idx: 1, text_idx: 2 },
            "JetBrains Mono",
            12.0,
            |theme| theme.palette().text,
        )
        .is_none()
    );

    app.chat_special_text_editors
        .insert((1_u64 << 32) | 2, text_editor::Content::with_text(&"x".repeat(20_001)));
    assert!(
        tool_inline_text_editor(
            &app.state(),
            ToolTextTarget::SpecialMessageText { msg_idx: 1, text_idx: 2 },
            "JetBrains Mono",
            12.0,
            |theme| theme.palette().text,
        )
        .is_some()
    );
}

#[test]
fn selected_chat_text_for_tool_filters_by_msg_and_tool_index() {
    let mut app = TextEditors::default();
    app.chat_tool_text_editors
        .insert(tool_text_key(0, 0, 0), text_editor::Content::with_text("msg0_tool0"));
    app.chat_tool_text_editors
        .insert(tool_text_key(0, 1, 0), text_editor::Content::with_text("msg0_tool1"));
    app.chat_tool_text_editors
        .insert(tool_text_key(1, 0, 0), text_editor::Content::with_text("msg1_tool0"));

    assert!(selected_chat_text_for_tool(&app.state(), 0, 0).is_none());
    assert!(selected_chat_text_for_tool(&app.state(), 0, 1).is_none());
    assert!(selected_chat_text_for_tool(&app.state(), 1, 0).is_none());
    assert!(selected_chat_text_for_tool(&app.state(), 2, 0).is_none());
}

#[test]
fn selected_chat_text_for_target_routes_to_selected_tool_text() {
    let mut app = TextEditors::default();
    let key = tool_text_key(3, 2, 0);
    app.chat_tool_text_editors.insert(key, text_editor::Content::with_text("selected tool text"));
    app.chat_tool_text_editors
        .get_mut(&key)
        .expect("tool editor")
        .perform(text_editor::Action::SelectAll);

    assert_eq!(selected_chat_text_for_tool(&app.state(), 3, 2).as_deref(), Some("selected tool text"));
    assert_eq!(
        selected_chat_text_for_target(&app.state(), (3_u64 << 32) | 3).as_deref(),
        Some("selected tool text")
    );
    assert_eq!(selected_chat_text_for_target(&app.state(), (3_u64 << 32) | 2), None);
}
