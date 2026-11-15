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
    let close = if form.writing() { Message::None } else { message(Event::Close) };
    let header = settings_modal_header(
        help.heading("供应商模式", "查看执行主机的直连、路由和聚合状态。", 28),
        settings_close_button(close.clone()),
    );
    let ready = form.pending.is_none();
    let mut body = column![].spacing(12).width(Length::Fill);
    if let Some(error) = form.error {
        body = body.push(settings_error_banner(error));
    }
    if form.confirming_direct && !form.needs_refresh {
        body = body.push(settings_panel(text(format!(
            "确认恢复已保存的直连供应商「{}」？执行主机将校验配置并更新客户端。",
            provider_name(snapshot, form.status.as_ref().and_then(|s| s.direct_provider.as_deref())),
        ))));
    }
    if !ready {
        body = body.push(settings_panel(text(if form.writing() {
            "正在恢复直连配置…"
        } else {
            "正在读取模式状态…"
        })));
    }
    if let Some(status) = &form.status {
        body = body.push(summary(snapshot, status));
        let mut members = column![
            settings_field_label("聚合配置"),
            text(if status.stack.active {
                "当前模式使用聚合配置"
            } else if status.stack.enabled {
                "已启用，当前未使用"
            } else {
                "未启用"
            }),
            text(format!("已配置 {} 家供应商", status.stack.members.len()))
                .size(13)
                .style(settings_muted_text_style),
            text("聚合模式切换不可用。").size(13).style(settings_muted_text_style),
        ]
        .spacing(6);
        for id in &status.stack.members {
            members = members.push(text(provider_name(snapshot, Some(id))).size(14));
        }
        body = body.push(settings_panel(members));
    }

    let footer = if form.confirming_direct {
        row![
            action("取消", Event::Cancel, ready, false),
            action("重新读取", Event::Refresh, ready && form.needs_refresh, false),
            action("确认恢复直连", Event::Confirm, form.can_restore_direct(), true),
        ]
    } else {
        row![
            action("关闭", Event::Close, !form.writing(), false),
            action("重新读取", Event::Refresh, ready, false),
            action("恢复直连…", Event::ReviewDirect, form.can_restore_direct(), true),
        ]
    };
    let card = settings_modal_layout_with_scroll_id(
        header,
        body,
        Some(footer.spacing(10).into()),
        Some(SCROLL_ID),
    )
    .width(640)
    .height(600);
    card.into()
}

fn summary<'a>(snapshot: Option<&'a crate::state::list::Snapshot>, status: &'a ProviderModeStatus) -> Element<'a, Message> {
    let mode = match status.mode {
        ProviderMode::Direct => "直连",
        ProviderMode::Proxy => "路由",
    };
    let runtime = match status.route_runtime {
        RouteRuntimeStatus::Unavailable => "执行主机的路由服务不可用，无法切换到路由。",
    };
    settings_panel(
        column![
            settings_field_label("模式状态"),
            text(format!("配置模式：{mode}")),
            text(format!(
                "直连供应商：{}",
                provider_name(snapshot, status.direct_provider.as_deref())
            )),
            text(format!("模式目标：{}", provider_name(snapshot, status.in_use_provider.as_deref()))),
            text(format!(
                "上次配置的路由目标：{}",
                provider_name(snapshot, status.proxy_route.as_deref())
            )),
            text(if status.attached {
                "客户端配置已接入路由"
            } else {
                "客户端配置未接入路由"
            }),
            text(runtime).size(13).style(settings_muted_text_style),
        ]
        .spacing(6),
    )
    .into()
}

fn provider_name<'a>(snapshot: Option<&'a crate::state::list::Snapshot>, id: Option<&'a str>) -> &'a str {
    let Some(id) = id else { return "未配置" };
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
