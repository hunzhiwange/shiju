use iced::widget::button;
use iced::{Background, Border, Color, Theme};
fn is_dark_theme(theme: &Theme) -> bool {
    theme.palette().background.r + theme.palette().background.g + theme.palette().background.b < 1.5
}

pub fn square_icon_button_style(
    theme: &Theme,
    status: button::Status,
    enabled: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let text = theme.palette().text;
    button::Style {
        background: match status {
            button::Status::Hovered => Some(text.scale_alpha(0.09).into()),
            button::Status::Pressed => Some(text.scale_alpha(0.14).into()),
            _ => None,
        },
        border: Border { radius: 8.0.into(), ..Border::default() },
        text_color: if enabled {
            selector_text_color(theme, false)
        } else {
            text.scale_alpha(if is_dark { 0.36 } else { 0.42 })
        },
        ..Default::default()
    }
}

pub fn selector_label_font() -> iced::Font {
    iced::Font { weight: iced::font::Weight::Medium, ..Default::default() }
}

pub fn selector_text_color(theme: &Theme, highlighted: bool) -> Color {
    if highlighted {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.97 } else { 0.95 })
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.92 } else { 0.88 })
    }
}

pub fn selector_chevron_color(theme: &Theme, highlighted: bool) -> Color {
    if highlighted {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.72 } else { 0.68 })
    } else {
        theme.palette().text.scale_alpha(if is_dark_theme(theme) { 0.58 } else { 0.62 })
    }
}

pub fn selector_pill_button_style(
    theme: &Theme,
    status: button::Status,
    highlighted: bool,
) -> button::Style {
    let palette = theme.palette();
    let background = match status {
        button::Status::Pressed => Some(palette.text.scale_alpha(0.15)),
        button::Status::Hovered => Some(palette.text.scale_alpha(0.10)),
        _ if highlighted => Some(palette.primary.scale_alpha(0.12)),
        _ => None,
    };

    button::Style {
        background: background.map(Background::Color),
        border: Border { radius: 999.0.into(), ..Border::default() },
        text_color: selector_text_color(theme, highlighted),
        ..Default::default()
    }
}

pub use desktop_chat_tools::selector_style::selectable_list_button_style;

pub use desktop_chat_tools::selector_style::popover_style;

pub use desktop_ui_controls::tooltip::tooltip_dark_style;
