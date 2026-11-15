//! 验证存储提供商选项、连接字段和不同配置状态下的布局。
//!
//! 连接字符串包含空值、Unicode 与多行内容，确保输入控件不误改原始配置。

use crate::view_test_support::*;
use crate::system_settings_storage::STORAGE_PROVIDER_OPTIONS;

use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn fixture() -> Fixture { Fixture::default() }

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

#[test]
fn field_row_builds_help_label_and_control_columns() {
    let app = fixture();
    let tree = element_tree(field_row_with_help(&app, "标签", "说明", text("control")));

    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].children[0].children.len(), 2);
    assert!(tree.children[1].children.is_empty());
}

#[test]
fn text_row_accepts_empty_unicode_and_multiline_connection_values() {
    let app = fixture();
    for value in ["", "数据库.example/记忆", "第一行\n第二行", "🔒 postgres"] {
        let tree = element_tree(text_row_with_help(
            &app,
            "地址",
            "说明",
            "placeholder",
            value,
            |_| Message::None,
        ));

        assert_eq!(tree.children.len(), 2);
        assert!(tree.children[1].children.is_empty());
    }
}

#[test]
fn embedded_view_builds_all_storage_sections_and_field_rows() {
    let tree = element_tree(embedded_view(&fixture()));

    assert_eq!(tree.children.len(), 6);
    for section_index in [0, 2, 4] {
        let heading = &tree.children[section_index];
        assert_eq!(heading.children.len(), 2);
    }

    for (panel_index, field_count) in [(1, 2), (3, 3), (5, 1)] {
        let panel = &tree.children[panel_index];
        assert_eq!(panel.children.len(), 2 * field_count - 1);
        for field in panel.children.iter().step_by(2) {
            assert_eq!(field.children.len(), 2);
            let labels = &field.children[0];
            assert_eq!(labels.children.len(), 1);
            assert_eq!(labels.children[0].children.len(), 2);
        }
    }
}

#[test]
fn view_builds_each_supported_and_unknown_provider_state() {
    let mut app = fixture();

    for provider in STORAGE_PROVIDER_OPTIONS {
        app.storage_settings.provider = provider.to_string();
        assert_eq!(element_tree(embedded_view(&app)).children.len(), 6, "provider={provider}");
    }

    app.storage_settings.provider = "unsupported".to_string();
    assert_eq!(element_tree(embedded_view(&app)).children.len(), 6);
}

#[test]
fn view_builds_empty_and_populated_connection_states() {
    let mut app = fixture();
    app.storage_settings.provider.clear();
    app.storage_settings.db_url_input.clear();
    app.storage_settings.schema.clear();
    app.storage_settings.table.clear();
    app.storage_settings.connect_timeout_secs_input.clear();
    app.storage_settings.tls = false;
    assert_eq!(element_tree(embedded_view(&app)).children.len(), 6);

    app.storage_settings.provider = "postgres".to_string();
    app.storage_settings.db_url_input = "postgres://user:pass@localhost/db".to_string();
    app.storage_settings.schema = "public".to_string();
    app.storage_settings.table = "memories".to_string();
    app.storage_settings.connect_timeout_secs_input = "30".to_string();
    app.storage_settings.tls = true;
    assert_eq!(element_tree(embedded_view(&app)).children.len(), 6);
}
