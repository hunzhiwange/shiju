use super::menu::{
    build_file_tree_menu, file_tree_menu_button_style, file_tree_menu_container_style,
    file_tree_menu_separator_style,
};
use crate::app::App;
use crate::app::state::{FileTreeClipboard, FileTreeClipboardMode};
use iced::widget::button;
use iced::{Background, Color, Element, Theme, Vector};

fn app_with_clipboard(mode: FileTreeClipboardMode) -> App {
    let mut app = App::new().0;
    app.file_tree_clipboard =
        Some(FileTreeClipboard { mode, src_path: "fixture/path".to_string() });
    app
}

fn menu_item_count(element: &Element<'_, crate::app::Message>) -> usize {
    let children = element.as_widget().children();
    children.len()
}

#[test]
fn file_tree_menu_without_optional_items_has_base_structure() {
    let app = App::new().0;

    let menu = build_file_tree_menu(&app, false);

    assert_eq!(menu_item_count(&menu), 15);
}

#[test]
fn file_tree_menu_find_option_adds_one_item() {
    let app = App::new().0;

    let without_find = build_file_tree_menu(&app, false);
    let with_find = build_file_tree_menu(&app, true);

    assert_eq!(menu_item_count(&without_find), 15);
    assert_eq!(menu_item_count(&with_find), 16);
}

#[test]
fn file_tree_menu_clipboard_adds_paste_for_copy_and_cut_modes() {
    let empty_app = App::new().0;
    let copy_app = app_with_clipboard(FileTreeClipboardMode::Copy);
    let cut_app = app_with_clipboard(FileTreeClipboardMode::Cut);

    let without_clipboard = build_file_tree_menu(&empty_app, false);
    let with_copy = build_file_tree_menu(&copy_app, false);
    let with_cut = build_file_tree_menu(&cut_app, false);

    assert_eq!(menu_item_count(&without_clipboard), 15);
    assert_eq!(menu_item_count(&with_copy), 16);
    assert_eq!(menu_item_count(&with_cut), 16);
}

#[test]
fn file_tree_menu_combines_find_and_paste_options() {
    let app = app_with_clipboard(FileTreeClipboardMode::Copy);

    let menu = build_file_tree_menu(&app, true);

    assert_eq!(menu_item_count(&menu), 17);
}

#[test]
fn file_tree_menu_button_style_maps_interaction_states_for_each_theme() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let active = file_tree_menu_button_style(&theme, button::Status::Active);
        let hovered = file_tree_menu_button_style(&theme, button::Status::Hovered);
        let pressed = file_tree_menu_button_style(&theme, button::Status::Pressed);

        assert_eq!(active.background, Some(Background::Color(Color::TRANSPARENT)));
        assert_eq!(hovered.background, Some(Background::Color(palette.background.weak.color)));
        assert_eq!(pressed.background, Some(Background::Color(palette.background.strong.color)));
        assert_eq!(active.text_color, theme.palette().text);
        assert_eq!(active.border.radius, 4.0.into());
    }
}

#[test]
fn file_tree_menu_separator_style_uses_strong_theme_background() {
    for theme in [Theme::Light, Theme::Dark] {
        let style = file_tree_menu_separator_style(&theme);

        assert_eq!(
            style.background,
            Some(Background::Color(theme.extended_palette().background.strong.color))
        );
    }
}

#[test]
fn file_tree_menu_container_style_keeps_theme_surface_and_geometry() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let style = file_tree_menu_container_style(&theme);

        assert_eq!(style.background, Some(Background::Color(palette.background.base.color)));
        assert_eq!(style.border.color, palette.background.strong.color);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius, 8.0.into());
        assert_eq!(style.shadow.color, Color::BLACK.scale_alpha(0.1));
        assert_eq!(style.shadow.offset, Vector::new(0.0, 4.0));
        assert_eq!(style.shadow.blur_radius, 12.0);
    }
}
