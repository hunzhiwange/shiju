use iced::Theme;
use desktop_assets::assets::Icon;
use super::{panel_style,resource_header_icons,ResourceBodyKind};
#[test]
fn browser_resource_surface_has_no_panel_border() {
    let browser = panel_style(&Theme::Dark, ResourceBodyKind::Browser);
    let overview = panel_style(&Theme::Dark, ResourceBodyKind::Overview);

    assert_eq!(browser.border.width, 0.0);
    assert_eq!(browser.border.radius, 0.0.into());
    assert!(overview.border.width > 0.0);
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
