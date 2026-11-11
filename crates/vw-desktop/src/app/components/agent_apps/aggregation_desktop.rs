//! Scoped Desktop form reuses the original mapping controls.
use crate::app::{
    Message,
    components::{
        app_providers::{desktop::mapping_fields, list::Help},
        system_settings_common::*,
    },
    message::{
        app_providers::desktop::{Event as LegacyEvent, message as legacy},
        settings::aggregation_desktop::{Event, message},
    },
    state::app_providers::desktop::Form,
};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};
use vw_gateway_client::vw_api_types::agent_apps::aggregation::{
    DesktopInstallation, DesktopProfileMode,
};
pub(crate) fn overlay<'a>(
    form: &'a Form,
    help: Help<'_>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let s = form.aggregation.as_ref().expect("scoped Desktop form");
    let active = !form.busy;
    let writable = active && !s.running();
    let action = |label: &'static str, event: Event, enabled: bool| {
        button(text(label))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(event)))
    };
    let mut body = column![text(&form.name).size(18), text("保存映射后刷新状态并预览；确认应用会发布聚合目录，恢复会退出聚合并使用网关保存的直连配置。"), text("请核对供应商、模型和提醒。切换模型后建议新建会话；发布或恢复成功后完全退出并重新启动 Claude Desktop。")].spacing(12).width(Length::Fill);
    if let Some(status) = &s.status {
        let installation = match status.installation {
            DesktopInstallation::Installed => "已安装",
            DesktopInstallation::NotInstalled => "未安装",
            DesktopInstallation::Unsupported => "不支持此平台",
        };
        let mode = match status.mode {
            DesktopProfileMode::Official => "官方",
            DesktopProfileMode::DirectGateway => "网关直连",
            DesktopProfileMode::Routing => "路由",
            DesktopProfileMode::Aggregation => "聚合",
        };
        body = body.push(text(format!(
            "安装：{installation} · 当前模式：{mode} · 已配置：{} · 恢复：{:?}",
            status.configured, status.recovery
        )));
        for notice in &status.notices {
            body = body.push(text(format!("客户端提醒：{notice:?}")));
        }
    }
    if s.dirty {
        body = body.push(text("草稿尚未保存，请保存映射后再预览。"));
    }
    if form.busy {
        body = body.push(text("正在请求执行主机…"));
    }
    if let Some(notice) = &form.notice {
        body = body.push(settings_panel(text(notice)));
    }
    if let Some(p) = &s.preview {
        body = body.push(text(format!(
            "预览默认供应商：{}",
            p.default_provider_id.as_deref().unwrap_or("未选择")
        )));
        for issue in &p.issues {
            body = body.push(settings_error_banner(format!(
                "预览问题：{:?}，阻止发布：{}",
                issue.code, issue.blocking
            )));
        }
        for notice in &p.notices {
            body = body.push(text(format!("预览提醒：{notice:?}")));
        }
        for member in &p.members {
            body = body.push(settings_field_label(&member.name));
            for model in &member.models {
                body = body.push(text(format!(
                    "{} · {} · 1M：{}",
                    model.label, model.id, model.supports_1m
                )));
            }
        }
    }
    if let Some(o) = &s.operation {
        body = body.push(text(format!(
            "操作进度：{:?} / {:?} · {}/{} · 已请求取消：{}",
            o.status, o.phase, o.completed, o.total, o.cancellation_requested
        )));
    }
    if form.mapping_editable {
        body = body.extend(mapping_fields(form, help));
    } else if !s.revision.is_empty() {
        body = body.push(settings_error_banner("此档案不支持映射编辑。"));
    }
    let footer = if let Some(restore) = s.confirmation {
        row![
            text(if restore {
                "确认退出聚合并恢复直连？"
            } else {
                "确认应用此预览的聚合目录？"
            }),
            action("取消", Event::Dismiss, active),
            button(text("确认执行"))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press_maybe(active.then_some(message(Event::Confirm)))
        ]
        .spacing(8)
        .wrap()
    } else {
        row![
            action("刷新", Event::Refresh, active),
            action(
                "保存映射",
                Event::Save,
                writable && form.mapping_editable && !s.revision.is_empty()
            ),
            action("预览", Event::Preview, writable && !s.dirty && !s.revision.is_empty()),
            action("应用预览", Event::Review(false), writable && s.can_publish()),
            action("恢复直连", Event::Review(true), writable && s.status.is_some()),
            action("查询进度", Event::Poll, active && s.operation.is_some()),
            action("请求取消", Event::Cancel, active && s.running())
        ]
        .spacing(8)
        .wrap()
    };
    let close = legacy(LegacyEvent::Cancel);
    let header = settings_modal_header(
        text("Desktop 模型映射与应用").size(20),
        settings_close_button(close.clone()),
    );
    let card = settings_modal_layout_with_scroll_id(
        header,
        body,
        Some(footer.into()),
        Some("desktop-provider-form"),
    )
    .width(800)
    .height(720);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
