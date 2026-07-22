use crate::app::assets::Icon;
use crate::app::{App, Message};
use iced::widget::{button, svg, text};
use iced::{Background, Color, Length, Theme};

fn background_color(background: Option<Background>) -> Option<Color> {
    match background {
        Some(Background::Color(color)) => Some(color),
        Some(other) => panic!("expected solid color background, got {other:?}"),
        None => None,
    }
}

#[test]
fn settings_close_button_is_a_fixed_square_with_one_child() {
    let element = super::help::settings_close_button(Message::None);
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Fixed(28.0));
    assert_eq!(size.height, Length::Fixed(28.0));
    assert_eq!(element.as_widget().children().len(), 1);
}

#[test]
fn settings_help_button_uses_a_shrink_sized_tooltip_shell() {
    let element = super::help::settings_help_button(Message::GatewayHealthTick);
    let size = element.as_widget().size();

    assert_eq!(size.width, Length::Shrink);
    assert_eq!(size.height, Length::Shrink);
    assert_eq!(element.as_widget().children().len(), 2);
}

#[test]
fn settings_close_icon_style_uses_theme_specific_text_alpha() {
    let dark = super::help::settings_close_icon_style(&Theme::Dark, svg::Status::Idle);
    let light = super::help::settings_close_icon_style(&Theme::Light, svg::Status::Hovered);

    assert_eq!(dark.color, Some(Theme::Dark.palette().text.scale_alpha(0.72)));
    assert_eq!(light.color, Some(Theme::Light.palette().text.scale_alpha(0.82)));
}

#[test]
fn settings_close_button_style_covers_interaction_statuses() {
    let theme = Theme::Dark;
    let palette = theme.extended_palette();

    let active = super::help::settings_close_button_style(&theme, button::Status::Active);
    let hovered = super::help::settings_close_button_style(&theme, button::Status::Hovered);
    let pressed = super::help::settings_close_button_style(&theme, button::Status::Pressed);
    let disabled = super::help::settings_close_button_style(&theme, button::Status::Disabled);

    assert_eq!(background_color(active.background), None);
    assert_eq!(
        background_color(hovered.background),
        Some(palette.background.weak.color.scale_alpha(0.72))
    );
    assert_eq!(
        background_color(pressed.background),
        Some(palette.background.strong.color.scale_alpha(0.82))
    );
    assert_eq!(background_color(disabled.background), None);
    assert_eq!(active.text_color, theme.palette().text);
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert_eq!(active.border.radius, 999.0.into());
}

#[test]
fn settings_help_copy_state_marks_only_matching_content_as_copied() {
    let help_text = "帮助内容\n\n```json\n{\"enabled\":true}\n```";

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&help_text, &mut hasher);
    let matching_hash = std::hash::Hasher::finish(&hasher);

    assert_eq!(
        super::help::settings_help_copy_state(Some(matching_hash), help_text),
        (Icon::Check, "已复制")
    );
    assert_eq!(super::help::settings_help_copy_state(None, help_text), (Icon::Copy, "复制帮助"));
    assert_eq!(
        super::help::settings_help_copy_state(Some(matching_hash), "其他帮助内容"),
        (Icon::Copy, "复制帮助")
    );
}

#[test]
fn help_modal_fills_overlay_for_default_and_copied_states() {
    let mut app = App::new().0;
    let help_text = String::from("帮助内容");

    let default = super::help::with_settings_help_modal(
        &app,
        text("base").into(),
        "帮助",
        help_text.clone(),
        Message::GatewayHealthTick,
    );
    assert_eq!(default.as_widget().children().len(), 2);

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&help_text, &mut hasher);
    app.last_copied_code_hash = Some(std::hash::Hasher::finish(&hasher));

    let copied = super::help::with_settings_help_modal(
        &app,
        text("base").into(),
        "",
        help_text,
        Message::None,
    );
    assert_eq!(copied.as_widget().children().len(), 2);
}
