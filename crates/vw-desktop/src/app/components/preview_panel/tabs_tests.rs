use super::tabs::{
    DisplayLanguage, all_languages, build_preview_tab_menu, preview_tab_menu_button_style,
    preview_tab_menu_container_style, preview_tab_menu_is_dark, preview_tab_menu_messages,
    preview_tab_menu_separator_style,
};
use crate::app::{Message, message::PreviewMessage};
use iced::widget::button;
use iced::{Background, Color, Theme, Vector};
use iced_code_editor::i18n::Language;

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn assert_color_close(actual: Color, expected: Color) {
    assert!((actual.r - expected.r).abs() < 0.0001, "red: {actual:?} != {expected:?}");
    assert!((actual.g - expected.g).abs() < 0.0001, "green: {actual:?} != {expected:?}");
    assert!((actual.b - expected.b).abs() < 0.0001, "blue: {actual:?} != {expected:?}");
    assert!((actual.a - expected.a).abs() < 0.0001, "alpha: {actual:?} != {expected:?}");
}

#[test]
fn all_languages_keep_stable_order_and_labels() {
    let labels = all_languages().iter().map(ToString::to_string).collect::<Vec<_>>();

    assert_eq!(labels.first().map(String::as_str), Some("English"));
    assert_eq!(
        labels,
        vec![
            "English",
            "简体中文",
            "Français",
            "Español",
            "Deutsch",
            "Italiano",
            "Português (BR)",
            "Português (PT)",
        ]
    );
}

#[test]
fn display_language_labels_match_editor_languages() {
    let cases = [
        (Language::English, "English"),
        (Language::ChineseSimplified, "简体中文"),
        (Language::French, "Français"),
        (Language::Spanish, "Español"),
        (Language::German, "Deutsch"),
        (Language::Italian, "Italiano"),
        (Language::PortugueseBR, "Português (BR)"),
        (Language::PortuguesePT, "Português (PT)"),
    ];

    for (language, label) in cases {
        assert_eq!(DisplayLanguage(language).to_string(), label);
    }
}

#[test]
fn display_language_is_copyable_and_comparable() {
    let language = DisplayLanguage(Language::English);
    let copied = language;

    assert_eq!(copied, language);
    assert!(format!("{copied:?}").contains("English"));
}

#[test]
fn preview_tab_menu_builds_for_owned_path_messages() {
    let menu = build_preview_tab_menu("/tmp/src/main.rs");
    let element: iced::Element<'_, Message> = menu;

    std::hint::black_box(element);
}

#[test]
fn preview_tab_menu_keeps_four_actions_and_two_separators() {
    let menu = build_preview_tab_menu("项目/含 空格.rs");
    let children = menu.as_widget().children();

    assert_eq!(children.len(), 6);
}

#[test]
fn preview_tab_menu_messages_preserve_path_and_action_order() {
    let [close, close_left, close_right, close_all] = preview_tab_menu_messages("项目/含 空格.rs");

    assert!(matches!(
        close,
        Message::Preview(PreviewMessage::Close(path)) if path == "项目/含 空格.rs"
    ));
    assert!(matches!(
        close_left,
        Message::Preview(PreviewMessage::TabMenuCloseLeft(path)) if path == "项目/含 空格.rs"
    ));
    assert!(matches!(
        close_right,
        Message::Preview(PreviewMessage::TabMenuCloseRight(path)) if path == "项目/含 空格.rs"
    ));
    assert!(matches!(close_all, Message::Preview(PreviewMessage::TabMenuCloseAll)));
}

#[test]
fn preview_tab_menu_button_style_maps_all_interaction_states() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let active = preview_tab_menu_button_style(&theme, button::Status::Active);
        let disabled = preview_tab_menu_button_style(&theme, button::Status::Disabled);
        let hovered = preview_tab_menu_button_style(&theme, button::Status::Hovered);
        let pressed = preview_tab_menu_button_style(&theme, button::Status::Pressed);

        assert_eq!(active.background, Some(Background::Color(Color::TRANSPARENT)));
        assert_eq!(disabled.background, Some(Background::Color(Color::TRANSPARENT)));
        assert_eq!(hovered.background, Some(Background::Color(palette.background.weak.color)));
        assert_eq!(pressed.background, Some(Background::Color(palette.background.strong.color)));
        assert_eq!(active.text_color, theme.palette().text);
        assert_eq!(active.border.radius, 4.0.into());
    }
}

#[test]
fn preview_tab_menu_dark_detection_uses_strict_rgb_sum_threshold() {
    let mut threshold_palette = iced::theme::Palette::DARK;
    threshold_palette.background = Color::from_rgb(0.5, 0.5, 0.5);
    let threshold = Theme::custom("threshold", threshold_palette);

    let mut below_palette = iced::theme::Palette::DARK;
    below_palette.background = Color::from_rgb(0.499, 0.5, 0.5);
    let below = Theme::custom("below-threshold", below_palette);

    assert!(!preview_tab_menu_is_dark(&threshold));
    assert!(preview_tab_menu_is_dark(&below));
    assert!(!preview_tab_menu_is_dark(&Theme::Light));
    assert!(preview_tab_menu_is_dark(&Theme::Dark));
}

#[test]
fn preview_tab_menu_separator_style_adapts_to_light_and_dark_themes() {
    let light = Theme::Light;
    let dark = Theme::Dark;
    let light_style = preview_tab_menu_separator_style(&light);
    let dark_style = preview_tab_menu_separator_style(&dark);
    let dark_text = dark.palette().text;

    assert_eq!(
        light_style.background,
        Some(Background::Color(light.extended_palette().background.strong.color))
    );
    assert_color_close(
        background_color(dark_style.background),
        Color::from_rgba(dark_text.r, dark_text.g, dark_text.b, 0.15),
    );
}

#[test]
fn preview_tab_menu_container_style_keeps_theme_surface_and_geometry() {
    for theme in [Theme::Light, Theme::Dark] {
        let palette = theme.extended_palette();
        let is_dark = preview_tab_menu_is_dark(&theme);
        let style = preview_tab_menu_container_style(&theme);

        assert_eq!(
            style.background,
            Some(Background::Color(if is_dark {
                palette.background.weak.color
            } else {
                palette.background.base.color
            }))
        );
        assert_eq!(
            style.border.color,
            if is_dark {
                palette.background.weak.color.scale_alpha(0.65)
            } else {
                palette.background.strong.color
            }
        );
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius, 8.0.into());
        assert_color_close(
            style.shadow.color,
            Color::BLACK.scale_alpha(if is_dark { 0.30 } else { 0.10 }),
        );
        assert_eq!(style.shadow.offset, Vector::new(0.0, 4.0));
        assert_eq!(style.shadow.blur_radius, 12.0);
    }
}
