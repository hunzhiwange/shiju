#[derive(Clone)]
pub enum OverlayEvent<E> {
    Local(Event),
    External(E),
}
use vw_api_app_operations::DataBackupSummary;
#[derive(Clone)]
pub enum Event {
    Open,
    Close,
    Principal(String),
    Name(String),
    Interval(String),
    Retain(String),
    Refresh,
    Create,
    SavePolicy,
    Cancel,
    Confirm,
    Rename(DataBackupSummary),
    Delete(DataBackupSummary),
}
fn message<E>(event: Event) -> OverlayEvent<E> { OverlayEvent::Local(event) }
pub enum Pending<'a> {
    Cleanup(&'a str),
    Rename(&'a str),
    Delete(&'a str),
}
pub struct ViewState<'a> {
    pub busy: bool,
    pub principal: &'a str,
    pub name: &'a str,
    pub interval: &'a str,
    pub retain: &'a str,
    pub pending: Option<Pending<'a>>,
    pub rows: &'a [DataBackupSummary],
}
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, Length};
use iced::widget::{button, column, row, text};

fn action<'a, E: Clone + 'static>(label: &'a str, event: Event, enabled: bool) -> Element<'a, OverlayEvent<E>> {
    button(text(label)).padding([8, 14]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then_some(message(event))).into()
}
pub fn view(available: bool) -> Element<'static, Event> {
    button(text("维护执行主机备份")).padding([8, 14]).style(rounded_action_btn_style).on_press_maybe(available.then_some(Event::Open)).into()
}

pub fn overlay<'a, E: Clone + 'static>(state: ViewState<'a>, visible: bool, base: Element<'a, E>, locations: Element<'a, E>) -> Element<'a, OverlayEvent<E>> {
    if !visible { return base.map(OverlayEvent::External); }
    let enabled = !state.busy;
    let header = settings_modal_header(text("执行主机备份维护").size(18), settings_close_button(message(Event::Close)));
    let mut body = column![
        text("本机回退备份仅用于原执行主机；所有操作由网关重新鉴权。"),
        text("授权主体 ID"),
        settings_text_input("填写授权主体 ID", state.principal).on_input_maybe(enabled.then_some(|v| message(Event::Principal(v)))),
        action("刷新列表、分类与策略", Event::Refresh, enabled && !state.principal.trim().is_empty()),
        text("备份名称（创建或重命名）"),
        settings_text_input("备份名称", state.name).on_input_maybe(enabled.then_some(|v| message(Event::Name(v)))),
        action("创建本机回退备份", Event::Create, enabled && state.pending.is_none() && !state.name.trim().is_empty() && !state.principal.trim().is_empty()),
        text("周期小时数（0 关闭自动备份） / 保留数量"),
        row![
            settings_text_input("小时", state.interval).on_input_maybe(enabled.then_some(|v| message(Event::Interval(v)))),
            settings_text_input("数量", state.retain).on_input_maybe(enabled.then_some(|v| message(Event::Retain(v)))),
            action("保存策略", Event::SavePolicy, enabled && !state.principal.trim().is_empty()),
        ].spacing(10),
    ].spacing(12).width(Length::Fill);
    if let Some(pending) = &state.pending {
        let prompt = match pending {
            Pending::Cleanup(id) => format!("确认永久清理分类 {id}？可能因保护规则仅清理部分文件。"),
            Pending::Rename(row) => format!("确认重命名备份“{}”？", row),
            Pending::Delete(row) => format!("确认永久删除备份“{}”？受保护或版本冲突将拒绝删除。", row),
        };
        body = body.push(column![text(prompt), row![action("取消操作", Event::Cancel, enabled),
            action("确认操作", Event::Confirm, enabled)].spacing(10)].spacing(8));
    }
    if state.rows.is_empty() { body = body.push(text("暂无已加载备份，请刷新列表。")); }
    for backup in state.rows {
        body = body.push(settings_group_card(column![
            text(backup.name.clone()),
            text(format!("{:?} · {} 字节 · 创建时间 {} · 类别 {:?}", backup.kind, backup.size_bytes, backup.created_at_ms, backup.categories)),
            row![action("重命名", Event::Rename(backup.clone()), enabled && state.pending.is_none()),
                action("删除", Event::Delete(backup.clone()), enabled && state.pending.is_none())].spacing(10),
        ].spacing(8)));
    }
    body = body.push(locations.map(OverlayEvent::External));
    let footer = row![text(if state.busy { "正在处理…" } else { "修改后自动刷新列表、分类与策略；网络失败后请先刷新核对。" }),
        action("关闭", Event::Close, enabled)].spacing(10);
    settings_modal_overlay(Some(base.map(OverlayEvent::External)), message(Event::Close), settings_modal_layout(header, body, Some(footer.into())).width(760).height(680))
}
