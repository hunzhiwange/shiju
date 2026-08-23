//! 验证明暗主题检测、颜色混合、四像素滚动条与图标按钮样式。
//! 阈值精确值按亮色处理，颜色插值比例会钳制，确保极端输入仍落在有效通道范围。

use super::theme::{
    additions_pill, change_pills, chat_scroll_direction, chat_secondary_muted_text_color,
    chat_secondary_subtle_text_color, chat_secondary_text_color, deletions_pill,
    eye_icon_button_style, eye_icon_svg_style, file_button_style, icon_button_style, icon_svg,
    is_dark_theme, mix_color, muted_icon_color, pill_button_style, simplified_block_style,
    simplified_code_block_style, weak_file_button_style,
};
use crate::app::Message;
use crate::app::assets::Icon;
use iced::widget::{button, scrollable, svg};
use iced::{Background, Color, Element, Theme, theme::Palette};

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn assert_radius(style: iced::widget::button::Style, expected: f32) {
    assert_eq!(style.border.radius.top_left, expected);
    assert_eq!(style.border.radius.top_right, expected);
    assert_eq!(style.border.radius.bottom_right, expected);
    assert_eq!(style.border.radius.bottom_left, expected);
}

fn assert_color_near(actual: Color, expected: Color) {
    assert!((actual.r - expected.r).abs() < f32::EPSILON);
    assert!((actual.g - expected.g).abs() < f32::EPSILON);
    assert!((actual.b - expected.b).abs() < f32::EPSILON);
    assert!((actual.a - expected.a).abs() < f32::EPSILON);
}

fn child_count(element: &Element<'_, Message>) -> usize {
    element.as_widget().children().len()
}

#[test]
fn theme_dark_detection_matches_builtin_themes() {
    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
}

#[test]
fn theme_dark_detection_treats_exact_rgb_sum_threshold_as_light() {
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
fn mix_color_clamps_and_interpolates_channels() {
    assert_eq!(mix_color(Color::BLACK, Color::WHITE, -1.0), Color::BLACK);
    assert_eq!(mix_color(Color::BLACK, Color::WHITE, 0.5), Color::from_rgba(0.5, 0.5, 0.5, 1.0));
    assert_eq!(mix_color(Color::BLACK, Color::WHITE, 2.0), Color::WHITE);

    let mixed =
        mix_color(Color::from_rgba(0.2, 0.4, 0.6, 0.8), Color::from_rgba(0.8, 0.2, 0.0, 0.4), 0.25);
    assert_color_near(mixed, Color::from_rgba(0.35, 0.35, 0.45, 0.7));
}

#[test]
fn secondary_and_icon_text_colors_follow_theme_branches() {
    assert_eq!(
        chat_secondary_text_color(&Theme::Dark),
        Theme::Dark.palette().text.scale_alpha(0.92)
    );
    assert_eq!(
        chat_secondary_text_color(&Theme::Light),
        Theme::Light.extended_palette().secondary.base.text.scale_alpha(0.90)
    );
    assert_eq!(
        chat_secondary_muted_text_color(&Theme::Dark),
        Theme::Dark.palette().text.scale_alpha(0.72)
    );
    assert_eq!(
        chat_secondary_muted_text_color(&Theme::Light),
        Theme::Light.extended_palette().secondary.base.text.scale_alpha(0.72)
    );
    assert_eq!(
        chat_secondary_subtle_text_color(&Theme::Dark),
        Theme::Dark.palette().text.scale_alpha(0.56)
    );
    assert_eq!(
        chat_secondary_subtle_text_color(&Theme::Light),
        Theme::Light.extended_palette().secondary.base.text.scale_alpha(0.58)
    );
    assert_eq!(muted_icon_color(&Theme::Dark), Theme::Dark.palette().text.scale_alpha(0.75));
    assert_eq!(
        muted_icon_color(&Theme::Light),
        Theme::Light.extended_palette().secondary.base.text.scale_alpha(0.85)
    );
}

#[test]
fn chat_scroll_direction_uses_vertical_four_pixel_scrollbar() {
    let direction = chat_scroll_direction();

    assert!(matches!(direction, scrollable::Direction::Vertical(_)));
    assert!(direction.horizontal().is_none());
    assert_eq!(
        format!("{:?}", direction.vertical().expect("vertical scrollbar")),
        "Scrollbar { width: 4.0, margin: 0.0, scroller_width: 4.0, alignment: Start, spacing: None }"
    );
}

#[test]
fn icon_svg_builds_static_icon_widget() {
    let _ = icon_svg(Icon::Eye);
}

#[test]
fn icon_button_style_covers_light_and_dark_statuses() {
    let dark_active = icon_button_style(&Theme::Dark, button::Status::Active);
    let dark_hovered = icon_button_style(&Theme::Dark, button::Status::Hovered);
    let dark_pressed = icon_button_style(&Theme::Dark, button::Status::Pressed);
    let light_active = icon_button_style(&Theme::Light, button::Status::Active);
    let light_hovered = icon_button_style(&Theme::Light, button::Status::Hovered);
    let light_pressed = icon_button_style(&Theme::Light, button::Status::Pressed);

    assert_eq!(background_color(dark_active.background), Color::from_rgba8(24, 25, 29, 0.88));
    assert_eq!(background_color(dark_hovered.background), Color::from_rgba8(31, 33, 38, 0.94));
    assert_eq!(background_color(dark_pressed.background), Color::from_rgba8(36, 38, 44, 0.98));
    assert_eq!(dark_active.border.color, Color::from_rgba8(45, 48, 54, 0.9));
    assert_eq!(dark_hovered.border.color, Color::from_rgba8(63, 67, 75, 0.94));
    assert_eq!(background_color(light_active.background), Color::from_rgba8(247, 248, 250, 1.0));
    assert_eq!(background_color(light_hovered.background), Color::from_rgba8(240, 243, 246, 1.0));
    assert_eq!(background_color(light_pressed.background), Color::from_rgba8(232, 236, 241, 1.0));
    assert_eq!(light_active.border.color, Color::from_rgba8(226, 231, 237, 1.0));
    assert_eq!(light_pressed.border.color, Color::from_rgba8(210, 216, 224, 1.0));
    assert_eq!(dark_pressed.text_color, chat_secondary_text_color(&Theme::Dark));
    assert_radius(light_pressed, 999.0);
}

#[test]
fn eye_icon_styles_keep_transparent_button_and_theme_svg_color() {
    let dark_button = eye_icon_button_style(&Theme::Dark, button::Status::Pressed);
    let light_button = eye_icon_button_style(&Theme::Light, button::Status::Hovered);

    assert!(dark_button.background.is_none());
    assert_eq!(dark_button.border.width, 0.0);
    assert_eq!(dark_button.border.color, Color::TRANSPARENT);
    assert_eq!(light_button.text_color, chat_secondary_text_color(&Theme::Light));
    assert_radius(dark_button, 999.0);
    assert_eq!(
        eye_icon_svg_style(&Theme::Dark, svg::Status::Idle).color,
        Some(Theme::Dark.palette().text.scale_alpha(0.92))
    );
    assert_eq!(
        eye_icon_svg_style(&Theme::Light, svg::Status::Hovered).color,
        Some(Theme::Light.extended_palette().secondary.base.text.scale_alpha(0.90))
    );
}

#[test]
fn simplified_container_styles_cover_empty_and_code_blocks() {
    let empty = simplified_block_style(&Theme::Dark);
    let dark_code = simplified_code_block_style(&Theme::Dark);
    let light_code = simplified_code_block_style(&Theme::Light);

    assert!(empty.background.is_none());
    assert_eq!(empty.border.width, 0.0);
    assert_eq!(empty.border.color, Color::TRANSPARENT);
    assert_eq!(empty.border.radius.top_left, 0.0);
    assert_eq!(background_color(dark_code.background), Color::from_rgba8(24, 25, 29, 0.90));
    assert_eq!(dark_code.border.color, Color::from_rgba8(45, 48, 54, 0.90));
    assert_eq!(background_color(light_code.background), Color::from_rgba8(247, 248, 250, 1.0));
    assert_eq!(light_code.border.color, Color::from_rgba8(226, 231, 237, 1.0));
    assert_eq!(dark_code.border.radius.top_left, 14.0);
}

#[test]
fn file_and_pill_button_styles_preserve_expected_radius() {
    assert_radius(pill_button_style(&Theme::Light, button::Status::Active), 999.0);
    assert_radius(file_button_style(&Theme::Dark, button::Status::Hovered), 10.0);
}

#[test]
fn weak_file_button_style_covers_status_and_theme_backgrounds() {
    let dark_pressed = weak_file_button_style(&Theme::Dark, button::Status::Pressed);
    let light_pressed = weak_file_button_style(&Theme::Light, button::Status::Pressed);
    let dark_hovered = weak_file_button_style(&Theme::Dark, button::Status::Hovered);
    let light_hovered = weak_file_button_style(&Theme::Light, button::Status::Hovered);
    let active = weak_file_button_style(&Theme::Light, button::Status::Active);
    let disabled = weak_file_button_style(&Theme::Dark, button::Status::Disabled);

    assert_eq!(
        background_color(dark_pressed.background),
        Theme::Dark.extended_palette().background.strong.color.scale_alpha(0.22)
    );
    assert_eq!(
        background_color(light_pressed.background),
        Theme::Light.extended_palette().background.strong.color.scale_alpha(0.16)
    );
    assert_eq!(
        background_color(dark_hovered.background),
        Theme::Dark.extended_palette().background.weak.color.scale_alpha(0.28)
    );
    assert_eq!(
        background_color(light_hovered.background),
        Theme::Light.extended_palette().background.weak.color.scale_alpha(0.35)
    );
    assert!(active.background.is_none());
    assert!(disabled.background.is_none());
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert_eq!(disabled.text_color, chat_secondary_muted_text_color(&Theme::Dark));
    assert_eq!(active.text_color, chat_secondary_muted_text_color(&Theme::Light));
}

#[test]
fn change_pill_elements_build_expected_widget_trees() {
    let additions = additions_pill(3);
    let deletions = deletions_pill(2);
    let changes = change_pills(3, 2);

    assert_eq!(child_count(&additions), 0);
    assert_eq!(child_count(&deletions), 0);
    assert_eq!(child_count(&changes), 2);
}
