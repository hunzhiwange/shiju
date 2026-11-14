//! Global application distribution controls, backed by execution-host capabilities.
use crate::app::{App, Message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::skills::bindings_view::{Event, message};
use iced::{Element, widget::{button, checkbox, column, container, row, text}};
use vw_gateway_client::vw_api_types::agent_apps_skills::bindings_api::*;

pub(super) fn view<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.skills_settings.bindings;
    if !s.open { return base; }
    let mut body = column![text("全局分发不改变 ACP 会话的技能选择或授权。"), settings_field_label("技能")].spacing(12);
    if let Some(inventory) = &s.inventory {
        for skill in &inventory.skills {
            if !skill.present { continue; }
            let selected = s.selected.as_ref() == Some(&skill.id);
            body = body.push(button(text(format!("{}{}", if selected { "✓ " } else { "" }, skill.name)))
                .padding([8, 10]).style(rounded_action_btn_style)
                .on_press_maybe((!s.busy).then(|| message(Event::Select(skill.id.clone())))));
        }
    }
    body = body.push(settings_field_label("应用与安装实例"));
    if let Some(snapshot) = &s.snapshot {
        for capability in &snapshot.capabilities {
            let catalog = app.apps_catalog.rows.iter().find(|r| r.entry.id == capability.app_id);
            let name = catalog.map(|r| r.entry.name.clone()).unwrap_or_else(|| format!("{:?}", capability.app_id));
            body = body.push(text(format!("{name}：{}", capability.reason)));
            if !capability.supported { continue; }
            let installations = catalog.and_then(|r| r.observation.as_ref().ok());
            if let Some(observation) = installations {
                if observation.installations.is_empty() { body = body.push(text("未发现可用安装实例")); }
                for installation in &observation.installations {
                    if installation.target_host_id != app.apps_catalog.host { continue; }
                    let target = SkillBindingTarget { installation_id: installation.id.0.clone(), app_id: capability.app_id };
                    body = body.push(checkbox(s.targets.contains(&target)).label(installation.id.0.clone())
                        .text_size(SETTINGS_CONTROL_TEXT_SIZE).style(settings_checkbox_style)
                        .on_toggle_maybe((!s.busy).then_some(move |enabled| message(Event::Target(target.clone(), enabled)))));
                }
            } else { body = body.push(text("应用实例尚未读取，请在应用设置中刷新")); }
        }
    }
    for result in &s.results {
        let status = match result.status { SkillBindingStatus::Enabled => "已启用", SkillBindingStatus::Disabled => "已停用", SkillBindingStatus::Failed => "失败" };
        body = body.push(text(format!("{} / {}：{status}", result.target.installation_id, result.directory)));
    }
    if s.busy { body = body.push(text("正在写入并重读，请稍候…")); }
    if let Some(error) = &s.error { body = body.push(settings_error_banner(error.clone())); }
    let action = |label, event, enabled| button(label).padding([8, 10]).style(rounded_action_btn_style)
        .on_press_maybe((!s.busy && enabled).then_some(message(event)));
    let can_bind = s.selected.is_some() && !s.targets.is_empty();
    let footer = row![action("刷新", Event::Refresh, true), action("保存目标偏好", Event::Save, s.snapshot.is_some()),
        action("启用所选", Event::Bind(true), can_bind), action("停用所选", Event::Bind(false), can_bind),
        action("重新同步", Event::Resync, !s.targets.is_empty())].spacing(8);
    let close = message(Event::Close);
    let header = settings_modal_header(text("应用技能绑定").size(20), settings_close_button(close.clone()));
    let card = settings_modal_layout(header, body, Some(footer.into())).style(super::modal_card_style).width(860).height(660);
    settings_modal_overlay(Some(base), close, container(card).padding(12))
}
