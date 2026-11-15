use super::menus::*;
use desktop_assets::assets::Icon;
use iced::{Background, Color, Theme};
use iced::widget::button;
#[test]
fn context_menu_container_style_uses_theme_background_and_border() {
    let style = context_menu_container_style(&Theme::Dark);

    let Some(Background::Color(background)) = style.background else {
        panic!("expected context menu background");
    };

    assert_eq!(background.a, 0.98);
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.border.color, Color::from_rgb8(0xCC, 0xCC, 0xCC));
}
#[test]
fn nav_popup_item_icon_distinguishes_directory_and_file() {
    assert_eq!(nav_popup_item_icon("src", true), Icon::FolderOpen);
    assert_eq!(nav_popup_item_icon("main.rs", false), Icon::Rust);
    assert_eq!(nav_popup_item_icon("archive.bin", false), Icon::Document);
}
#[test]
fn nav_popup_item_button_style_changes_background_by_status() {
    let theme = Theme::Light;
    let active = nav_popup_item_button_style(&theme, button::Status::Active);
    let hovered = nav_popup_item_button_style(&theme, button::Status::Hovered);
    let pressed = nav_popup_item_button_style(&theme, button::Status::Pressed);

    assert!(active.background.is_none());
    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert!(matches!(hovered.background, Some(Background::Color(_))));
    assert!(matches!(pressed.background, Some(Background::Color(_))));
    assert_ne!(hovered.background, pressed.background);
}
#[test]
fn nav_popup_container_style_uses_popup_surface_border_and_shadow() {
    let light = nav_popup_container_style(&Theme::Light);
    let dark = nav_popup_container_style(&Theme::Dark);

    assert!(matches!(light.background, Some(Background::Color(_))));
    assert_eq!(light.border.width, 1.0);
    assert_eq!(light.shadow.color, Color::BLACK);
    assert_eq!(light.shadow.offset, iced::Vector::new(0.0, 4.0));
    assert_eq!(light.shadow.blur_radius, 12.0);
    assert_ne!(light.background, dark.background);
}
