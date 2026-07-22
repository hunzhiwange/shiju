use super::*;
use crate::app::App;
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

#[test]
fn storage_provider_options_keep_supported_order() {
    assert_eq!(STORAGE_PROVIDER_OPTIONS, ["postgres", "mariadb", "sqlite"]);
}

#[test]
fn field_row_builds_label_description_and_control_columns() {
    let tree = element_tree(field_row("标签", "说明", text("control")));

    assert_eq!(tree.children.len(), 2);
    assert_eq!(tree.children[0].children.len(), 2);
    assert!(tree.children[1].children.is_empty());
}

#[test]
fn text_row_places_one_text_input_in_control_column() {
    let tree =
        element_tree(text_row("地址", "说明", "placeholder", "value", |_| Message::None));

    assert_eq!(tree.children.len(), 2);
    assert!(tree.children[1].children.is_empty());
}

#[test]
fn text_row_accepts_unicode_and_multiline_connection_values() {
    for value in ["数据库.example/记忆", "第一行\n第二行", "🔒 postgres"] {
        let tree =
            element_tree(text_row("地址", "说明", "placeholder", value, |_| Message::None));

        assert!(tree.children[1].children.is_empty());
    }
}

#[test]
fn view_builds_all_storage_sections_and_field_rows() {
    let tree = element_tree(view(&test_app()));

    assert_eq!(tree.children.len(), 9);
    assert_eq!(tree.children[0].children.len(), 2);

    for section_index in [1, 4, 7] {
        let section = &tree.children[section_index];
        assert_eq!(section.children.len(), 2);
    }

    let connection_panel = &tree.children[2];
    assert_eq!(connection_panel.children.len(), 3);

    let naming_panel = &tree.children[5];
    assert_eq!(naming_panel.children.len(), 5);

    let security_panel = &tree.children[8];
    assert_eq!(security_panel.children.len(), 1);
}

#[test]
fn view_builds_each_supported_and_unknown_provider_state() {
    let mut app = test_app();

    for provider in STORAGE_PROVIDER_OPTIONS {
        app.storage_settings.provider = provider.to_string();
        assert_eq!(element_tree(view(&app)).children.len(), 9, "provider={provider}");
    }

    app.storage_settings.provider = "unsupported".to_string();
    assert_eq!(element_tree(view(&app)).children.len(), 9);
}

#[test]
fn view_builds_empty_and_populated_connection_states() {
    let mut app = test_app();
    app.storage_settings.provider.clear();
    app.storage_settings.db_url_input.clear();
    app.storage_settings.schema.clear();
    app.storage_settings.table.clear();
    app.storage_settings.connect_timeout_secs_input.clear();
    app.storage_settings.tls = false;
    assert_eq!(element_tree(view(&app)).children.len(), 9);

    app.storage_settings.provider = "postgres".to_string();
    app.storage_settings.db_url_input = "postgres://user:pass@localhost/db".to_string();
    app.storage_settings.schema = "public".to_string();
    app.storage_settings.table = "memories".to_string();
    app.storage_settings.connect_timeout_secs_input = "30".to_string();
    app.storage_settings.tls = true;
    assert_eq!(element_tree(view(&app)).children.len(), 9);
}
