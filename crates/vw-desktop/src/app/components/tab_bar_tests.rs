use super::*;
use crate::app::message::ViewMessage;
use crate::app::state::AppTab;
use crate::app::{App, Message, Screen};
use iced::advanced::renderer::Headless;
use iced::advanced::widget::Tree;
use iced::widget::button;
use iced::{Background, Color, Font, Length, Pixels, Rectangle, Theme};

fn test_app() -> App {
    App::new().0
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn tab(id: &str, title: &str, screen: Screen) -> AppTab {
    AppTab { id: id.to_string(), title: title.to_string(), screen, project_path: None }
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn max_child_count(tree: &Tree) -> usize {
    tree.children
        .iter()
        .map(max_child_count)
        .fold(tree.children.len(), usize::max)
}

#[test]
fn view_uses_public_height_and_full_width() {
    let app = test_app();
    let element = view(&app);

    assert_eq!(element.as_widget().size().width, Length::Fill);
    assert_eq!(element.as_widget().size().height, Length::Fixed(TAB_BAR_HEIGHT));
}

#[test]
fn view_filters_internal_tabs_and_omits_add_button_when_apps_is_open() {
    let empty_tree = element_tree(view(&test_app()));
    assert_eq!(empty_tree.children.len(), 2);

    let mut app = test_app();
    app.open_tabs = vec![
        tab("home", "Home", Screen::Home),
        tab("find:internal", "Find", Screen::Home),
        tab("project", "Project", Screen::Project),
    ];
    app.hovered_tab_id = Some("project".to_string());
    let without_apps = element_tree(view(&app));
    assert_eq!(without_apps.children.len(), 3);
    assert_eq!(max_child_count(&without_apps.children[1]), 2);

    app.open_tabs.retain(|tab| tab.id != "project");
    app.open_tabs.push(tab("apps", "Apps", Screen::Apps));
    let with_apps = element_tree(view(&app));
    assert_eq!(with_apps.children.len(), 2);
}

#[test]
fn tab_button_close_layer_requires_hovered_non_home_tab() {
    let home = element_tree(tab_btn(
        "home".to_string(),
        "Home".to_string(),
        true,
        true,
        Message::View(ViewMessage::TabSelected("home".to_string())),
    ));
    let idle = element_tree(tab_btn(
        "project".to_string(),
        "Project".to_string(),
        false,
        false,
        Message::View(ViewMessage::TabSelected("project".to_string())),
    ));
    let hovered = element_tree(tab_btn(
        "project".to_string(),
        "Project".to_string(),
        false,
        true,
        Message::View(ViewMessage::TabSelected("project".to_string())),
    ));

    assert_eq!(max_child_count(&home), 1);
    assert_eq!(max_child_count(&idle), 1);
    assert_eq!(max_child_count(&hovered), 2);
}

#[test]
fn tab_button_keeps_shrink_width_and_full_height() {
    let element = tab_btn(
        "project".to_string(),
        "Project".to_string(),
        false,
        false,
        Message::View(ViewMessage::TabSelected("project".to_string())),
    );

    assert_eq!(element.as_widget().size().width, Length::Shrink);
    assert_eq!(element.as_widget().size().height, Length::Fill);
}

#[test]
fn display_tab_label_preserves_boundaries_and_truncates_unicode() {
    let home_boundary = "a".repeat(22);
    let regular_boundary = "界".repeat(24);

    assert_eq!(display_tab_label(&home_boundary, true), home_boundary);
    assert_eq!(
        display_tab_label(&format!("{home_boundary}b"), true),
        format!("{home_boundary}...")
    );
    assert_eq!(display_tab_label(&regular_boundary, false), regular_boundary);
    assert_eq!(
        display_tab_label(&format!("{regular_boundary}界"), false),
        format!("{regular_boundary}...")
    );
}

#[test]
fn dark_theme_detection_matches_builtin_themes() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn add_button_style_only_highlights_hovered_status() {
    let dark_active = add_button_style(&Theme::Dark, button::Status::Active);
    let dark_hovered = add_button_style(&Theme::Dark, button::Status::Hovered);
    let light_hovered = add_button_style(&Theme::Light, button::Status::Hovered);

    assert!(dark_active.background.is_none());
    assert_eq!(dark_active.text_color, Theme::Dark.palette().text);
    assert_eq!(dark_active.border.radius.top_left, 999.0);
    assert_eq!(
        background_color(dark_hovered.background),
        Theme::Dark.extended_palette().background.strong.color.scale_alpha(0.8)
    );
    assert_eq!(
        background_color(light_hovered.background),
        Theme::Light.extended_palette().background.weak.color
    );
}

#[test]
fn tab_bar_style_switches_theme_border_and_project_visibility() {
    let dark = tab_bar_style(&Theme::Dark, false);
    let light_project = tab_bar_style(&Theme::Light, true);

    assert_eq!(
        background_color(dark.background),
        Theme::Dark.extended_palette().background.base.color
    );
    assert_eq!(dark.border.width, 1.0);
    assert_eq!(dark.border.color, Theme::Dark.extended_palette().background.strong.color);
    assert_eq!(light_project.border.width, 0.0);
    assert_eq!(light_project.border.color, Theme::Light.extended_palette().background.weak.color);
}

#[test]
fn tab_label_style_covers_active_hovered_and_idle_theme_branches() {
    let dark_active = tab_label_style(&Theme::Dark, true, false);
    let dark_hovered = tab_label_style(&Theme::Dark, false, true);
    let light_idle = tab_label_style(&Theme::Light, false, false);

    assert_eq!(
        background_color(dark_active.background),
        Theme::Dark.extended_palette().background.strong.color.scale_alpha(0.85)
    );
    assert_eq!(dark_active.text_color, Some(Theme::Dark.palette().text));
    assert_eq!(
        background_color(dark_hovered.background),
        Theme::Dark.extended_palette().background.weak.color.scale_alpha(0.7)
    );
    assert_eq!(dark_hovered.text_color, Some(Theme::Dark.palette().text.scale_alpha(0.78)));
    assert_eq!(background_color(light_idle.background), Color::TRANSPARENT);
    assert_eq!(light_idle.text_color, Some(Theme::Light.palette().text.scale_alpha(0.55)));
    assert_eq!(light_idle.border.radius.top_left, 6.0);
    assert_eq!(light_idle.border.radius.bottom_left, 0.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn close_corner_triangle_draws_valid_bounds_and_handles_zero_width() {
    let renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("tiny-skia headless renderer should be available");
    let triangle = CloseCornerTriangle;

    let empty = triangle.draw(
        &(),
        &renderer,
        &Theme::Dark,
        Rectangle::new(iced::Point::ORIGIN, iced::Size::new(0.0, 18.0)),
        iced::mouse::Cursor::Unavailable,
    );
    let drawn = triangle.draw(
        &(),
        &renderer,
        &Theme::Dark,
        Rectangle::new(iced::Point::ORIGIN, iced::Size::new(18.0, 18.0)),
        iced::mouse::Cursor::Unavailable,
    );

    assert_eq!(empty.len(), 1);
    assert_eq!(drawn.len(), 1);
}
