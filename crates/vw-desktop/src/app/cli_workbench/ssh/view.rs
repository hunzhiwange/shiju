use desktop_assets::assets::Icon;
use desktop_ui_controls::single_line_label::SingleLineLabel;
use desktop_ui_controls::system_settings_common::icons::icon_btn_svg_style;
use desktop_ui_controls::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_checkbox_style,
    settings_close_button, settings_divider, settings_error_banner, settings_field_label,
    settings_modal_header, settings_modal_layout, settings_modal_overlay_with_escape,
    settings_muted_text_style, settings_panel, settings_section_content, settings_text_input,
};
use iced::{
    Alignment, Element, Length,
    widget::{Space, button, checkbox, column, container, row, text},
};
use vw_api_types::cli_workbench::{CliWorkbenchErrorCode as Code, SshConnection};

use super::{Action, Form, Panel};
use crate::app::cli_workbench::{Message, Workbench, update, view::View};

fn msg(ui: View, action: Action) -> Message {
    ui.msg(update::Action::Ssh(action))
}

fn control<'a>(
    ui: View,
    label: &'static str,
    action: Option<Action>,
    primary: bool,
) -> button::Button<'a, Message> {
    button(text(ui.label(label)).size(13))
        .padding([7, 12])
        .style(if primary { primary_action_btn_style } else { rounded_action_btn_style })
        .on_press_maybe(action.map(|action| msg(ui, action)))
}

pub(in super::super) fn view<'a>(
    ui: View,
    base: Element<'a, Message>,
    state: &'a Workbench,
    panel: &'a Panel,
) -> Element<'a, Message> {
    let close = ui.msg(update::Action::Dismiss);
    let ready = !state.ssh.pending;
    let back = control(ui, "返回", ready.then_some(Action::Back), false);
    let mut footer = row![].spacing(8).align_y(Alignment::Center);
    let (title, body, height) = match panel {
        Panel::Saved => {
            if let Some(catalog) = &state.ssh.catalog {
                footer = footer
                    .push(control(ui, "刷新", ready.then_some(Action::Refresh), false))
                    .push(Space::new().width(Length::Fill))
                    .push(control(ui, "添加连接", ready.then_some(Action::ShowImport), true));
                let mut entries =
                    column![settings_field_label(ui.label("已保存的连接"))].spacing(10);
                if catalog.saved.is_empty() {
                    entries = entries.push(
                        container(
                            column![
                                icon_svg(Icon::Globe, 28.0).style(icon_btn_svg_style),
                                text(ui.label("还没有 SSH 连接")).size(15),
                            ]
                            .align_x(Alignment::Center)
                            .spacing(12),
                        )
                        .center_x(Length::Fill)
                        .padding([32, 0]),
                    );
                } else {
                    let mut connections = column![].spacing(6);
                    for (index, connection) in catalog.saved.iter().enumerate() {
                        if index > 0 {
                            connections = connections.push(settings_divider());
                        }
                        connections = connections.push(connection_row(
                            ui,
                            connection,
                            ready,
                            super::can_connect(state),
                        ));
                    }
                    entries = entries.push(settings_panel(connections).padding([8, 12]));
                    if !super::can_connect(state) && state.selected.is_none() {
                        entries = entries.push(
                            text(ui.label("请选择项目后连接"))
                                .size(12)
                                .style(settings_muted_text_style),
                        );
                    }
                }
                ("SSH 连接", entries.into(), 430.0)
            } else {
                footer = footer.push(Space::new().width(Length::Fill)).push(control(
                    ui,
                    "刷新",
                    ready.then_some(Action::Refresh),
                    true,
                ));
                let body = if state.ssh.pending {
                    text(ui.label("正在读取 SSH 连接…")).size(14).into()
                } else {
                    Space::new().into()
                };
                ("SSH 连接", body, 260.0)
            }
        }
        Panel::Import(selected) => {
            let mut aliases =
                column![settings_field_label(ui.label("网关 SSH 配置中的主机"))].spacing(10);
            if let Some(catalog) = &state.ssh.catalog {
                if catalog.aliases.is_empty() {
                    aliases = aliases.push(
                        text(ui.label("没有可导入的 SSH 主机"))
                            .size(14)
                            .style(settings_muted_text_style),
                    );
                } else {
                    let mut list = column![].spacing(6);
                    for (index, alias) in catalog.aliases.iter().enumerate() {
                        if index > 0 {
                            list = list.push(settings_divider());
                        }
                        let name = alias.clone();
                        list = list.push(
                            row![
                                icon_svg(Icon::Terminal, 16.0).style(icon_btn_svg_style),
                                text(alias).size(14).width(Length::Fill),
                                checkbox(selected.contains(alias))
                                    .on_toggle_maybe(ready.then_some(move |checked| {
                                        msg(ui, Action::ToggleAlias(name.clone(), checked))
                                    }))
                                    .style(settings_checkbox_style),
                            ]
                            .align_y(Alignment::Center)
                            .spacing(10)
                            .padding([6, 0]),
                        );
                    }
                    aliases = aliases.push(settings_panel(list).padding([8, 12]));
                }
            }
            footer = footer
                .push(back)
                .push(control(ui, "刷新", ready.then_some(Action::Refresh), false))
                .push(Space::new().width(Length::Fill))
                .push(control(ui, "手动添加", ready.then_some(Action::AddManual), false))
                .push(control(
                    ui,
                    "导入所选",
                    (ready && !selected.is_empty()).then_some(Action::Import),
                    true,
                ));
            ("添加 SSH 连接", aliases.into(), 430.0)
        }
        Panel::Edit(form) => {
            footer = footer.push(back).push(Space::new().width(Length::Fill)).push(control(
                ui,
                "保存",
                (ready && form.fields().is_some()).then_some(Action::Save),
                true,
            ));
            let height = if form.from_config {
                300.0
            } else if form.identity.is_some() {
                485.0
            } else {
                420.0
            };
            let title = if form.id.is_some() { "编辑 SSH 连接" } else { "添加 SSH 连接" };
            (title, form_view(ui, form, ready), height)
        }
        Panel::Details(id) | Panel::Delete(id) => {
            let Some(connection) = state.ssh.connection(id) else { return base };
            let deleting = matches!(panel, Panel::Delete(_));
            footer = footer.push(back).push(Space::new().width(Length::Fill));
            if deleting {
                footer = footer.push(control(
                    ui,
                    "删除",
                    ready.then(|| Action::Delete(id.clone())),
                    true,
                ));
                (
                    "删除 SSH 连接",
                    column![
                        text(&connection.fields.name).size(16),
                        text(ui.label("从已保存的连接中删除此主机？")).size(13)
                    ]
                    .spacing(10)
                    .into(),
                    270.0,
                )
            } else {
                footer = footer
                    .push(control(
                        ui,
                        "删除",
                        ready.then(|| Action::ConfirmDelete(id.clone())),
                        false,
                    ))
                    .push(control(ui, "编辑", ready.then(|| Action::Edit(id.clone())), true));
                ("SSH 连接详情", details(ui, connection), 375.0)
            }
        }
    };
    let title =
        settings_modal_header(text(ui.label(title)).size(20), settings_close_button(close.clone()));
    let mut content = column![].spacing(10);
    if let Some(code) = state.ssh.error {
        let label = match code {
            Code::NotFound if state.ssh.catalog.is_none() => "当前网关未提供 SSH 连接管理",
            Code::Unavailable | Code::Unauthorized => "SSH 需要直连装有 OpenSSH 的本机网关",
            Code::InvalidRequest => "请检查 SSH 连接信息",
            Code::NotFound => "SSH 主机已变更，请刷新",
            _ => "无法读取或保存 SSH 连接",
        };
        content = content.push(settings_error_banner(ui.label(label).to_owned()));
    }
    let card = settings_modal_layout(title, content.push(body), Some(footer.into()))
        .width(Length::Fill)
        .height(Length::Fill)
        .max_width(560)
        .max_height(height);
    settings_modal_overlay_with_escape(base, close, card)
}

fn connection_row<'a>(
    ui: View,
    connection: &'a SshConnection,
    enabled: bool,
    can_connect: bool,
) -> Element<'a, Message> {
    row![
        icon_svg(Icon::Globe, 17.0).style(icon_btn_svg_style),
        column![
            SingleLineLabel::new(connection.fields.name.clone(), 14.0),
            SingleLineLabel::new(connection.fields.host.clone(), 12.0),
        ]
        .spacing(2)
        .width(Length::Fill),
        control(ui, "详情", enabled.then(|| Action::Details(connection.id.clone())), false),
        control(
            ui,
            "连接",
            (enabled && can_connect).then(|| Action::Connect(connection.id.clone())),
            true
        ),
    ]
    .align_y(Alignment::Center)
    .spacing(8)
    .padding([4, 0])
    .into()
}

fn field<'a>(
    ui: View,
    label: &'static str,
    placeholder: &'static str,
    value: &str,
    change: fn(String) -> Action,
    editable: bool,
) -> Element<'a, Message> {
    let input = settings_text_input(ui.label(placeholder), value)
        .on_input_maybe(editable.then_some(move |value| msg(ui, change(value))));
    column![settings_field_label(ui.label(label)), input].spacing(6).into()
}

fn form_view<'a>(ui: View, form: &'a Form, enabled: bool) -> Element<'a, Message> {
    let mut form_view = column![
        field(ui, "显示名称", "", &form.name, Action::Name, enabled),
        field(
            ui,
            if form.from_config { "别名" } else { "主机名" },
            "host.com 或 user@host.com",
            &form.host,
            Action::Host,
            enabled && !form.from_config
        ),
    ]
    .spacing(12);
    if !form.from_config {
        form_view = form_view
            .push(field(
                ui,
                "SSH 端口（可选）",
                "由 SSH 配置决定",
                &form.port,
                Action::Port,
                enabled,
            ))
            .push(
                checkbox(form.identity.is_some())
                    .label(ui.label("指定身份文件"))
                    .on_toggle_maybe(
                        enabled.then_some(move |active| msg(ui, Action::UseIdentity(active))),
                    )
                    .style(settings_checkbox_style),
            );
        if let Some(identity) = &form.identity {
            form_view = form_view.push(field(
                ui,
                "网关上的身份文件路径",
                "~/.ssh/id_ed25519",
                identity,
                Action::Identity,
                enabled,
            ));
        }
    }
    form_view.into()
}

fn details<'a>(ui: View, connection: &'a SshConnection) -> Element<'a, Message> {
    let fields = &connection.fields;
    let default = ui.label("由 SSH 配置决定");
    let mut rows = column![].spacing(8);
    for (index, (label, value)) in [
        (if connection.from_config { "别名" } else { "主机名" }, fields.host.clone()),
        ("端口", fields.port.map_or_else(|| default.into(), |port| port.to_string())),
        ("身份文件", fields.identity_file.clone().unwrap_or_else(|| default.into())),
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            rows = rows.push(settings_divider());
        }
        rows = rows.push(
            row![
                text(ui.label(label)).size(13).style(settings_muted_text_style),
                Space::new().width(Length::FillPortion(1)),
                text(value).size(13).width(Length::FillPortion(4)).align_x(Alignment::End),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    column![
        row![icon_svg(Icon::Globe, 20.0).style(icon_btn_svg_style), text(&fields.name).size(18),]
            .spacing(10)
            .align_y(Alignment::Center),
        settings_section_content(rows).padding([10, 0]),
    ]
    .spacing(14)
    .into()
}
