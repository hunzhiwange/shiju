//! Explicit Claude controls, isolated from plugin manifest activation.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_skills::claude_api::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub status: Option<ClaudeConfigStatus>,
    pub notice: Option<String>,
    pub error: Option<String>,
    generation: u64,
    context: Option<AgentAppsContext>,
    catalog: u64,
    project: Option<String>,
    host: String,
}
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Open, Close, Refresh,
    Marker(ClaudeConfigAction), Onboarding(ClaudeConfigAction),
    Completed(u64, Result<(ClaudeConfigResponse, Option<String>), String>),
}
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillsClaudeConfig(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.skills_settings.claude_config;
    if matches!(event, Event::Close) {
        if !state.busy {
            *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        }
        return Task::none();
    }
    let client = match crate::app::config::gateway_client() {
        Ok(client) => client,
        Err(_) => return app.show_error_toast("无法连接配置服务"),
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if matches!(event, Event::Open) {
        if state.open { return Task::none(); }
        *state = State { open: true, generation: state.generation.wrapping_add(1),
            context: Some(context.clone()), catalog: app.apps_catalog.generation,
            project: app.project_id.clone(), host: app.apps_catalog.host.clone(), ..Default::default() };
    }
    if !state.open { return Task::none(); }
    if state.context.as_ref() != Some(&context) || state.catalog != app.apps_catalog.generation
        || state.project != app.project_id || state.host != app.apps_catalog.host {
        *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    if let Event::Completed(generation, result) = event {
        if generation != state.generation || !state.busy { return Task::none(); }
        state.busy = false;
        match result {
            Ok((response, notice)) if Some(response.scope.project_id.as_ref()) == state.project.as_deref()
                && response.scope.target_host_id == state.host => {
                state.status = Some(response.status);
                state.notice = notice.clone();
                state.error = None;
                if let Some(notice) = notice {
                    return app.show_toast(crate::app::state::ToastKind::Success, notice);
                }
            }
            _ => {
                state.error = Some("操作失败；请刷新确认状态。外部配置或并发事务可能冲突，未自动覆盖。".into());
                return app.show_error_toast("Claude 配置操作失败，请刷新确认");
            }
        }
        return Task::none();
    }
    if state.busy { return Task::none(); }
    let change = match event {
        Event::Marker(action) => Some((false, action)),
        Event::Onboarding(action) => Some((true, action)),
        Event::Open | Event::Refresh => None,
        _ => return Task::none(),
    };
    // Never replace an external credential from this view.
    if matches!(event, Event::Marker(_)) && !matches!(state.status,
        Some(ClaudeConfigStatus::Missing) | Some(ClaudeConfigStatus::Valid { primary_api_key_present: false, .. })
        | Some(ClaudeConfigStatus::Valid { managed: true, .. })) { return Task::none(); }
    let Some(project) = state.project.clone() else {
        state.error = Some("请先打开已登记的项目".into()); return Task::none();
    };
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        state.error = Some("请先在应用设置中连接并授权目标主机".into()); return Task::none();
    }
    let scope = ClaudeConfigQuery { project_id: project.into(), target_host_id: state.host.clone() };
    let token = app.apps_catalog.token.clone();
    state.busy = true;
    state.error = None;
    state.notice = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Task::perform(crate::app::session_gateway::gateway_claude_config_request(scope, token, change),
        move |result| message(Event::Completed(generation, result)))
}

#[cfg(test)]
#[path = "claude_view_tests.rs"]
mod tests;
