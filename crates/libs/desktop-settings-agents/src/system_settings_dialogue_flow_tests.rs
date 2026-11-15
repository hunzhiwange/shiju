//! 验证对话流开关消息、字段布局及保存结果横幅的判定。
//! 保存成功只接受约定前缀，避免普通提示被误判为成功状态。

use super::*;

use iced::Length;
use iced::advanced::widget::Tree;
use iced::widget::text;

// 每个开关组合都基于同一默认应用，避免其他设置影响字段树数量。
fn test_app() -> Fixture {
    Fixture::default()
}

fn view_tree(app: &Fixture) -> Tree {
    let element = view(&inputs(app));
    Tree::new(element.as_widget())
}

fn assert_field_tree(tree: &Tree) {
    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].children.len(), 2);
    assert!(tree.children[1].children.is_empty());
}

#[test]
fn field_row_keeps_label_help_and_control_in_separate_columns() {
    let app = test_app();
    let element = field_row(&inputs(&app), "显示推理摘要", "说明", text("control"));
    assert_eq!(element.as_widget().size().width, Length::Fill);

    let tree = Tree::new(element.as_widget());
    assert_field_tree(&tree);
}

#[test]
fn reasoning_summary_toggle_message_preserves_both_boolean_values() {
    for expected in [false, true] {
        let Message::DialogueFlowShowReasoningSummaryToggled(actual) =
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
        let Message::DialogueFlowExpandShellToolSectionToggled(actual) =
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
        let Message::DialogueFlowExpandEditToolSectionToggled(actual) =
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

#[derive(Default)] struct Fixture { dialogue_flow_show_reasoning_summary: bool, dialogue_flow_expand_shell_tool_section: bool, dialogue_flow_expand_edit_tool_section: bool, dialogue_flow_settings_save_message: Option<String>, dialogue_flow_settings_translation: Option<desktop_i18n::format::Message> }
fn inputs(app: &Fixture) -> super::super::dialogue_flow_inputs::View<'_> { super::super::dialogue_flow_inputs::View { dialogue_flow_show_reasoning_summary: app.dialogue_flow_show_reasoning_summary, dialogue_flow_expand_shell_tool_section: app.dialogue_flow_expand_shell_tool_section, dialogue_flow_expand_edit_tool_section: app.dialogue_flow_expand_edit_tool_section, dialogue_flow_settings_save_message: &app.dialogue_flow_settings_save_message, dialogue_flow_settings_translation: &app.dialogue_flow_settings_translation, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese, app_theme: &iced::Theme::Light,  help_text: help } }
fn help<'a>(_: desktop_i18n::I18n, _: Option<&iced::Theme>, _: &'a str, label: iced::widget::Text<'a>, body: std::borrow::Cow<'a, str>) -> iced::widget::Row<'a, Message> { desktop_ui_controls::system_settings_common::help::settings_hover_help_label(label, iced::widget::text(body).into()) }
