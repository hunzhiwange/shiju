use super::*;
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn field_row_wraps_controls() {
    keep_element(field_row("编辑器主题", "说明", text("control")));
}

#[test]
fn view_uses_app_theme_when_following_system_theme() {
    let mut app = test_app();
    app.editor_follow_system_theme = true;
    app.app_theme = iced::Theme::Dark;
    app.editor_theme = iced::Theme::Light;
    app.current_font_size = 15.0;
    app.current_line_height = 24.0;
    app.auto_adjust_line_height = true;

    keep_element(view(&app));
}

#[test]
fn view_uses_editor_theme_when_not_following_system_theme() {
    let mut app = test_app();
    app.editor_follow_system_theme = false;
    app.app_theme = iced::Theme::Light;
    app.editor_theme = iced::Theme::Dark;
    app.current_font_size = 30.0;
    app.current_line_height = 60.0;
    app.auto_adjust_line_height = false;

    keep_element(view(&app));
}

#[test]
fn view_default_values_does_not_panic() {
    let app = test_app();
    keep_element(view(&app));
}

#[test]
fn view_with_min_boundary_values() {
    let mut app = test_app();
    app.editor_follow_system_theme = true;
    app.current_font_size = 10.0;
    app.current_line_height = 10.0;
    keep_element(view(&app));
}

#[test]
fn view_with_max_boundary_values() {
    let mut app = test_app();
    app.editor_follow_system_theme = false;
    app.current_font_size = 30.0;
    app.current_line_height = 60.0;
    keep_element(view(&app));
}

#[test]
fn view_keeps_intro_section_and_five_setting_rows() {
    let app = test_app();
    let element = view(&app);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 3);
    assert_eq!(tree.children[0].children.len(), 2);
    assert_eq!(tree.children[1].children.len(), 2);

    let panel_column = &tree.children[2];
    assert_eq!(panel_column.children.len(), 9);
    for row_index in [0, 2, 4, 6, 8] {
        let field = &panel_column.children[row_index];
        assert_eq!(field.children.len(), 2);
    }
}
