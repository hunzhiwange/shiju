use super::header_tabs::*;
use iced::{Background, Color, Point, Theme};
use iced::widget::button;
fn style_background(style: &button::Style) -> Option<Color> {
    match &style.background {
        Some(Background::Color(color)) => Some(*color),
        None => None,
        _ => panic!("expected a solid color background"),
    }
}

#[test]
fn header_tabs_height_tracks_scrollbar_visibility() {
    assert_eq!(header_tabs_height(false), 30.0);
    assert_eq!(header_tabs_height(true), 36.0);
}
#[test]
fn tab_selection_matches_only_the_active_path() {
    assert!(tab_is_selected(Some("/tmp/a.rs"), "/tmp/a.rs"));
    assert!(!tab_is_selected(Some("/tmp/b.rs"), "/tmp/a.rs"));
    assert!(!tab_is_selected(None, "/tmp/a.rs"));
}
#[test]
fn tab_context_menu_matches_only_the_target_path() {
    assert!(tab_context_menu_matches(Some("/tmp/a.rs"), "/tmp/a.rs"));
    assert!(!tab_context_menu_matches(Some("/tmp/b.rs"), "/tmp/a.rs"));
    assert!(!tab_context_menu_matches(None, "/tmp/a.rs"));
}
#[test]
fn tab_menu_anchor_defaults_to_origin_and_preserves_position() {
    assert_eq!(tab_menu_anchor(None), Point::ORIGIN);
    assert_eq!(tab_menu_anchor(Some(Point::new(12.0, 24.0))), Point::new(12.0, 24.0));
}
#[test]
fn close_button_style_changes_background_by_status() {
    let theme = Theme::Light;
    let active = close_button_style(&theme, button::Status::Active);
    let hovered = close_button_style(&theme, button::Status::Hovered);
    let pressed = close_button_style(&theme, button::Status::Pressed);

    assert_eq!(style_background(&active), None);
    assert_eq!(style_background(&hovered).map(|color| color.a), Some(0.10));
    assert_eq!(style_background(&pressed).map(|color| color.a), Some(0.18));
    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.color, Color::TRANSPARENT);
}
#[test]
fn tab_button_style_handles_light_theme_selection_and_pointer_states() {
    let theme = Theme::Light;
    let unselected = tab_button_style(&theme, button::Status::Active, false);
    let selected = tab_button_style(&theme, button::Status::Active, true);
    let hovered = tab_button_style(&theme, button::Status::Hovered, false);
    let pressed = tab_button_style(&theme, button::Status::Pressed, false);

    assert_eq!(style_background(&unselected), None);
    assert_eq!(style_background(&selected).map(|color| color.a), Some(0.18));
    assert_eq!(style_background(&hovered).map(|color| color.a), Some(0.10));
    assert_eq!(style_background(&pressed), style_background(&selected));
    assert_eq!(selected.text_color, theme.palette().text);
    assert_eq!(selected.border.color, Color::TRANSPARENT);
}
#[test]
fn tab_button_style_uses_dark_theme_surface_tones() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();
    let selected = tab_button_style(&theme, button::Status::Active, true);
    let hovered = tab_button_style(&theme, button::Status::Hovered, false);

    assert_eq!(
        style_background(&selected),
        Some(palette.background.strong.color.scale_alpha(0.95))
    );
    assert_eq!(style_background(&hovered), Some(palette.background.strong.color.scale_alpha(0.8)));
    assert_ne!(style_background(&selected), style_background(&hovered));
}
