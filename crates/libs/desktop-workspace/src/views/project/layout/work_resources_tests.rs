use iced::Theme;
use desktop_assets::assets::Icon;
use super::{content_panel_style, panel_style, resource_header_icons};

#[test]
fn work_resource_surface_uses_code_panel_border_in_both_themes() {
    for theme in [Theme::Dark, Theme::Light] {
        assert_eq!(panel_style(&theme).border, content_panel_style(&theme).border);
    }
}

#[test]
fn work_resource_header_uses_direction_and_fullscreen_icons() {
    assert_eq!(
        resource_header_icons(false),
        [Icon::ArrowLeft, Icon::ArrowRight, Icon::LayoutTextWindow, Icon::Fullscreen]
    );
    assert_eq!(
        resource_header_icons(true),
        [Icon::ArrowLeft, Icon::ArrowRight, Icon::LayoutTextWindow, Icon::FullscreenExit]
    );
}
