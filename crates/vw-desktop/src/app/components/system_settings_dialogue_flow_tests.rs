use super::*;
use crate::app::{App, Message, message::SettingsMessage};
use iced::advanced::widget::Tree;
use iced::widget::text;
use iced::{Background, Length, Theme};

fn test_app() -> App {
    App::new().0
}

fn view_tree(app: &App) -> Tree {
    let element = view(app);
    Tree::new(element.as_widget())
}

fn assert_field_tree(tree: &Tree) {
    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].children.len(), 2);
    assert!(tree.children[1].children.is_empty());
}

#[test]
fn field_row_keeps_label_description_and_control_in_separate_columns() {
    let element = field_row("显示推理摘要", "说明", text("control"));
    assert_eq!(element.as_widget().size().width, Length::Fill);

    let tree = Tree::new(element.as_widget());
    assert_field_tree(&tree);
}

#[test]
fn reasoning_summary_toggle_message_preserves_both_boolean_values() {
    for expected in [false, true] {
        let Message::Settings(SettingsMessage::DialogueFlowShowReasoningSummaryToggled(actual)) =
            show_reasoning_summary_toggled(expected)
        else {
            panic!("unexpected settings message variant");
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn shell_section_toggle_message_preserves_both_boolean_values() {
    for expected in [false, true] {
        let Message::Settings(SettingsMessage::DialogueFlowExpandShellToolSectionToggled(actual)) =
            expand_shell_tool_section_toggled(expected)
        else {
            panic!("unexpected settings message variant");
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn edit_section_toggle_message_preserves_both_boolean_values() {
    for expected in [false, true] {
        let Message::Settings(SettingsMessage::DialogueFlowExpandEditToolSectionToggled(actual)) =
            expand_edit_tool_section_toggled(expected)
        else {
            panic!("unexpected settings message variant");
        };
        assert_eq!(actual, expected);
    }
}

#[test]
fn success_save_message_requires_exact_leading_prefix() {
    assert!(is_success_save_message("已保存"));
    assert!(is_success_save_message("已保存对话流配置"));
    assert!(!is_success_save_message(""));
    assert!(!is_success_save_message("保存成功"));
    assert!(!is_success_save_message(" 已保存"));
}

#[test]
fn success_save_message_classifies_any_prefixed_detail_as_success() {
    assert!(is_success_save_message("已保存失败详情"));
    assert!(is_success_save_message("已保存\n下一行"));
}

#[test]
fn success_banner_style_uses_success_palette_in_light_and_dark_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let style = success_banner_style(&theme);

        assert_eq!(style.text_color, Some(palette.success.base.color));
        assert_eq!(
            style.background,
            Some(Background::Color(palette.success.weak.color.scale_alpha(0.18)))
        );
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.color, palette.success.base.color.scale_alpha(0.45));
        assert_eq!(style.border.radius.top_left, 14.0);
    }
}

#[test]
fn view_builds_intro_panel_and_three_fields_for_every_toggle_combination() {
    let mut app = test_app();
    for bits in 0_u8..8 {
        app.dialogue_flow_show_reasoning_summary = bits & 0b001 != 0;
        app.dialogue_flow_expand_shell_tool_section = bits & 0b010 != 0;
        app.dialogue_flow_expand_edit_tool_section = bits & 0b100 != 0;

        let tree = view_tree(&app);
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children[0].children.len(), 2);

        let panel_column = &tree.children[1];
        assert_eq!(panel_column.children.len(), 5);
        for row_index in [0, 2, 4] {
            assert_field_tree(&panel_column.children[row_index]);
        }
    }
}

#[test]
fn view_appends_exactly_one_banner_for_success_error_and_empty_messages() {
    let mut app = test_app();
    assert_eq!(view_tree(&app).children.len(), 2);

    for message in ["已保存对话流配置", "保存失败", ""] {
        app.dialogue_flow_settings_save_message = Some(message.to_string());
        let tree = view_tree(&app);
        assert_eq!(tree.children.len(), 3);
        assert!(tree.children[2].children.is_empty());
    }
}
