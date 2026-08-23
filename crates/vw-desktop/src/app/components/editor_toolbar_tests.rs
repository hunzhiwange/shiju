//! 验证编辑器工具栏在不同页面、保存状态和主题下的按钮结构与样式。
//! 禁用按钮沿用空闲色，防止仅改变可点击性时引入低对比度或误导性高亮。

use super::editor_toolbar::{icon_button, icon_button_style, icon_svg, tooltip_content_style};
use crate::app::{App, Message, Screen, assets::Icon};
use iced::widget::tooltip::Position;
use iced::{Background, Color};

#[test]
fn toolbar_icon_svg_and_button_build() {
    let _ = icon_svg(Icon::Save);
    let _ = icon_button(Icon::Save, "保存", Position::Top, Message::PreviewLspTick, false);
    let _ = icon_button(Icon::Save, "保存", Position::Bottom, Message::PreviewLspTick, true);
}

#[test]
fn toolbar_icon_svg_with_various_icons() {
    let _ = icon_svg(Icon::ArrowClockwise);
    let _ = icon_svg(Icon::ArrowCounterClockwise);
    let _ = icon_svg(Icon::Search);
    let _ = icon_svg(Icon::ArrowRepeat);
}

#[test]
fn toolbar_view_handles_project_and_non_project_screens() {
    let mut app = App::new().0;
    let _ = super::editor_toolbar::view(&app, Some(Message::PreviewLspTick), true);

    app.screen = Screen::Project;
    let _ = super::editor_toolbar::view(&app, None, false);
}

#[test]
fn toolbar_view_without_save_on_normal_screen() {
    let app = App::new().0;
    let _ = super::editor_toolbar::view(&app, None, true);
    let _ = super::editor_toolbar::view(&app, None, false);
}

#[test]
fn toolbar_view_with_save_and_not_dirty() {
    let app = App::new().0;
    let _ = super::editor_toolbar::view(&app, Some(Message::PreviewLspTick), false);
}

#[test]
fn toolbar_icon_button_style_highlighted() {
    let theme = iced::Theme::Light;
    let p = theme.palette().primary;

    for status in [
        iced::widget::button::Status::Active,
        iced::widget::button::Status::Hovered,
        iced::widget::button::Status::Pressed,
    ] {
        let style = icon_button_style(&theme, status, true);
        assert_eq!(style.border.color, p);
        assert!(style.background.is_some());
    }
}

#[test]
fn toolbar_icon_button_style_non_highlighted() {
    let theme = iced::Theme::Light;
    let gray = Color::from_rgba8(225, 225, 225, 1.0);

    for status in [
        iced::widget::button::Status::Active,
        iced::widget::button::Status::Hovered,
        iced::widget::button::Status::Pressed,
    ] {
        let style = icon_button_style(&theme, status, false);
        assert_eq!(style.border.color, gray);
        assert!(style.background.is_some());
    }
}

#[test]
fn toolbar_tooltip_content_style_is_valid() {
    let style = tooltip_content_style();
    assert!(style.background.is_some());
    assert_eq!(style.border.width, 1.0);
    assert_eq!(style.text_color, None);
    assert!(!style.snap);
}

#[test]
fn toolbar_icon_button_style_disabled_uses_idle_colors_for_both_themes() {
    for theme in [iced::Theme::Light, iced::Theme::Dark] {
        let primary = theme.palette().primary;
        let highlighted = icon_button_style(&theme, iced::widget::button::Status::Disabled, true);
        let regular = icon_button_style(&theme, iced::widget::button::Status::Disabled, false);

        assert_eq!(
            highlighted.background,
            Some(Background::Color(Color::from_rgba(primary.r, primary.g, primary.b, 0.12,)))
        );
        assert_eq!(highlighted.border.color, primary);
        assert_eq!(
            regular.background,
            Some(Background::Color(Color::from_rgba8(250, 250, 250, 1.0)))
        );
        assert_eq!(regular.border.color, Color::from_rgba8(225, 225, 225, 1.0));
    }
}

#[test]
fn toolbar_tooltip_content_style_keeps_expected_surface_contract() {
    let style = tooltip_content_style();

    assert_eq!(style.background, Some(Background::Color(Color::from_rgba8(245, 245, 245, 1.0))));
    assert_eq!(style.border.color, Color::from_rgba8(210, 210, 210, 1.0));
    assert_eq!(style.border.radius.top_left, 8.0);
    assert_eq!(style.shadow, iced::Shadow::default());
}
