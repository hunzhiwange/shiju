use crate::app::{App, Message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::skills::updates_view::{Event, Operation, message};
use iced::{Element, Length};
use iced::widget::{button, column, container, row, text};

pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.updates;
    if !s.open { return base; }
    let mut body = column![text(format!("目标主机：{}", app.apps_catalog.host)),
        text("更新校验已检查的当前哈希。卸载将备份受管内容并保留外部路径。")].spacing(10).width(Length::Fill);
    if let Some(outcome) = &s.outcome { body = body.push(text(outcome.clone())); }
    if let Some(backup) = &s.backup {
        body = body.push(text(format!("备份：{}", backup.backup_id)))
            .push(text(format!("备份路径：{}", backup.backup_path.display())));
        for path in &backup.preserved_paths { body = body.push(text(format!("保留外部路径：{}", path.display()))); }
    }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    if s.busy { body = body.push(text("正在操作或刷新，请等待…")); }
    if let Some(inventory) = &s.inventory {
        body = body.push(text(format!("库存版本：{}", inventory.version)));
        for skill in &inventory.skills {
            let mut card = column![text(skill.name.clone()),
                text(format!("可用版本：{}", skill.version.as_deref().unwrap_or("未提供"))),
                text(format!("当前哈希：{}", skill.content_hash.as_deref().unwrap_or("未提供")))].spacing(6);
            if let Some(info) = s.updates.as_ref().and_then(|u| u.updates.iter().find(|u| u.directory == skill.id)) {
                card = card.push(text(format!("远端哈希：{}", info.remote_hash)))
                    .push(button("确认更新").padding([8, 10]).style(primary_action_btn_style)
                        .on_press_maybe((!s.busy).then(|| message(Event::Run(Operation::Update(skill.id.clone()))))));
            }
            card = card.push(button("确认卸载并备份").padding([8, 10]).style(rounded_action_btn_style)
                .on_press_maybe((!s.busy).then(|| message(Event::Run(Operation::Uninstall(skill.id.clone()))))));
            body = body.push(settings_group_card(card));
        }
    }
    if let Some(updates) = &s.updates {
        for failure in &updates.failures { body = body.push(text(format!("失败仓库：{}/{} ({})：{:?}", failure.owner, failure.name, failure.branch, failure.error))); }
        for failure in &updates.skill_failures { body = body.push(text(format!("技能检查失败：{}：{:?}", failure.directory, failure.error))); }
        for id in &updates.without_repository { body = body.push(text(format!("无仓库来源：{id}"))); }
    }
    let close = message(Event::Close);
    let header = settings_modal_header(text("技能更新与卸载").size(20), settings_close_button(close.clone()));
    let footer = row![button("刷新状态").padding([8, 10]).style(rounded_action_btn_style).on_press_maybe((!s.busy).then(|| message(Event::Refresh))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style).on_press_maybe((!s.busy).then_some(close.clone()))].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into())).style(super::modal_card_style).width(760).height(600);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
