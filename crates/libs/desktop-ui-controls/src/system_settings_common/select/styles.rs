use super::super::{settings_pick_list_menu_style, settings_pick_list_style};
use iced::widget::{button, container, pick_list, svg};
use iced::{Border, Color, Theme};

pub(super) fn icon(theme: &Theme, _: svg::Status) -> svg::Style {
    svg::Style { color: Some(theme.palette().text.scale_alpha(0.7)) }
}

pub(super) fn label(theme: &Theme, placeholder: bool, enabled: bool) -> Color {
    let alpha = if !enabled {
        0.55
    } else if placeholder {
        0.65
    } else {
        1.0
    };
    theme.palette().text.scale_alpha(alpha)
}

pub(super) fn trigger(
    theme: &Theme,
    enabled: bool,
    open: bool,
    focused: bool,
    hovered: bool,
) -> pick_list::Style {
    let mut style = settings_pick_list_style(theme, pick_list::Status::Active);
    let palette = theme.extended_palette();
    style.border.radius = 10.0.into();
    style.handle_color = theme.palette().text.scale_alpha(0.7);
    if !enabled {
        style.background = palette.background.base.color.scale_alpha(0.6).into();
        style.border.color = style.border.color.scale_alpha(0.6);
    } else {
        if open || hovered {
            style.background = theme.palette().text.scale_alpha(0.05).into();
            style.border.color = palette.background.strong.color;
        }
        if focused {
            style.border.color = theme.palette().primary.scale_alpha(0.84);
        }
    }
    style
}

pub(super) fn card(theme: &Theme) -> container::Style {
    let style = settings_pick_list_menu_style(theme);
    container::Style {
        background: Some(theme.extended_palette().background.base.color.into()),
        border: Border { radius: 16.0.into(), ..style.border },
        shadow: style.shadow,
        ..Default::default()
    }
}

pub(super) fn option(theme: &Theme, status: button::Status, keyboard: bool) -> button::Style {
    let highlighted =
        keyboard || matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        text_color: theme.palette().text,
        background: Some(if highlighted {
            theme.palette().text.scale_alpha(0.06).into()
        } else {
            Color::TRANSPARENT.into()
        }),
        border: Border { radius: 9.0.into(), ..Default::default() },
        ..Default::default()
    }
}
