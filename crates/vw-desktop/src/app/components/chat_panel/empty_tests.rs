//! 验证空会话占位内容、工作区标题及明暗主题胶囊样式。
//! 亮度阈值两侧和恰好位于阈值的主题 fixture 用于锁定严格比较语义。

use super::{
    empty_session_placeholder, empty_session_title, is_dark_theme, placeholder_chip_style,
    session_loading_placeholder,
};
use crate::app::{App, state::WorkspaceMode};
use iced::{Background, Color, Theme, theme::Palette};

#[test]
fn empty_session_title_matches_workspace_mode() {
    assert_eq!(empty_session_title(WorkspaceMode::Code), "识矩软件智能体");
    assert_eq!(empty_session_title(WorkspaceMode::Work), "您的专属 AI 工作助理");
}

#[test]
fn is_dark_theme_detects_dark_mode() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn placeholder_chip_style_light_theme_has_expected_structure() {
    let style = placeholder_chip_style(&Theme::Light);
    assert!(style.background.is_some());
    assert_eq!(style.border.width, 1.0);
}

#[test]
fn placeholder_chip_style_dark_theme_has_expected_structure() {
    let style = placeholder_chip_style(&Theme::Dark);
    assert!(style.background.is_some());
    assert_eq!(style.border.width, 1.0);
}

#[test]
fn placeholder_chip_style_preserves_pill_radius_and_theme_surfaces() {
    let light = placeholder_chip_style(&Theme::Light);
    let dark = placeholder_chip_style(&Theme::Dark);

    assert_eq!(light.background, Some(Background::Color(Color::from_rgba8(247, 248, 250, 1.0))));
    assert_eq!(dark.background, Some(Background::Color(Color::from_rgba8(24, 25, 29, 0.92))));
    assert_eq!(light.border.radius, 999.0.into());
    assert_eq!(dark.border.radius, 999.0.into());
}

#[test]
fn placeholder_chip_style_keeps_lightweight_defaults() {
    let style = placeholder_chip_style(&Theme::Light);

    assert_eq!(style.text_color, None);
    assert_eq!(style.shadow, iced::Shadow::default());
}

#[test]
fn dark_theme_threshold_is_strictly_below_half_rgb_sum() {
    let at_threshold = Theme::custom(
        "threshold",
        Palette { background: Color::from_rgb(0.5, 0.5, 0.5), ..Palette::LIGHT },
    );
    let below_threshold = Theme::custom(
        "below-threshold",
        Palette { background: Color::from_rgb(0.49, 0.5, 0.5), ..Palette::LIGHT },
    );

    assert!(!is_dark_theme(&at_threshold));
    assert!(is_dark_theme(&below_threshold));
}

#[test]
fn session_loading_placeholder_produces_element_with_default_app() {
    let app = App::new().0;
    let _element = session_loading_placeholder(&app);
}

#[test]
fn session_loading_placeholder_produces_element_with_context() {
    let mut app = App::new().0;
    app.active_session_id = Some("test-id".into());
    app.project_path = Some("/project".into());
    app.selected_branch = Some("main".into());
    app.project_updated_at_ms = Some(1_000_000);
    let _element = session_loading_placeholder(&app);
}

#[test]
fn empty_session_placeholder_produces_element_with_default_app() {
    let app = App::new().0;
    let _element = empty_session_placeholder(&app);
}

#[test]
fn empty_session_placeholder_produces_element_with_context() {
    let mut app = App::new().0;
    app.project_path = Some("/project".into());
    app.selected_branch = Some("develop".into());
    app.project_updated_at_ms = Some(2_000_000);
    let _element = empty_session_placeholder(&app);
}
