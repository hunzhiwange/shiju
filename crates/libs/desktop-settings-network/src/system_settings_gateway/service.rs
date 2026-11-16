use super::*;
use iced::widget::column;

pub(super) fn service_action_button(
    label: &'static str,
    command: &'static str,
    running_command: Option<&str>,
) -> Element<'static, Message> {
    let is_running = running_command == Some(command);
    let text_label = if is_running { format!("{label}...") } else { label.to_string() };
    let label_content =
        text(text_label).size(14).height(Length::Fill).align_y(iced::alignment::Vertical::Center);
    let btn = button(
        container(label_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .padding([6, 8])
    .width(Length::Fill)
    .height(Length::Fixed(44.0));
    let btn = if is_danger_service_command(command) {
        btn.style(danger_action_btn_style)
    } else {
        btn.style(rounded_action_btn_style)
    };

    if running_command.is_none() {
        btn.on_press(Message::Gateway(
            GatewayMessage::ServiceCommandRequested(command.to_string()),
        ))
        .into()
    } else {
        btn.into()
    }
}

pub(super) fn service_cli_command(command: &str) -> String {
    format!("shiju server {command}")
}

pub(super) fn is_danger_service_command(command: &str) -> bool {
    matches!(command, "stop" | "uninstall")
}

pub fn service_command_copy_button(
    command: &'static str,
    copied_command: Option<&str>,
) -> Element<'static, Message> {
    let cli_command = service_cli_command(command);
    let copied = copied_command == Some(cli_command.as_str());
    let label = if copied { "✓".to_string() } else { cli_command.clone() };
    let icon = if copied { Icon::Check } else { Icon::Copy };
    let content = row![
        text(label)
            .size(11)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
        Space::new().width(Length::Fill),
        icon_svg(icon, 13.0).style(|theme: &iced::Theme, _status| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    button(container(content).width(Length::Fill).height(Length::Fill).center_y(Length::Fill))
        .padding([4, 8])
        .width(Length::Fill)
        .height(Length::Fixed(40.0))
        .on_press(Message::Gateway(GatewayMessage::CopyServiceCommand(
            cli_command,
        )))
        .style(rounded_action_btn_style)
        .into()
}

pub(super) fn service_action_cell(
    label: &'static str,
    command: &'static str,
    running_command: Option<&str>,
    copied_command: Option<&str>,
) -> Element<'static, Message> {
    column![
        service_action_button(label, command, running_command),
        service_command_copy_button(command, copied_command),
    ]
    .spacing(6)
    .width(Length::Fill)
    .into()
}

pub(super) fn service_controls_content<'a>(app: View<'a>) -> Element<'a, Message> {
    let s = app.gateway_settings;
    let running = s.service_action_running.as_deref();
    let copied_command = s.copied_service_command.as_deref();
    let actions = column![
        row![
            service_action_cell(
                connection_text(app.desktop_locale, "安装服务"),
                "install",
                running,
                copied_command
            ),
            service_action_cell(
                connection_text(app.desktop_locale, "启动服务"),
                "start",
                running,
                copied_command
            ),
            service_action_cell(
                connection_text(app.desktop_locale, "停止服务"),
                "stop",
                running,
                copied_command
            ),
        ]
        .spacing(12),
        row![
            service_action_cell(
                connection_text(app.desktop_locale, "重启服务"),
                "restart",
                running,
                copied_command
            ),
            service_action_cell(
                connection_text(app.desktop_locale, "查询状态"),
                "status",
                running,
                copied_command
            ),
            service_action_cell(
                connection_text(app.desktop_locale, "卸载服务"),
                "uninstall",
                running,
                copied_command
            ),
        ]
        .spacing(12),
    ]
    .spacing(14)
    .width(Length::Fill);

    let mut content = column![actions].spacing(12).width(Length::Fill);
    if let Some(output) = s.service_action_output.as_ref().filter(|value| !value.trim().is_empty())
    {
        content = content.push(
            container(
                text(output.display(app.desktop_locale))
                    .size(13)
                    .style(settings_muted_text_style)
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
            )
            .padding([10, 12])
            .width(Length::Fill),
        );
    }

    settings_group_card(content).padding([16, 22]).into()
}

pub fn service_display_label(
    locale: config_ui::DesktopLocale,
    command: &str,
) -> &'static str {
    let key = match command {
        "stop" => "停止服务",
        "uninstall" => "卸载服务",
        "install" => "安装服务",
        "start" => "启动服务",
        "restart" => "重启服务",
        "status" => "查询状态",
        _ => "服务命令",
    };
    connection_text(locale, key)
}
