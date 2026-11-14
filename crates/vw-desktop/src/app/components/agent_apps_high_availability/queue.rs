//! Queue eligibility and ordering are confirmed by the execution host.
use crate::app::{Message, components::system_settings_common::*, message::agent_apps_high_availability::{self as queue, Event, Operation, State}};
use iced::{Element, Length, widget::{button, column, container, row, text}};

fn action(label: &'static str, event: Event, enabled: bool) -> Element<'static, Message> {
    button(text(label)).padding([8, 14]).style(rounded_action_btn_style)
        .on_press_maybe(enabled.then(|| queue::message(event))).into()
}
pub(crate) fn overlay<'a>(state: &'a State, base: Element<'a, Message>) -> Element<'a, Message> {
    let close = queue::message(Event::Close);
    let header = settings_modal_header(text("备用队列").size(20), Some(settings_close_button(close.clone())));
    let mut body = column![
        text("队列资格与顺序由执行主机确认；官方账号不能加入备用队列。"),
        action("刷新", Event::Run(Operation::Refresh), !state.busy),
    ].spacing(12);
    if state.busy { body = body.push(text("正在等待执行主机确认…")); }
    if let Some(error) = &state.error { body = body.push(settings_error_banner(error)); }
    if let Some(snapshot) = &state.queue {
        body = body.push(action(if snapshot.enabled { "停用故障转移" } else { "启用故障转移" }, Event::Run(Operation::Enabled(!snapshot.enabled)), !state.busy));
        if snapshot.entries.is_empty() { body = body.push(text("备用队列为空")); }
        for (index, entry) in snapshot.entries.iter().enumerate() {
            body = body.push(settings_panel(column![
                text(format!("{}. {}", index + 1, entry.name)),
                row![
                    action("上移", Event::Run(Operation::Move(index, index.saturating_sub(1))), !state.busy && index > 0),
                    action("下移", Event::Run(Operation::Move(index, index + 1)), !state.busy && index + 1 < snapshot.entries.len()),
                    action("移除", Event::Run(Operation::Remove(entry.provider_id.clone())), !state.busy),
                ].spacing(8).wrap(),
            ].spacing(8)));
        }
        body = body.push(text("可加入的供应商").size(16));
        for entry in state.candidates.iter().filter(|c| !snapshot.entries.iter().any(|e| e.provider_id == c.provider_id)) {
            body = body.push(row![text(&entry.name), action("加入", Event::Run(Operation::Add(entry.provider_id.clone())), !state.busy)].spacing(8));
        }
    }
    let footer = row![action("关闭", Event::Close, true)];
    let card = settings_modal_layout(header, body, Some(footer.into())).width(760).height(640);
    settings_modal_overlay(Some(base), close, container(card).padding(12).center(Length::Fill))
}
