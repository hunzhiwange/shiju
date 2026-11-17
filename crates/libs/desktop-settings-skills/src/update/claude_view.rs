//! Explicit Claude controls, isolated from plugin manifest activation.
use desktop_provider_apps::AgentAppsContext;
use super::Input;
use vw_api_app_skills::claude_api::*;

#[derive(Clone, Debug, Default)]
pub struct State {
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
pub enum Event {
    Open, Close, Refresh,
    Marker(ClaudeConfigAction), Onboarding(ClaudeConfigAction),
    Completed(u64, Result<(ClaudeConfigResponse, Option<String>), String>),
}


pub enum Effect { None, Error(String), Success(String),  Request { generation: u64, scope: ClaudeConfigQuery, change: Option<(bool, ClaudeConfigAction)> } }
pub fn update(state: &mut State, input: &Input, event: Event) -> Effect {

    if matches!(event, Event::Close) {
        if !state.busy {
            *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        }
        return Effect::None;
    }
    let Some(context) = input.context.clone() else { return Effect::Error(("无法连接配置服务").into()); };
    if matches!(event, Event::Open) {
        if state.open { return Effect::None; }
        *state = State { open: true, generation: state.generation.wrapping_add(1),
            context: Some(context.clone()), catalog: input.catalog,
            project: input.project.clone(), host: input.host.clone(), ..Default::default() };
    }
    if !state.open { return Effect::None; }
    if state.context.as_ref() != Some(&context) || state.catalog != input.catalog
        || state.project != input.project || state.host != input.host {
        *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        return Effect::None;
    }
    if let Event::Completed(generation, result) = event {
        if generation != state.generation || !state.busy { return Effect::None; }
        state.busy = false;
        match result {
            Ok((response, notice)) if Some(response.scope.project_id.as_ref()) == state.project.as_deref()
                && response.scope.target_host_id == state.host => {
                state.status = Some(response.status);
                state.notice = notice.clone();
                state.error = None;
                if let Some(notice) = notice {
                    return Effect::Success(notice);
                }
            }
            _ => {
                state.error = Some("操作失败；请刷新确认状态。外部配置或并发事务可能冲突，未自动覆盖。".into());
                return Effect::Error(("Claude 配置操作失败，请刷新确认").into());
            }
        }
        return Effect::None;
    }
    if state.busy { return Effect::None; }
    let change = match event {
        Event::Marker(action) => Some((false, action)),
        Event::Onboarding(action) => Some((true, action)),
        Event::Open | Event::Refresh => None,
        _ => return Effect::None,
    };
    // Never replace an external credential from this view.
    if matches!(event, Event::Marker(_)) && !matches!(state.status,
        Some(ClaudeConfigStatus::Missing) | Some(ClaudeConfigStatus::Valid { primary_api_key_present: false, .. })
        | Some(ClaudeConfigStatus::Valid { managed: true, .. })) { return Effect::None; }
    let project = vw_api_transport::local_settings_project(state.project.as_deref()).to_owned();
    let scope = ClaudeConfigQuery { project_id: project.into(), target_host_id: state.host.clone() };
    state.busy = true;
    state.error = None;
    state.notice = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Effect::Request { generation, scope, change }
}

#[cfg(test)]
#[path = "claude_view_tests.rs"]
mod tests;
