//! Repository settings share the standard theme-aware, scrollable modal layout.
use crate::app::{App, Message, message::SettingsMessage};
use crate::app::message::settings::skills::repo_modal::Event;
use crate::app::components::system_settings_common::*;
use iced::widget::{button, column, container, row, text};
use iced::Element;
fn event(e: Event) -> Message { Message::Settings(SettingsMessage::SkillRepositories(e)) }
pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.repositories;
    if !s.open { return base; }
    let mut body = column![
        settings_field_label("仓库（owner/name）"),
        settings_text_input("owner/name", &s.identity)
            .on_input_maybe((!s.busy).then_some(|v| event(Event::Identity(v)))),
        settings_field_label("分支（留空使用默认分支）"),
        settings_text_input("", &s.branch)
            .on_input_maybe((!s.busy).then_some(|v| event(Event::Branch(v)))),
        button("添加仓库").padding([8, 10]).style(primary_action_btn_style)
            .on_press_maybe((!s.busy && s.snapshot.is_some()).then_some(event(Event::Add))),
        super::deeplink::view(app),
    ].spacing(10);
    if let Some(snapshot) = &s.snapshot {
        for (index, repo) in snapshot.repos.iter().enumerate() {
            body = body.push(row![
                container(text(format!("{}/{} · {}", repo.owner, repo.name, repo.branch)))
                    .width(iced::Length::Fill),
                button(if repo.enabled { "停用" } else { "启用" }).padding([8, 10])
                    .style(rounded_action_btn_style)
                    .on_press_maybe((!s.busy).then_some(event(Event::Toggle(index)))),
                button("删除").padding([8, 10]).style(rounded_action_btn_style)
                    .on_press_maybe((!s.busy).then_some(event(Event::Remove(index)))),
            ].spacing(8).align_y(iced::Alignment::Center));
        }
        if snapshot.repos.is_empty() { body = body.push(text("尚未配置仓库")); }
    }
    if s.busy { body = body.push(text("正在同步仓库…")); }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    let close = event(Event::Close);
    let header = settings_modal_header(text("仓库管理").size(20), settings_close_button(close.clone()));
    let footer = row![
        button("刷新").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(event(Event::Refresh))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style)
            .on_press_maybe((!s.busy).then_some(close.clone())),
    ].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(760).height(580);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
