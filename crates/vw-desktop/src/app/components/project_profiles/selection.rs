//! Read-only profile chooser; applying a profile belongs to the apply workflow.
use crate::app::{Message, state::project_profiles::State, message::project_profiles::{Event, message}};
use crate::app::components::system_settings_common::*;
use iced::{Element, widget::{button, column, text}};
use vw_gateway_client::vw_api_types::project_profiles::ProjectProfileScope;
pub(crate) fn view(state: &State, scope: ProjectProfileScope) -> Element<'_, Message> {
    let group = state.groups.get(scope);
    let mut content = column![text("项目配置组合").size(15),
        settings_text_input("按组合名称搜索", &group.query).on_input(move |query| message(Event::Query(scope, query))),
        button(text(if group.loading { "读取中" } else { "刷新组合" })).padding([8,12]).style(rounded_action_btn_style)
            .on_press_maybe((!group.loading).then(|| message(Event::Refresh(scope))))].spacing(8);
    if group.error { content = content.push(text("读取失败，请重试").style(settings_muted_text_style)); }
    if group.loaded {
        let current = group.current.as_ref().and_then(|id| group.profiles.iter().find(|p| &p.id == id));
        content = content.push(text(match (current, &group.current) {
            (Some(profile), _) => format!("当前组合：{}{}", profile.name, if profile.payload.scope_captured(scope) { "" } else { " · 未拍快照" }),
            (None, Some(_)) => "当前组合不可用，请刷新".into(),
            _ => "尚未选择当前组合".into(),
        }));
        let query = group.query.trim().to_lowercase();
        let mut count = 0;
        for profile in &group.profiles {
            if !profile.name.to_lowercase().contains(&query) { continue; }
            count += 1;
            content = content.push(text(format!("{}{}{}", profile.name,
                if group.current.as_ref() == Some(&profile.id) { " · 当前" } else { "" },
                if profile.payload.scope_captured(scope) { "" } else { " · 未拍快照" })));
        }
        if count == 0 { content = content.push(text("没有匹配的组合").style(settings_muted_text_style)); }
    }
    content.into()
}
