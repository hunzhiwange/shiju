//! Scoped Desktop form reuses the original mapping controls.
use desktop_ui_controls::system_settings_common::*;
use desktop_provider_apps::{state::desktop::Form, aggregation_desktop_types::Event};
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};
use vw_api_app_aggregation::DesktopInstallation;
use vw_api_app_aggregation::DesktopProfileMode;
pub fn overlay<'a, Message: Clone + 'static>(i18n: desktop_i18n::I18n,
    form: &'a Form,
    mapping_fields: Vec<Element<'a, Message>>,
    message: fn(Event) -> Message,
    close: Message,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    let s = form.aggregation.as_ref().expect("scoped Desktop form");
    let active = !form.busy;
    let writable = active && !s.running();
    let action = move |label: &'static str, event: Event, enabled: bool| {
        button(text(i18n.__(label)))
            .padding([8, 14])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(message(event)))
    };
    let mut body = column![text(&form.name).size(18), text(i18n.__("保存映射后刷新状态并预览；确认应用会发布聚合目录，恢复会退出聚合并使用网关保存的直连配置。")), text(i18n.__("请核对供应商、模型和提醒。切换模型后建议新建会话；发布或恢复成功后完全退出并重新启动 Claude Desktop。"))].spacing(12).width(Length::Fill);
    if let Some(status) = &s.status {
        let installation = match status.installation {
            DesktopInstallation::Installed => i18n.__("已安装"),
            DesktopInstallation::NotInstalled => i18n.__("未安装"),
            DesktopInstallation::Unsupported => i18n.__("不支持此平台"),
        };
        let mode = match status.mode {
            DesktopProfileMode::Official => i18n.__("官方"),
            DesktopProfileMode::DirectGateway => i18n.__("网关直连"),
            DesktopProfileMode::Routing => i18n.__("路由"),
            DesktopProfileMode::Aggregation => i18n.__("聚合"),
        };
        body = body.push(text(i18n.sprintf("安装：%s · 当前模式：%s · 已配置：%s · 恢复：%s", &[format!("{}", installation).into(), format!("{}", mode).into(), format!("{}", status.configured).into(), format!("{:?}", status.recovery).into()])));
        for notice in &status.notices {
            body = body.push(text(i18n.sprintf("客户端提醒：%s", &[format!("{:?}", notice).into()])));
        }
    }
    if s.dirty {
        body = body.push(text(i18n.__("草稿尚未保存，请保存映射后再预览。")));
    }
    if form.busy {
        body = body.push(text(i18n.__("正在加载…")));
    }
    if let Some(notice) = &form.notice {
        body = body.push(settings_panel(text(i18n.__(notice))));
    }
    if let Some(p) = &s.preview {
        body = body.push(text(i18n.sprintf("预览默认供应商：%s", &[format!("{}", p.default_provider_id.as_deref().unwrap_or(i18n.__("未选择"))).into()])));
        for issue in &p.issues {
            body = body.push(settings_error_banner(i18n.sprintf("预览问题：%s，阻止发布：%s", &[format!("{:?}", issue.code).into(), format!("{}", issue.blocking).into()])));
        }
        for notice in &p.notices {
            body = body.push(text(i18n.sprintf("预览提醒：%s", &[format!("{:?}", notice).into()])));
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
        body = body.push(text(i18n.sprintf("操作进度：%s / %s · %u/%s · 已请求取消：%s", &[format!("{:?}", o.status).into(), format!("{:?}", o.phase).into(), (o.completed).into(), format!("{}", o.total).into(), format!("{}", o.cancellation_requested).into()])));
    }
    if form.mapping_editable {
        body = body.extend(mapping_fields);
    } else if !s.revision.is_empty() {
        body = body.push(settings_error_banner(i18n.__("此档案不支持映射编辑。")));
    }
    let footer = if let Some(restore) = s.confirmation {
        row![
            text(if restore {
                i18n.__("确认退出聚合并恢复直连？")
            } else {
                i18n.__("确认应用此预览的聚合目录？")
            }),
            action(i18n.__("取消"), Event::Dismiss, active),
            button(text(i18n.__("确认执行")))
                .padding([8, 14])
                .style(primary_action_btn_style)
                .on_press_maybe(active.then_some(message(Event::Confirm)))
        ]
        .spacing(8)
        .wrap()
    } else {
        row![
            action(i18n.__("刷新"), Event::Refresh, active),
            action(
                i18n.__("保存映射"),
                Event::Save,
                writable && form.mapping_editable && !s.revision.is_empty()
            ),
            action(i18n.__("预览"), Event::Preview, writable && !s.dirty && !s.revision.is_empty()),
            action(i18n.__("应用预览"), Event::Review(false), writable && s.can_publish()),
            action(i18n.__("恢复直连"), Event::Review(true), writable && s.status.is_some()),
            action(i18n.__("查询进度"), Event::Poll, active && s.operation.is_some()),
            action(i18n.__("请求取消"), Event::Cancel, active && s.running())
        ]
        .spacing(8)
        .wrap()
    };
    let header = settings_modal_header(
        text(i18n.__("Desktop 模型映射与应用")).size(20),
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
