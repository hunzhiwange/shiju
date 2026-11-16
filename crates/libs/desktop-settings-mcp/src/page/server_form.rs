//! Legacy server editing modal.
use super::*;
use iced::widget::column;
pub(crate) fn server_form_heading(locale: DesktopLocale, editing: bool) -> (&'static str, &'static str) {
    if editing {
        (
            tr(locale, "编辑 MCP 服务器"),
            tr(locale, "修改当前服务器；高级 JSON 字段会在传输类型不变时保留。"),
        )
    } else {
        (
            tr(locale, "新增 MCP 服务器"),
            tr(locale, "按服务器逐项添加；复杂字段可继续在高级 JSON 中编辑。"),
        )
    }
}

pub(crate) fn server_form_toggle<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message) -> Message {
    event(McpMessage::NewServerPanelToggled)
}

pub fn new_server_panel<'a, Message: Clone + 'static>(event: fn(McpMessage) -> Message, s: &'a State, locale: DesktopLocale, theme: &'a iced::Theme, help: Help<Message>) -> Element<'a, Message> {
    let i18n = I18n::new(locale);
    let editing = s.editing_server_index.is_some();
    let (title, description) = server_form_heading(locale, editing);
    let controls_enabled = !s.loading && !s.saving;
    let input = |placeholder, value: &str, on_input: fn(String) -> McpMessage| {
        settings_text_input(placeholder, value)
            .on_input(move |next| event(on_input(next)))
    };

    let name_input = input(
        tr(locale, "例如 local-filesystem"),
        &s.new_server.name_input,
        McpMessage::NewServerNameChanged,
    );
    let timeout_input =
        input("30", &s.new_server.timeout_input, McpMessage::NewServerTimeoutChanged);
    let transport =
        settings_select(McpNewServerTransport::ALL, Some(s.new_server.transport), move |value| {
            event(McpMessage::NewServerTransportChanged(value))
        })
        .width(Length::Fill);
    let transport_label = tr(locale, "类型");
    let mut transport_field = column![
        help(
            i18n,
            theme,
            transport_label,
            settings_field_label(transport_label),
            format!("stdio · {} / {}\nHTTP / SSE · URL", tr(locale, "命令"), tr(locale, "参数")).into(),
        )
        .wrap(),
        transport,
    ]
    .spacing(6)
    .width(Length::Fill);
    if editing {
        transport_field = transport_field.push(
            text(tr(locale, "更改类型并保存会清除原有的 env 或 headers。"))
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    let mut fields =
        column![form_field(tr(locale, "名称"), name_input), transport_field].spacing(12);
    match s.new_server.transport {
        McpNewServerTransport::Stdio => {
            fields = fields
                .push(form_field(
                    tr(locale, "命令"),
                    input(
                        "npx / uvx / python",
                        &s.new_server.command_input,
                        McpMessage::NewServerCommandChanged,
                    ),
                ))
                .push(form_field(
                    tr(locale, "参数"),
                    input(
                        "-y, @modelcontextprotocol/server-filesystem, /tmp",
                        &s.new_server.args_input,
                        McpMessage::NewServerArgsChanged,
                    ),
                ));
        }
        McpNewServerTransport::Http | McpNewServerTransport::Sse => {
            fields = fields.push(form_field(
                "URL",
                input(
                    "https://example.com/mcp",
                    &s.new_server.url_input,
                    McpMessage::NewServerUrlChanged,
                ),
            ));
        }
    }
    fields = fields.push(form_field(tr(locale, "超时"), timeout_input)).push(
        row![
            checkbox(s.new_server.enabled)
                .label(tr(locale, "启用"))
                .on_toggle(move |value| {
                    event(McpMessage::NewServerEnabledToggled(
                        value,
                    ))
                })
                .style(settings_checkbox_style),
            checkbox(s.new_server.required)
                .label(tr(locale, "必需"))
                .on_toggle(move |value| {
                    event(McpMessage::NewServerRequiredToggled(
                        value,
                    ))
                })
                .style(settings_checkbox_style),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .wrap(),
    );
    let mut body = column![].spacing(12);
    if let Some(error) = server_save_error(locale, s) {
        body = body.push(settings_error_banner(error));
    }
    body = body.push(settings_panel(fields));

    let heading =
        help(i18n, theme, title, text(title).size(20), description.into())
            .wrap();
    let header = settings_modal_header(heading, settings_close_button(server_form_toggle(event)));
    let submit_label =
        if editing { tr(locale, "保存修改") } else { tr(locale, "新增服务器") };
    let submit_message = if editing { McpMessage::SaveServerEdit } else { McpMessage::AddServer };
    let mut footer = row![Space::new().width(Length::Fill)];
    if editing {
        footer = footer.push(toolbar_button(
            tr(locale, "取消编辑"),
            controls_enabled
                .then(|| event(McpMessage::ServerEditCanceled)),
        ));
    }
    let footer = footer
        .push(toolbar_button(tr(locale, "收起"), Some(server_form_toggle(event))))
        .push(
            button(text(submit_label).size(13))
                .padding([8, 16])
                .style(primary_action_btn_style)
                .on_press_maybe(
                    controls_enabled
                        .then(|| event(submit_message)),
                ),
        )
        .spacing(8)
        .align_y(Alignment::Center);
    settings_modal_layout(header, body, Some(footer.into()))
        .style(|theme| {
            let mut style = settings_modal_card_style(theme);
            if let Some(iced::Background::Color(color)) = &mut style.background {
                color.a = 1.0;
            }
            style
        })
        .width(640)
        .height(590)
        .into()
}
