//! Scoped inventory and explicit installation confirmation, alongside built-in skills.
use crate::app::{App, Message};
use crate::app::message::settings::skills::install_view::{Event, message};
use crate::app::components::system_settings_common::*;
use iced::{Element, Length};
use iced::widget::{button, column, container, row, text};

pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.install;
    if !s.open { return base; }
    let mut body = column![text(format!("目标主机：{}", app.apps_catalog.host)),
        text("从已登记仓库安装。安装只写入该主机技能目录；应用分发需另行选择。"),
        settings_field_label("仓库标识（owner:name）"),
        settings_text_input("owner:name", &s.repo).on_input(|v| message(Event::Repo(v))),
        settings_field_label("仓库内来源目录"),
        settings_text_input("来源目录", &s.selection).on_input(|v| message(Event::Selection(v))),
        settings_field_label("安装目录名"),
        settings_text_input("单段目录名", &s.directory).on_input(|v| message(Event::Directory(v))),
    ].spacing(10).width(Length::Fill);
    if s.busy { body = body.push(text("正在读取或安装，请等待任务结束…")); }
    if let Some(e) = &s.error { body = body.push(settings_error_banner(e.clone())); }
    if let Some(data) = &s.inventory {
        body = body.push(text(format!("库存版本：{}", data.version)));
        for skill in &data.skills {
            body = body.push(button(text(skill.name.clone())).padding([8, 10])
                .style(rounded_action_btn_style).on_press(message(Event::Select(skill.id.clone()))));
        }
        if let Some(detail) = data.skills.iter().find(|item| Some(&item.id) == s.selected.as_ref()) {
            body = body.push(settings_group_card(column![
                text(detail.name.clone()),
                text(format!("来源：{}", detail.source.as_deref().unwrap_or("本地或归档"))),
                text(detail.description.clone().unwrap_or_default()),
                text(format!("版本：{}", detail.version.as_deref().unwrap_or("未提供"))),
                text(format!("内容哈希：{}", detail.content_hash.as_deref().unwrap_or("未提供"))),
                text(detail.markdown.clone().unwrap_or_default()),
            ].spacing(6)));
        }
    }
    let close = message(Event::Close);
    let header = settings_modal_header(text("技能详情与安装").size(20), settings_close_button(close.clone()));
    let footer = row![
        button("确认安装到此主机").padding([8, 10]).style(primary_action_btn_style)
            .on_press_maybe((!s.busy && s.inventory.is_some()).then_some(message(Event::Install))),
        button("刷新").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(760).height(600);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
