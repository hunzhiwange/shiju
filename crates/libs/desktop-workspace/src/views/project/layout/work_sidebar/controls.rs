use super::*;

pub fn sidebar_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        text_color: Some(theme.palette().text),
        border: Border {
            color: palette.background.strong.color.scale_alpha(0.72),
            width: 0.0,
            radius: 0.0.into(),
        },
        ..work_surface_style(theme)
    }
}

pub(super) fn navigation_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered => Some(palette.background.strong.color.scale_alpha(0.68)),
        button::Status::Pressed => Some(palette.primary.weak.color.scale_alpha(0.42)),
        _ => None,
    };
    button::Style {
        background: background.map(Background::Color),
        text_color: palette.background.base.text,
        border: Border { radius: 8.0.into(), ..Default::default() },
        ..Default::default()
    }
}

pub(super) fn header_icon(icon: Icon) -> svg::Svg<'static> {
    svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(WORK_HEADER_ICON_SIZE))
        .height(Length::Fixed(WORK_HEADER_ICON_SIZE))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text.scale_alpha(0.82)),
        })
}

pub(super) fn header_control_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style + Copy {
    move |theme, status| {
        let palette = theme.extended_palette();
        let background = if active {
            Some(palette.background.strong.color.scale_alpha(0.82))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(palette.background.strong.color.scale_alpha(0.62))
        } else {
            None
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: palette.background.base.text,
            border: Border { radius: 10.0.into(), ..Default::default() },
            ..Default::default()
        }
    }
}

pub(super) fn header_icon_button(
    i18n: desktop_i18n::I18n,
    icon: Icon,
    label: &'static str,
    message: Message,
    active: bool,
) -> Element<'static, Message> {
    header_icon_button_sized(
        i18n,
        header_icon(icon),
        label,
        message,
        active,
        WORK_HEADER_CONTROL_SIZE,
    )
}

pub(super) fn header_icon_button_sized(
    i18n: desktop_i18n::I18n,
    icon: svg::Svg<'static>,
    label: &'static str,
    message: Message,
    active: bool,
    control_size: f32,
) -> Element<'static, Message> {
    let control = icon_control(icon, Some(message), active, control_size);
    Tooltip::new(
        control,
        container(text(i18n.__(label)).size(12)).padding([6, 8]).style(tooltip_dark_style),
        TooltipPosition::Bottom,
    )
    .gap(6)
    .into()
}

pub(super) fn icon_control(
    icon: svg::Svg<'static>,
    message: Option<Message>,
    active: bool,
    control_size: f32,
) -> button::Button<'static, Message> {
    let mut control = button(
        container(icon)
            .width(Length::Fixed(control_size))
            .height(Length::Fixed(control_size))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
    .padding(0)
    .style(header_control_style(active));
    if let Some(message) = message {
        control = control.on_press(message);
    }

    control
}
