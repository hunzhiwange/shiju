//! 验证提示消息的类型样式、关闭操作和内容布局。
//!
//! 提示夹具遍历消息种类与关闭状态，并检查暗色主题下的容器和文本样式。

use super::toast::{
    cache_sync_retry_message, confirm_dialog, icon_svg, is_dark_theme, toast_card_style,
    toast_icon_badge_style, toast_message_text_color, toast_palette, view,
};
use crate::app::assets::Icon;
use crate::app::message::SkillHubMessage;
use crate::app::skill_hub::SkillCacheSyncState;
use crate::app::state::{Toast, ToastKind};
use crate::app::{App, Message};
use iced::{Background, Color, Element, Theme};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn assert_color_close(actual: Color, expected: Color) {
    let epsilon = 0.001;
    assert!((actual.r - expected.r).abs() < epsilon, "red channel {actual:?} != {expected:?}");
    assert!((actual.g - expected.g).abs() < epsilon, "green channel {actual:?} != {expected:?}");
    assert!((actual.b - expected.b).abs() < epsilon, "blue channel {actual:?} != {expected:?}");
    assert!((actual.a - expected.a).abs() < epsilon, "alpha channel {actual:?} != {expected:?}");
}

fn style_background_color(style: iced::widget::container::Style) -> Color {
    match style.background.expect("style should define background") {
        Background::Color(color) => color,
        other => panic!("expected solid color background, got {other:?}"),
    }
}

#[test]
fn toast_palette_maps_each_kind_to_expected_icon_colors_and_title() {
    let cases = [
        (
            ToastKind::Success,
            Icon::Check,
            Color::from_rgb8(0x28, 0x8F, 0x61),
            Color::from_rgb8(0xE8, 0xF7, 0xEF),
            "操作已完成",
        ),
        (
            ToastKind::Info,
            Icon::QuestionCircle,
            Color::from_rgb8(0x2A, 0x6F, 0xC2),
            Color::from_rgb8(0xE8, 0xF1, 0xFE),
            "提示",
        ),
        (
            ToastKind::Warning,
            Icon::QuestionCircle,
            Color::from_rgb8(0xB5, 0x7A, 0x00),
            Color::from_rgb8(0xFF, 0xF4, 0xD6),
            "请注意",
        ),
        (
            ToastKind::Error,
            Icon::X,
            Color::from_rgb8(0xC5, 0x3E, 0x3E),
            Color::from_rgb8(0xFE, 0xEA, 0xEA),
            "操作失败",
        ),
    ];

    for (kind, icon, accent, background, title) in cases {
        let palette = toast_palette(kind);
        assert_eq!(palette.icon, icon);
        assert_color_close(palette.accent, accent);
        assert_color_close(palette.background, background);
        assert_eq!(palette.title, title);
    }
}

#[test]
fn theme_detection_distinguishes_dark_and_light_palettes() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn icon_svg_constructs_static_svg_with_requested_icon() {
    let svg = icon_svg(Icon::Check, 14.0, Color::from_rgb8(0x28, 0x8F, 0x61));

    std::hint::black_box(svg);
}

#[test]
fn toast_badge_style_adjusts_alpha_for_theme() {
    let accent = Color::from_rgb8(0x2A, 0x6F, 0xC2);

    let light = toast_icon_badge_style(&Theme::Light, accent);
    assert_color_close(style_background_color(light), accent.scale_alpha(0.12));

    let dark = toast_icon_badge_style(&Theme::Dark, accent);
    assert_color_close(style_background_color(dark), accent.scale_alpha(0.18));
    assert_color_close(dark.border.color, accent.scale_alpha(0.36));
    assert_eq!(dark.border.width, 1.0);
}

#[test]
fn toast_badge_style_preserves_circular_border_geometry_across_themes() {
    let accent = Color::from_rgb8(0x2A, 0x6F, 0xC2);

    for theme in [Theme::Light, Theme::Dark] {
        let style = toast_icon_badge_style(&theme, accent);

        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius.top_left, 999.0);
        assert_eq!(style.border.radius.top_right, 999.0);
        assert_eq!(style.border.radius.bottom_right, 999.0);
        assert_eq!(style.border.radius.bottom_left, 999.0);
    }
}

#[test]
fn toast_message_text_color_uses_theme_specific_alpha() {
    assert_color_close(
        toast_message_text_color(&Theme::Light),
        Theme::Light.palette().text.scale_alpha(0.88),
    );
    assert_color_close(
        toast_message_text_color(&Theme::Dark),
        Theme::Dark.extended_palette().background.base.text.scale_alpha(0.94),
    );
}

#[test]
fn toast_card_style_uses_theme_specific_background_border_and_shadow() {
    let palette = toast_palette(ToastKind::Warning);

    let light = toast_card_style(&Theme::Light, palette.accent, palette.background);
    assert_color_close(style_background_color(light), palette.background);
    assert_color_close(light.border.color, palette.accent.scale_alpha(0.22));
    assert_color_close(light.shadow.color, Color::BLACK.scale_alpha(0.08));
    assert_eq!(light.shadow.offset.x, 0.0);
    assert_eq!(light.shadow.offset.y, 14.0);
    assert_eq!(light.shadow.blur_radius, 26.0);

    let dark = toast_card_style(&Theme::Dark, palette.accent, palette.background);
    assert_color_close(style_background_color(dark), palette.background.scale_alpha(0.22));
    assert_color_close(dark.border.color, palette.accent.scale_alpha(0.54));
    assert_color_close(dark.shadow.color, Color::BLACK.scale_alpha(0.18));
}

#[test]
fn toast_card_style_preserves_border_and_shadow_geometry_across_themes() {
    let palette = toast_palette(ToastKind::Info);

    for theme in [Theme::Light, Theme::Dark] {
        let style = toast_card_style(&theme, palette.accent, palette.background);

        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius.top_left, 12.0);
        assert_eq!(style.border.radius.top_right, 12.0);
        assert_eq!(style.border.radius.bottom_right, 12.0);
        assert_eq!(style.border.radius.bottom_left, 12.0);
        assert_eq!(style.shadow.offset.x, 0.0);
        assert_eq!(style.shadow.offset.y, 14.0);
        assert_eq!(style.shadow.blur_radius, 26.0);
    }
}

#[test]
fn view_returns_empty_element_without_active_toast() {
    let app = test_app();

    keep_element(view(&app));
}

#[test]
fn view_renders_active_toast_for_each_kind() {
    for kind in [ToastKind::Success, ToastKind::Info, ToastKind::Warning, ToastKind::Error] {
        let mut app = test_app();
        app.active_toast = Some(Toast { id: 7, kind, message: format!("toast {kind:?}") });

        keep_element(view(&app));
    }
}

#[test]
fn cache_sync_pending_retry_exposes_retry_action_in_toast() {
    let mut app = test_app();
    app.expert_hub.cache_sync =
        SkillCacheSyncState::PendingRetry { skill_id: 7, refresh_detail: false };
    app.active_toast = Some(Toast {
        id: 7,
        kind: ToastKind::Warning,
        message: "服务端已更新，本地缓存待同步".to_string(),
    });

    assert!(matches!(
        cache_sync_retry_message(&app),
        Some(Message::SkillHub(SkillHubMessage::RetrySkillCacheSync))
    ));
    keep_element(view(&app));
}

#[test]
fn confirm_dialog_constructs_overlay_with_actions() {
    keep_element(confirm_dialog(
        "删除项目",
        "该操作不可撤销",
        "删除",
        "取消",
        Message::None,
        Message::CloseError,
    ));
}
