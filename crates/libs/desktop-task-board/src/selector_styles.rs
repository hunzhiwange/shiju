use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};
use desktop_ui_controls::system_settings_common::*;
pub fn popover_style(theme: &Theme) -> container::Style {
    let menu = settings_pick_list_menu_style(theme);
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: menu.border,
        shadow: menu.shadow,
        ..Default::default()
    }
}
pub fn selector_option_style(
    theme: &Theme,
    status: button::Status,
    selected: bool,
) -> button::Style {
    let background = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Some(theme.palette().text.scale_alpha(0.08).into())
    } else if selected {
        Some(settings_pick_list_menu_style(theme).selected_background)
    } else {
        None
    };
    button::Style {
        background,
        border: Border { radius: 9.0.into(), ..Default::default() },
        text_color: theme.palette().text,
        ..Default::default()
    }
}
pub fn selector_trigger_style(
    theme: &Theme,
    status: button::Status,
    open: bool,
) -> button::Style {
    use iced::widget::pick_list::Status;
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let status = if open {
        Status::Opened { is_hovered: hovered }
    } else if hovered {
        Status::Hovered
    } else {
        Status::Active
    };
    let style = settings_pick_list_style(theme, status);
    button::Style {
        background: Some(style.background),
        border: Border { radius: 10.0.into(), ..style.border },
        text_color: style.text_color,
        ..Default::default()
    }
}
pub fn square_icon_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    enabled: bool,
) -> iced::widget::button::Style {
    let mut style = round_icon_btn_style(theme, status);
    if !enabled {
        style.text_color = theme.extended_palette().background.weak.text;
    }
    style.border.radius = 12.0.into();
    style
}
pub fn tooltip_dark_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(24, 24, 24, 0.96))),
        text_color: Some(Color::WHITE),
        border: Border { radius: 8.0.into(), width: 0.0, color: Color::TRANSPARENT },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.40),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}
