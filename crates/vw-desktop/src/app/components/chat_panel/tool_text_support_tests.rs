use super::tool_text_support::{
    ToolTextTarget, chat_text_font, chat_text_font_name, chat_text_line_height,
    chat_text_selection_color, is_safe_for_text_editor, message_target_from_context,
    read_only_text_style, selected_chat_text_for_message, selected_chat_text_for_target,
    selected_chat_text_for_tool, tool_inline_text_editor, tool_text_editor, tool_text_key,
    tool_text_style, tool_text_style_with_danger,
};
use crate::app::App;
use iced::widget::text_editor;
use iced::{Color, Theme};

#[test]
fn tool_text_key_packs_indices_without_collisions_for_adjacent_values() {
    assert_ne!(tool_text_key(1, 2, 3), tool_text_key(1, 2, 4));
    assert_ne!(tool_text_key(1, 2, 3), tool_text_key(1, 3, 3));
    assert_eq!(tool_text_key(0, 0, 0), 0);
}

#[test]
fn text_editor_safety_rejects_oversized_lines() {
    assert!(is_safe_for_text_editor("short\ntext"));
    assert!(!is_safe_for_text_editor(&"x".repeat(2_001)));
    assert!(!is_safe_for_text_editor(&"x".repeat(20_001)));
    assert!(is_safe_for_text_editor(""));
}

#[test]
fn chat_text_font_name_is_configured() {
    assert!(!chat_text_font_name().is_empty());
    let _ = chat_text_font();
    match chat_text_line_height() {
        iced::widget::text::LineHeight::Relative(value) => assert_eq!(value, 1.6),
        _ => panic!("chat text line height should be relative"),
    }
}

#[test]
fn text_styles_use_theme_selection_and_danger_color() {
    let theme = Theme::Dark;
    let selection = chat_text_selection_color(&theme);
    assert!(selection.a > 0.0);

    let readonly = read_only_text_style(&theme, Color::from_rgb8(1, 2, 3));
    assert_eq!(readonly.value, Color::from_rgb8(1, 2, 3));
    assert_eq!(readonly.border.width, 0.0);

    let normal = tool_text_style(&theme, text_editor::Status::Active);
    let danger = tool_text_style_with_danger(&theme, text_editor::Status::Active, true);
    assert_ne!(normal.value, danger.value);
}

#[test]
fn text_editor_lookup_returns_none_for_missing_targets_and_some_for_present_targets() {
    let mut app = App::new().0;

    assert!(
        tool_text_editor(
            &app,
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
            &app,
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
            &app,
            ToolTextTarget::SpecialMessageText { msg_idx: 7, text_idx: 8 },
            chat_text_font_name(),
            12.0,
            |theme| theme.palette().text,
        )
        .is_some()
    );
    assert_eq!(selected_chat_text_for_target(&app, 1_u64 << 32), None);
}

#[test]
fn oversized_text_targets_fall_back_to_preview_elements() {
    let mut app = App::new().0;
    app.chat_tool_text_editors
        .insert(tool_text_key(0, 0, 0), text_editor::Content::with_text(&"x".repeat(20_001)));

    assert!(
        tool_text_editor(
            &app,
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
            &app,
            ToolTextTarget::ToolCardText { msg_idx: 0, tool_idx: 0, text_idx: 0 },
            "JetBrains Mono",
            12.0,
            |theme| theme.palette().text,
        )
        .is_some()
    );
}

#[test]
fn message_target_from_context_decodes_bit_packed_target() {
    assert_eq!(message_target_from_context(0), (0, None));
    let t = (7usize as u64) << 32;
    assert_eq!(message_target_from_context(t), (7, None));
    let t = ((7usize as u64) << 32) | 3;
    assert_eq!(message_target_from_context(t), (7, Some(2)));
    let t = ((7usize as u64) << 32) | 1;
    assert_eq!(message_target_from_context(t), (7, Some(0)));
    let t = ((7usize as u64) << 32) | 0xFFFF_FFFF;
    assert_eq!(message_target_from_context(t), (7, Some(0xFFFF_FFFE)));
}

#[test]
fn chat_text_selection_color_differs_between_dark_and_light_themes() {
    let dark_color = chat_text_selection_color(&Theme::Dark);
    let light_color = chat_text_selection_color(&Theme::Light);
    assert_ne!(dark_color, light_color);
    assert!(dark_color.a > 0.0);
    assert!(light_color.a > 0.0);
}

#[test]
fn tool_text_editor_returns_none_for_missing_special_message_text() {
    let app = App::new().0;
    assert!(
        tool_text_editor(
            &app,
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
    let mut app = App::new().0;
    app.chat_special_text_editors.insert(0u64, text_editor::Content::with_text("special text"));

    assert!(
        tool_text_editor(
            &app,
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
            &app,
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
    let mut app = App::new().0;
    app.chat_special_text_editors
        .insert(0u64, text_editor::Content::with_text(&"x".repeat(20_001)));

    assert!(
        tool_text_editor(
            &app,
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
    let mut app = App::new().0;
    assert!(
        tool_inline_text_editor(
            &app,
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
            &app,
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
    let mut app = App::new().0;
    app.chat_message_editors.push(text_editor::Content::with_text("hello"));
    assert!(selected_chat_text_for_message(&app, 0).is_none());
}

#[test]
fn selected_chat_text_for_message_prefers_message_editor_over_fallback_sources() {
    let mut app = App::new().0;
    app.chat_message_editors.push(text_editor::Content::with_text("message selected"));
    app.chat_message_editors[0].perform(text_editor::Action::SelectAll);

    let block_key = 0_u128 << 64;
    app.chat_markdown_block_editors
        .insert(block_key, text_editor::Content::with_text("block selected"));
    app.chat_markdown_block_editors
        .get_mut(&block_key)
        .expect("block editor")
        .perform(text_editor::Action::SelectAll);

    assert_eq!(selected_chat_text_for_message(&app, 0).as_deref(), Some("message selected"));
}

#[test]
fn selected_chat_text_for_message_falls_back_to_block_special_tool_and_think_sources() {
    let mut block_app = App::new().0;
    let block_key = 2_u128 << 64;
    block_app
        .chat_markdown_block_editors
        .insert(block_key, text_editor::Content::with_text("block selected"));
    block_app
        .chat_markdown_block_editors
        .get_mut(&block_key)
        .expect("block editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&block_app, 2).as_deref(), Some("block selected"));

    let mut special_app = App::new().0;
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
        selected_chat_text_for_message(&special_app, 3).as_deref(),
        Some("special selected")
    );

    let mut tool_app = App::new().0;
    let tool_key = tool_text_key(4, 1, 0);
    tool_app
        .chat_tool_text_editors
        .insert(tool_key, text_editor::Content::with_text("tool selected"));
    tool_app
        .chat_tool_text_editors
        .get_mut(&tool_key)
        .expect("tool editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&tool_app, 4).as_deref(), Some("tool selected"));

    let mut think_app = App::new().0;
    let think_key = 5_u64 << 32;
    think_app
        .chat_think_editors
        .insert(think_key, text_editor::Content::with_text("think selected"));
    think_app
        .chat_think_editors
        .get_mut(&think_key)
        .expect("think editor")
        .perform(text_editor::Action::SelectAll);
    assert_eq!(selected_chat_text_for_message(&think_app, 5).as_deref(), Some("think selected"));
}

#[test]
fn tool_inline_text_editor_handles_missing_and_oversized_special_message_text() {
    let mut app = App::new().0;
    assert!(
        tool_inline_text_editor(
            &app,
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
            &app,
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
    let mut app = App::new().0;
    app.chat_tool_text_editors
        .insert(tool_text_key(0, 0, 0), text_editor::Content::with_text("msg0_tool0"));
    app.chat_tool_text_editors
        .insert(tool_text_key(0, 1, 0), text_editor::Content::with_text("msg0_tool1"));
    app.chat_tool_text_editors
        .insert(tool_text_key(1, 0, 0), text_editor::Content::with_text("msg1_tool0"));

    assert!(selected_chat_text_for_tool(&app, 0, 0).is_none());
    assert!(selected_chat_text_for_tool(&app, 0, 1).is_none());
    assert!(selected_chat_text_for_tool(&app, 1, 0).is_none());
    assert!(selected_chat_text_for_tool(&app, 2, 0).is_none());
}

#[test]
fn selected_chat_text_for_target_routes_to_selected_tool_text() {
    let mut app = App::new().0;
    let key = tool_text_key(3, 2, 0);
    app.chat_tool_text_editors.insert(key, text_editor::Content::with_text("selected tool text"));
    app.chat_tool_text_editors
        .get_mut(&key)
        .expect("tool editor")
        .perform(text_editor::Action::SelectAll);

    assert_eq!(selected_chat_text_for_tool(&app, 3, 2).as_deref(), Some("selected tool text"));
    assert_eq!(
        selected_chat_text_for_target(&app, (3_u64 << 32) | 3).as_deref(),
        Some("selected tool text")
    );
    assert_eq!(selected_chat_text_for_target(&app, (3_u64 << 32) | 2), None);
}

#[test]
fn tool_text_style_without_danger_matches_tool_text_style() {
    let theme = Theme::Dark;
    let default = tool_text_style(&theme, text_editor::Status::Active);
    let no_danger = tool_text_style_with_danger(&theme, text_editor::Status::Active, false);
    assert_eq!(default.value, no_danger.value);
}
