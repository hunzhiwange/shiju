use super::*;

pub(super) fn navigation_icon(icon: Icon) -> svg::Svg<'static> {
    navigation_icon_enabled(icon, true)
}

pub(super) fn navigation_icon_enabled(icon: Icon, enabled: bool) -> svg::Svg<'static> {
    let size = match icon {
        Icon::Plus | Icon::Grid1x2 => 15.0,
        _ => 18.0,
    };
    svg::Svg::new(assets::get_icon(icon))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text.scale_alpha(if enabled { 0.78 } else { 0.32 })),
        })
}

pub(super) fn navigation_button(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    icon: Icon,
    message: Message,
    trailing: Option<&'static str>,
) -> Element<'static, Message> {
    navigation_button_maybe(i18n, label, icon, Some(message), trailing)
}

pub(super) fn navigation_button_maybe(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    icon: Icon,
    message: Option<Message>,
    trailing: Option<&'static str>,
) -> Element<'static, Message> {
    let enabled = message.is_some();
    let mut content = row![
        container(navigation_icon_enabled(icon, enabled))
            .width(Length::Fixed(WORK_NAVIGATION_ICON_COLUMN_WIDTH)),
        text(i18n.__(label)).size(14).style(move |theme: &Theme| text::Style {
            color: (!enabled).then(|| theme.palette().text.scale_alpha(0.32)),
        }),
        Space::new().width(Length::Fill),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);

    if let Some(trailing) = trailing {
        content = content.push(text(i18n.__(trailing)).size(11).style(|theme: &Theme| {
            text::Style { color: Some(theme.palette().text.scale_alpha(0.48)) }
        }));
    }

    let mut control =
        button(content).padding([9, 11]).width(Length::Fill).style(navigation_button_style);
    if let Some(message) = message {
        control = control.on_press(message);
    }
    control.into()
}

pub(super) fn more_menu_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(Background::Color(palette.background.weak.color)),
        text_color: Some(palette.background.weak.text),
        border: Border {
            width: 1.0,
            color: palette.background.strong.color.scale_alpha(0.72),
            radius: 18.0.into(),
        },
        shadow: Shadow {
            color: iced::Color::BLACK.scale_alpha(if palette.is_dark { 0.34 } else { 0.16 }),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 30.0,
        },
        ..Default::default()
    }
}

pub(super) fn more_menu_divider() -> Element<'static, Message> {
    container(Space::new().height(Length::Fixed(1.0)))
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(
                theme.extended_palette().background.strong.color.scale_alpha(0.52),
            )),
            ..Default::default()
        })
        .into()
}

pub(super) fn close_more_then(message: Message) -> Message {
    Message::Batch(vec![Message::View(ViewMessage::WorkSidebarMoreClosed), message])
}

pub(super) fn more_menu_action(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    icon: Icon,
    message: Message,
    width: Length,
) -> Element<'static, Message> {
    button(
        row![
            container(navigation_icon(icon)).width(Length::Fixed(28.0)),
            text(i18n.__(label)).size(15),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .on_press(close_more_then(message))
    .padding([11, 12])
    .width(width)
    .style(navigation_button_style)
    .into()
}

pub(super) fn more_menu_items(
    i18n: desktop_i18n::I18n,
    width: Length,
) -> iced::widget::Column<'static, Message> {
    let specs = more_menu_item_specs();
    let add_url = Message::Batch(vec![
        Message::View(ViewMessage::OpenApps),
        Message::View(ViewMessage::ToggleWebLinksMenu),
    ]);
    column![
        more_menu_action(i18n, specs[0].0, specs[0].1, add_url, width),
        more_menu_divider(),
        more_menu_action(
            i18n,
            specs[1].0,
            specs[1].1,
            Message::View(ViewMessage::OpenKnowledge),
            width
        ),
        more_menu_action(
            i18n,
            specs[2].0,
            specs[2].1,
            Message::View(ViewMessage::OpenWorkflowTool),
            width,
        ),
        more_menu_divider(),
        more_menu_action(i18n, specs[3].0, specs[3].1, Message::View(ViewMessage::OpenApps), width),
    ]
    .spacing(3)
    .width(width)
}

pub(super) fn more_control(app: &ViewInput<'_>) -> Element<'static, Message> {
    let (_, icon, trailing) = navigation_specs()[8];
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let trigger = navigation_button(
        desktop_i18n::I18n::new(app.desktop_locale),
        desktop_i18n::I18n::new(app.desktop_locale).__("更多"),
        icon,
        Message::View(ViewMessage::WorkSidebarMoreOpened),
        trailing,
    );
    let hover_trigger =
        mouse_area(trigger).on_enter(Message::View(ViewMessage::WorkSidebarMoreOpened));
    let menu = container(more_menu_items(i18n, Length::Fill))
        .padding([12, 14])
        .width(Length::Fixed(252.0))
        .style(more_menu_style);
    SideOverlay::new(hover_trigger, menu)
        .show(app.work_sidebar_more_open)
        .gap(8.0)
        .on_close(Message::View(ViewMessage::WorkSidebarMoreClosed))
        .on_mouse_exit(Message::View(ViewMessage::WorkSidebarMoreClosed))
        .into()
}
