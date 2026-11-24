#[derive(Clone)]
pub enum Event { Close, Refresh, Cancel, Confirm, ReviewDirect }
fn message(event: Event) -> Message { Message::Mode(event) }
use desktop_ui_controls::system_settings_common::*;
use super::help::Help;
use crate::{Message, state::mode::*};
use iced::{
    Element, Length,
    widget::{button, column, row, text},
};
use vw_api_app_provider_environment::mode_state::ProviderMode;
use vw_api_app_provider_environment::mode_state::ProviderModeStatus;
use vw_api_app_provider_environment::mode_state::RouteRuntimeStatus;

pub fn card<'a>(
    snapshot: Option<&'a crate::state::list::Snapshot>,
    form: &'a Form,
    help: Help<'_>,
) -> Element<'a, Message> {
    let i18n = help.i18n;
    let close = if form.writing() { Message::None } else { message(Event::Close) };
    let header = column![
        settings_modal_header(
            help.heading(i18n.__("供应商模式"), i18n.__("查看直连、路由和聚合状态。"), 20),
            settings_close_button(close.clone()),
        ),
        settings_divider(),
    ]
    .spacing(12);
    let ready = form.pending.is_none();
    let mut body = column![].spacing(12).width(Length::Fill);
    if let Some(error) = form.error {
        body = body.push(settings_error_banner(i18n.__(error)));
    }
    if form.confirming_direct && !form.needs_refresh {
        body = body.push(settings_panel(text(i18n.format("确认恢复已保存的直连供应商「%s」？保存时将校验配置并更新客户端。", &[(provider_name(i18n, snapshot, form.status.as_ref().and_then(|s| s.direct_provider.as_deref())).to_string()).into()]).expect("valid display template"))));
    }
    if !ready {
        body = body.push(settings_panel(text(if form.writing() {
            i18n.__("正在恢复直连配置…")
        } else {
            i18n.__("正在读取模式状态…")
        })));
    }
    if let Some(status) = &form.status {
        body = body.push(summary(i18n, snapshot, status));
        let mut members = column![
            settings_field_label(i18n.__("聚合配置")),
            text(if status.stack.active {
                i18n.__("当前模式使用聚合配置")
            } else if status.stack.enabled {
                i18n.__("已启用，当前未使用")
            } else {
                i18n.__("未启用")
            }),
            text(i18n.format("已配置 %s 家供应商", &[(format!("{}", status.stack.members.len())).into()]).expect("valid display template"))
                .size(13)
                .style(settings_muted_text_style),
            text(i18n.__("聚合模式切换不可用。")).size(13).style(settings_muted_text_style),
        ]
        .spacing(6);
        for id in &status.stack.members {
            members = members.push(text(provider_name(i18n, snapshot, Some(id))).size(14));
        }
        body = body.push(settings_panel(members));
    }

    let footer = if form.confirming_direct {
        row![
            action(i18n.__("取消"), Event::Cancel, ready, false),
            action(i18n.__("重新读取"), Event::Refresh, ready && form.needs_refresh, false),
            action(i18n.__("确认恢复直连"), Event::Confirm, form.can_restore_direct(), true),
        ]
    } else {
        row![
            action(i18n.__("关闭"), Event::Close, !form.writing(), false),
            action(i18n.__("重新读取"), Event::Refresh, ready, false),
            action(i18n.__("恢复直连…"), Event::ReviewDirect, form.can_restore_direct(), true),
        ]
    };
    let card = settings_modal_layout_with_scroll_id(
        header,
        body,
        Some(
            column![
                settings_divider(),
                iced::widget::container(footer.spacing(10)).align_right(Length::Fill)
            ]
            .spacing(12)
            .into(),
        ),
        Some(SCROLL_ID),
    )
    .width(640)
    .height(600);
    card.into()
}

fn summary<'a>(i18n: desktop_i18n::I18n, snapshot: Option<&'a crate::state::list::Snapshot>, status: &'a ProviderModeStatus) -> Element<'a, Message> {
    let mode = match status.mode {
        ProviderMode::Direct => i18n.__("直连"),
        ProviderMode::Proxy => i18n.__("路由"),
    };
    let runtime = match status.route_runtime {
        RouteRuntimeStatus::Unavailable => i18n.__("路由服务不可用，无法切换到路由。"),
    };
    settings_panel(
        column![
            settings_field_label(i18n.__("模式状态")),
            text(i18n.format("配置模式：%s", &[(mode.to_string()).into()]).expect("valid display template")),
            text(i18n.format("直连供应商：%s", &[(provider_name(i18n, snapshot, status.direct_provider.as_deref()).to_string()).into()]).expect("valid display template")),
            text(i18n.format("模式目标：%s", &[(provider_name(i18n, snapshot, status.in_use_provider.as_deref()).to_string()).into()]).expect("valid display template")),
            text(i18n.format("上次配置的路由目标：%s", &[(provider_name(i18n, snapshot, status.proxy_route.as_deref()).to_string()).into()]).expect("valid display template")),
            text(if status.attached {
                i18n.__("客户端配置已接入路由")
            } else {
                i18n.__("客户端配置未接入路由")
            }),
            text(runtime).size(13).style(settings_muted_text_style),
        ]
        .spacing(6),
    )
    .into()
}

fn provider_name<'a>(i18n: desktop_i18n::I18n, snapshot: Option<&'a crate::state::list::Snapshot>, id: Option<&'a str>) -> &'a str {
    let Some(id) = id else { return i18n.__("未配置") };
    snapshot
        .and_then(|snapshot| snapshot.catalog.rows.iter().find(|entry| entry.provider.id == id))
        .map_or(id, |entry| entry.provider.name.as_str())
}

fn action(
    label: &'static str,
    event: Event,
    enabled: bool,
    primary: bool,
) -> Element<'static, Message> {
    button(text(label))
        .padding([8, 14])
        .style(if primary { primary_action_btn_style } else { rounded_action_btn_style })
        .on_press_maybe(enabled.then(|| message(event)))
        .into()
}
