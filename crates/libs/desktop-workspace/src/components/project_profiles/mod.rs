//! Profile chooser and maintenance; applying belongs to the apply workflow.
use crate::project_profiles::{State, Action};
use desktop_ui_controls::system_settings_common::*;
mod apply;
#[path = "management.rs"]
mod management;
use iced::{
    Alignment, Element, Length,
    widget::{button, column, row, text},
};
use vw_api_project_profiles::ProjectProfileScope;
pub fn view<Identity>(i18n: desktop_i18n::I18n, state: &State<Identity>, scope: ProjectProfileScope) -> Element<'_, Event> {
    let group = state.groups.get(scope);
    let enabled = !group.loading && state.management.is_none() && state.application.is_none();
    let mut actions = row![
        button(
            text(if group.loading { i18n.__("读取中…") } else { i18n.__("刷新") }).size(SETTINGS_CONTROL_TEXT_SIZE)
        )
        .padding([8, 12])
        .style(rounded_action_btn_style)
        .on_press_maybe((!group.loading).then_some(Event::Refresh(scope))),
        button(text(i18n.__("创建组合")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding([8, 12])
            .style(rounded_action_btn_style)
            .on_press_maybe(enabled.then_some(Event::Open(scope, Action::Create))),
    ]
    .spacing(8);
    if group.current.is_some() {
        actions = actions.push(
            button(text(i18n.__("清除当前标记")).size(SETTINGS_CONTROL_TEXT_SIZE))
                .padding([8, 12])
                .style(rounded_action_btn_style)
                .on_press_maybe(enabled.then_some(Event::Open(scope, Action::Clear))),
        );
    }
    let mut content = column![
        row![text(i18n.__("项目配置组合")).size(16).width(Length::Fill), actions.wrap()]
            .spacing(10)
            .align_y(Alignment::Center),
        settings_text_input(i18n.__("按组合名称搜索"), &group.query)
            .on_input(move |query| Event::Query(scope, query)),
    ]
    .spacing(10);
    if group.error {
        content = content.push(text(i18n.__("读取失败，请重试")).style(settings_muted_text_style));
    }
    if group.loaded {
        let current = group.current.as_ref().and_then(|id| group.profiles.iter().find(|p| &p.id == id));
        content = content.push(text(match (current, &group.current) {
            (Some(profile), _) => i18n.format("当前组合：%s%s", &[profile.name.to_string().into(), (if profile.payload.scope_captured(scope) { "" } else { i18n.__(" · 未拍快照") }).to_string().into()]).expect("valid workbench template"),
            (None, Some(_)) => i18n.__("当前组合不可用，请刷新").into(),
            _ => i18n.__("尚未选择当前组合").into(),
        }));
        if group.result_error {
            content = content.push(text(i18n.__("最近保存结果读取失败，请刷新重试")).style(settings_muted_text_style));
        } else if let Some(result) = &group.stored_result {
            use vw_api_project_profiles::ProjectProfileApplyStatus;
            let status = match result.status {
                ProjectProfileApplyStatus::Applied => i18n.__("已应用"),
                ProjectProfileApplyStatus::Partial => i18n.__("部分应用失败"),
                ProjectProfileApplyStatus::Uncaptured => i18n.__("未拍快照"),
                ProjectProfileApplyStatus::Cancelled => i18n.__("已取消"),
                ProjectProfileApplyStatus::Failed => i18n.__("失败"),
            };
            content = content.push(text(i18n.format("最近保存结果：%s（历史记录，不确认本次请求）", &[status.to_string().into()]).expect("valid workbench template")));
            content = content.push(text(i18n.format("组合：%s · %s", &[result.profile_id.0.to_string().into(), (if result.selected { i18n.__("当时已选择") } else { i18n.__("当时未确认选择") }).to_string().into()]).expect("valid workbench template")));
            for item in &result.items {
                content = content.push(text(format!("{:?} · {} · {:?}{}", item.category,
                    item.reference_id.as_deref().unwrap_or(i18n.__("空集合")), item.status,
                    item.reason_code.as_ref().map(|code| format!(" · {code}")).unwrap_or_default())));
            }
        } else {
            content = content.push(text(i18n.__("暂无最近保存的组合结果")).style(settings_muted_text_style));
        }
        let query = group.query.trim().to_lowercase();
        let mut count = 0;
        for profile in &group.profiles {
            if !profile.name.to_lowercase().contains(&query) { continue; }
            count += 1;
            content = content.push(text(format!("{}{}{}", profile.name,
                if group.current.as_ref() == Some(&profile.id) { i18n.__(" · 当前") } else { "" },
                if profile.payload.scope_captured(scope) { "" } else { i18n.__(" · 未拍快照") })));
            content = content.push(apply::action(i18n, profile, scope, enabled));
            content = content.push(management::actions(i18n, profile, scope, enabled));
        }
        if count == 0 { content = content.push(text(i18n.__("没有匹配的组合")).style(settings_muted_text_style)); }
    }
    apply::overlay(i18n, state, scope, management::overlay(i18n, state, scope, content.into()))
}

#[derive(Clone)]
pub enum Event {
    Apply(vw_api_project_profiles::ProjectProfileScope, ApplyEvent),
    Open(vw_api_project_profiles::ProjectProfileScope, Action),
    Name(vw_api_project_profiles::ProjectProfileScope, String),
    Close(vw_api_project_profiles::ProjectProfileScope),
    Submit(vw_api_project_profiles::ProjectProfileScope),
    Refresh(vw_api_project_profiles::ProjectProfileScope),
    Query(vw_api_project_profiles::ProjectProfileScope, String),
}
#[derive(Clone)]
pub enum ApplyEvent {
    Open(vw_api_project_profiles::ProjectProfileId, u64), Close, Confirm, Recover, Cancel,
}
