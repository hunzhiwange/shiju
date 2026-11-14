//! Read-only market inside the existing skills settings entry point.
use crate::app::{App, Message};
use crate::app::message::settings::skills::market_view::{Event, Results, message};
use crate::app::components::system_settings_common::*;
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.market;
    if !s.open { return base; }
    let mut body = column![
        settings_field_label("市场搜索（留空发现已配置仓库）"),
        settings_text_input("搜索技能", &s.query).on_input(|q| message(Event::Query(q)))
            .on_submit(message(Event::Refresh)),
    ].spacing(10).width(Length::Fill);
    if s.busy { body = body.push(text("正在加载…")); }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    match &s.results {
        Some(Results::Discovery(report)) => {
            for failure in &report.failures {
                body = body.push(settings_error_banner(format!("仓库 {}/{}（{}）发现失败：{:?}",
                    failure.owner, failure.name, failure.branch, failure.error)));
            }
            for skill in &report.skills {
                body = body.push(settings_group_card(column![text(skill.name.clone()),
                    text(skill.description.clone()),
                    text(format!("{}/{} · {}", skill.repo_owner, skill.repo_name, skill.repo_branch))].spacing(6)));
            }
            if report.skills.is_empty() {
                body = body.push(text(if report.failures.is_empty() { "未发现技能" }
                    else { "没有可展示的技能；部分仓库失败，请查看上方原因" }));
            }
        }
        Some(Results::Search(report)) => {
            body = body.push(text(format!("共 {} 项，当前显示 {} 项", report.total_count, report.skills.len())));
            for skill in &report.skills {
                body = body.push(settings_group_card(column![text(skill.name.clone()),
                    text(format!("{}/{} · {} · 安装次数 {}", skill.repo_owner, skill.repo_name,
                        skill.repo_branch, skill.installs))].spacing(6)));
            }
            if report.skills.is_empty() { body = body.push(text("没有匹配的技能")); }
        }
        None => {}
    }
    let close = message(Event::Close);
    let header = settings_modal_header(text("技能市场").size(20), settings_close_button(close.clone()));
    let footer = row![
        button(if s.query.trim().is_empty() { "发现仓库技能" } else { "搜索" })
            .padding([8, 10]).style(primary_action_btn_style)
            .on_press_maybe((!s.busy).then_some(message(Event::Refresh))),
        button("关闭").padding([8, 10]).style(rounded_action_btn_style).on_press(close.clone()),
    ].spacing(8);
    let card = settings_modal_layout(header, body, Some(footer.into()))
        .style(super::modal_card_style).width(760).height(580);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
