//! Execution-host backups and explicit restore destination confirmation.
use crate::app::{App, Message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::skills::backup_modal::{Event, message};
use iced::{Element, widget::{button, checkbox, column, container, row, text}};

pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.backups;
    if !s.open { return base; }
    let control = |label: String, event| button(label).padding([8, 10]).style(rounded_action_btn_style)
        .on_press_maybe((!s.busy).then_some(message(event)));
    let mut body = column![text("来源：当前项目与执行主机的卸载备份。原始仓库与历史 apps 未包含在备份接口中。")].spacing(12);
    for backup in &s.backups {
        body = body.push(control(
            format!("{} / {} · 时间戳 {} · {}{}", backup.name, backup.directory, backup.created_at,
                backup.backup_id, if s.selected.as_ref() == Some(&backup.backup_id) { "（已选择）" } else { "" }),
            Event::Select(backup.backup_id.clone())));
    }
    if s.backups.is_empty() { body = body.push(text("暂无备份；读取失败时可刷新重试。")); }
    body = body.push(settings_field_label("恢复目标确认"))
        .push(text("恢复到当前执行主机的技能主副本；勾选需要重新绑定的 apps。不勾选时只恢复主副本。"));
    for target in &s.available {
        let target = target.clone();
        let toggle = checkbox(s.targets.contains(&target))
            .label(format!("{:?} / {}", target.app_id, target.installation_id))
            .text_size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_checkbox_style);
        body = body.push(if s.busy { toggle } else { toggle.on_toggle(move |enabled| message(Event::Target(target.clone(), enabled))) });
    }
    body = body.push(text("恢复冲突请刷新版本后重试；恢复不会删除备份。删除备份不可撤销，请先核对已选项。"));
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    if s.busy { body = body.push(text("正在处理…")); }
    let close = message(Event::Close);
    let header = settings_modal_header(text("备份与恢复").size(20), settings_close_button(close.clone()));
    let footer = row![control("刷新".into(), Event::Refresh),
        button("确认恢复").padding([8, 10]).style(primary_action_btn_style)
            .on_press_maybe((!s.busy && s.selected.is_some() && s.version.is_some()).then_some(message(Event::Restore))),
        button("删除所选备份").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy && s.selected.is_some()).then_some(message(Event::Delete))),
        control("关闭".into(), Event::Close)].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(780).height(600);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
