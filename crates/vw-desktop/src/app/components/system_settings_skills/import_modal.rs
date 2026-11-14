//! Theme-aware unmanaged import confirmation; source paths are read-only.
use crate::app::{App, Message};
use crate::app::message::settings::skills::import_modal::{Event, APPS, eligible, message};
use crate::app::components::system_settings_common::*;
use iced::{Element, Length};
use iced::widget::{button, checkbox, column, container, row, text};
pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.import;
    if !s.open { return base; }
    let mut body = column![text("选择来源应用，扫描后勾选需要导入的技能。")].spacing(10);
    for &id in APPS {
        body = body.push(checkbox(s.apps.contains(&id)).label(format!("{id:?}"))
            .text_size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_checkbox_style)
            .on_toggle_maybe((!s.busy).then_some(move |v| message(Event::App(id, v)))));
    }
    if let Some(report) = &s.report {
        for skill in &report.skills {
            let id = skill.directory.clone();
            let reason = if skill.managed { "已托管，跳过" }
                else if skill.conflict { "来源冲突，跳过" }
                else if skill.external_link { "外部链接，跳过" }
                else if !eligible(skill) { "缺少可用扫描令牌，跳过" } else { "可导入" };
            body = body.push(column![
                checkbox(s.selected.contains(&id)).label(skill.name.clone())
                    .text_size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_checkbox_style)
                    .on_toggle_maybe((!s.busy && eligible(skill)).then_some(move |v| message(Event::Select(id.clone(), v)))),
                text(format!("来源目录：{}", skill.directory)),
                text(format!("foundIn: {:?} · {}", skill.apps, reason)),
            ].spacing(5));
        }
        if report.skills.is_empty() { body = body.push(text("未发现未托管技能")); }
    }
    for (id, result) in &s.outcomes {
        body = body.push(text(format!("{}：{}", id, match result {
            Ok(()) => "已导入".to_owned(), Err(error) => format!("失败，可重试：{error}"),
        })));
    }
    if s.busy { body = body.push(text("正在扫描或导入…")); }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    let close = message(Event::Close);
    let header = settings_modal_header(text("导入未托管技能").size(20), settings_close_button(close.clone()));
    let footer = row![
        button("扫描 / 刷新").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy && !s.apps.is_empty()).then_some(message(Event::Scan))),
        button("导入所选 / 重试").padding([8, 10]).style(primary_action_btn_style)
            .on_press_maybe((!s.busy && !s.selected.is_empty()).then_some(message(Event::Import))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ].spacing(8);
    let card = settings_modal_layout(header, body.width(Length::Fill), Some(footer.into()))
        .style(super::modal_card_style).width(760).height(620);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
