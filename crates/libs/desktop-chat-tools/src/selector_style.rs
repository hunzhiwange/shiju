use iced::{Background, Border, Color, Theme};
use iced::widget::{button,container, text};
use crate::card_style::is_dark_theme;
pub use desktop_ui_controls::tooltip::tooltip_dark_style;
pub fn selectable_list_button_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let is_dark = is_dark_theme(theme);
    let hovered = matches!(status, button::Status::Hovered);
    let pressed = matches!(status, button::Status::Pressed);
    let neutral_hover = if is_dark {
        Color::from_rgba8(31, 33, 38, 0.94)
    } else {
        Color::from_rgba8(241, 243, 246, 1.0)
    };
    let selected_bg = Color::from_rgba(
        theme.palette().primary.r,
        theme.palette().primary.g,
        theme.palette().primary.b,
        if is_dark { 0.15 } else { 0.10 },
    );
    let selected_hover_bg = Color::from_rgba(
        theme.palette().primary.r,
        theme.palette().primary.g,
        theme.palette().primary.b,
        if is_dark { 0.19 } else { 0.13 },
    );
    let background = if pressed {
        Some(Background::Color(if selected { selected_hover_bg } else { neutral_hover }))
    } else if hovered {
        Some(Background::Color(if selected { selected_hover_bg } else { neutral_hover }))
    } else if selected {
        Some(Background::Color(selected_bg))
    } else {
        None
    };
    let border_color = if selected {
        theme.palette().primary.scale_alpha(if is_dark { 0.42 } else { 0.24 })
    } else if hovered || pressed {
        if is_dark {
            Color::from_rgba8(58, 61, 69, 0.92)
        } else {
            Color::from_rgba8(218, 223, 230, 1.0)
        }
    } else {
        Color::TRANSPARENT
    };

    button::Style {
        background,
        border: Border {
            radius: 8.0.into(),
            width: if selected || hovered || pressed { 1.0 } else { 0.0 },
            color: border_color,
        },
        text_color: if selected {
            selector_text_color(theme, true)
        } else {
            theme.palette().text.scale_alpha(if is_dark { 0.90 } else { 0.88 })
        },
        ..Default::default()
    }
}
pub fn popover_style(theme: &Theme) -> container::Style {
    let is_dark = is_dark_theme(theme);
    let background = if is_dark {
        Color::from_rgba8(20, 21, 24, 0.985)
    } else {
        Color::from_rgba8(252, 252, 253, 0.985)
    };
    let border = if is_dark {
        Color::from_rgba8(44, 47, 53, 0.96)
    } else {
        Color::from_rgba8(226, 231, 237, 1.0)
    };
    container::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 14.0.into(), width: 1.0, color: border },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.22 } else { 0.08 }),
            offset: iced::Vector::new(0.0, 10.0),
            blur_radius: 28.0,
        },
        ..Default::default()
    }
}
pub fn session_lock_tooltip<'a>(content: impl Into<iced::Element<'a, crate::selector_input::Message>>, i18n: desktop_i18n::I18n, locked: bool) -> iced::Element<'a, crate::selector_input::Message> {
 if locked { iced::widget::tooltip::Tooltip::new(content,
 container(text(i18n.__("会话开始后无法更改。如需更换，请新建会话。")).size(12).style(|_: &Theme| iced::widget::text::Style { color: Some(Color::WHITE) })).padding([6,8]).style(tooltip_dark_style),
 iced::widget::tooltip::Position::Top).gap(6).into() } else { content.into() }
}

fn selector_text_color(theme: &Theme, highlighted: bool) -> Color {
    theme.palette().text.scale_alpha(match (is_dark_theme(theme), highlighted) {
        (true, true) => 0.97, (false, true) => 0.95,
        (true, false) => 0.92, (false, false) => 0.88,
    })
}
