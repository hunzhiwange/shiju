//! Execution-host storage and synchronization controls.
use crate::app::{App, Message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::skills::storage_modal::{Event, message};
use iced::{Element, widget::{button, column, container, row, text}};
use vw_gateway_client::vw_api_types::agent_apps_skills::{bindings_api::SkillBindingMode, storage_api::SkillStorageLocation};

pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.storage;
    if !s.open { return base; }
    let control = |label, event| button(label).padding([8, 10]).style(rounded_action_btn_style)
        .on_press_maybe((!s.busy).then_some(message(event)));
    let mut body = column![settings_field_label("当前主副本位置")].spacing(12);
    if let Some(current) = &s.current {
        body = body.push(text(&current.root)).push(text(format!("当前同步模式：{:?}", current.mode)));
    } else { body = body.push(text("尚未读取")); }
    body = body.push(settings_field_label("待选位置"))
        .push(row![control("运行时目录", Event::Select(SkillStorageLocation::Runtime)),
            control(".agents 目录", Event::Select(SkillStorageLocation::Agents))].spacing(8))
        .push(text(match s.pending { Some(SkillStorageLocation::Runtime) => "待迁移到运行时目录",
            Some(SkillStorageLocation::Agents) => "待迁移到 .agents 目录", None => "请选择迁移目标" }))
        .push(text("点击移动并切换后开始迁移；失败保留当前位置。"))
        .push(settings_field_label("同步模式"))
        .push(row![control("Auto", Event::Mode(SkillBindingMode::Auto)),
            control("Symlink", Event::Mode(SkillBindingMode::Symlink)),
            control("Copy", Event::Mode(SkillBindingMode::Copy))].spacing(8))
        .push(text("重新同步使用已保存的应用绑定目标。远程目标目录不可在本机打开。"));
    for result in &s.results { body = body.push(text(result)); }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    if s.busy { body = body.push(text("正在处理，请稍候…")); }
    let close = message(Event::Close);
    let header = settings_modal_header(text("存储与同步").size(20), settings_close_button(close.clone()));
    let footer = row![control("刷新", Event::Refresh), control("打开目录", Event::OpenDirectory), control("重新同步", Event::Resync),
        button("移动并切换").padding([8, 10]).style(primary_action_btn_style)
            .on_press_maybe((!s.busy && s.current.is_some() && s.pending.is_some()).then_some(message(Event::Move))),
        control("关闭", Event::Close)].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(760).height(580);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
