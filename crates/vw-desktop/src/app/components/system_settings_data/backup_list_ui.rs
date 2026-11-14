use crate::app::{App, Message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::data_management::{export_ui, backup_list_ui::{self, Event, Pending, message}};
use iced::{Element, Length};
use iced::widget::{button, column, row, text};

fn action<'a>(label: &'a str, event: Event, enabled: bool) -> Element<'a, Message> {
    button(text(label)).padding([8, 14]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(message(event))).into()
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    action("维护执行主机备份", Event::Open, export_ui::identity(app).is_some())
}
pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if !backup_list_ui::visible(app) { return base; }
    let state = &app.apps_catalog.data_backups;
    let enabled = !state.busy;
    let header = settings_modal_header(text("执行主机备份维护").size(18), settings_close_button(message(Event::Close)));
    let mut body = column![
        text("本机回退备份仅用于原执行主机；所有操作由网关重新鉴权。"),
        text("授权主体 ID"),
        settings_text_input("填写授权主体 ID", &state.principal).on_input_maybe(enabled.then_some(|v| message(Event::Principal(v)))),
        action("刷新列表、分类与策略", Event::Refresh, enabled && !state.principal.trim().is_empty()),
        text("备份名称（创建或重命名）"),
        settings_text_input("备份名称", &state.name).on_input_maybe(enabled.then_some(|v| message(Event::Name(v)))),
        action("创建本机回退备份", Event::Create, enabled && state.pending.is_none() && !state.name.trim().is_empty() && !state.principal.trim().is_empty()),
        text("周期小时数（0 关闭自动备份） / 保留数量"),
        row![
            settings_text_input("小时", &state.interval).on_input_maybe(enabled.then_some(|v| message(Event::Interval(v)))),
            settings_text_input("数量", &state.retain).on_input_maybe(enabled.then_some(|v| message(Event::Retain(v)))),
            action("保存策略", Event::SavePolicy, enabled && !state.principal.trim().is_empty()),
        ].spacing(10),
    ].spacing(12).width(Length::Fill);
    if let Some(pending) = &state.pending {
        let prompt = match pending {
            Pending::Cleanup(id) => format!("确认永久清理分类 {id}？可能因保护规则仅清理部分文件。"),
            Pending::Rename(row) => format!("确认重命名备份“{}”？", row.name),
            Pending::Delete(row) => format!("确认永久删除备份“{}”？受保护或版本冲突将拒绝删除。", row.name),
        };
        body = body.push(column![text(prompt), row![action("取消操作", Event::Cancel, enabled),
            action("确认操作", Event::Confirm, enabled)].spacing(10)].spacing(8));
    }
    if state.rows.is_empty() { body = body.push(text("暂无已加载备份，请刷新列表。")); }
    for backup in &state.rows {
        body = body.push(settings_group_card(column![
            text(backup.name.clone()),
            text(format!("{:?} · {} 字节 · 创建时间 {} · 类别 {:?}", backup.kind, backup.size_bytes, backup.created_at_ms, backup.categories)),
            row![action("重命名", Event::Rename(backup.clone()), enabled && state.pending.is_none()),
                action("删除", Event::Delete(backup.clone()), enabled && state.pending.is_none())].spacing(10),
        ].spacing(8)));
    }
    body = body.push(super::locations_ui::view(app));
    let footer = row![text(if state.busy { "正在处理…" } else { "修改后自动刷新列表、分类与策略；网络失败后请先刷新核对。" }),
        action("关闭", Event::Close, enabled)].spacing(10);
    settings_modal_overlay(Some(base), message(Event::Close), settings_modal_layout(header, body, Some(footer.into())).width(760).height(680))
}
