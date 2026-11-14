//! Read-only discovery and market search share one request generation.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_skills::{discovery_api::*, search_api::*};

#[derive(Debug, Clone)]
pub(crate) enum Results { Discovery(DiscoverSkillsResponse), Search(MarketSearchResponse) }
#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub query: String,
    pub results: Option<Results>,
    pub error: Option<String>,
    generation: u64,
    context: Option<AgentAppsContext>,
    catalog: u64,
    project: Option<String>,
    host: String,
    version: u64,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Query(String), Refresh,
    Completed(u64, Result<Results, String>),
}
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillsMarket(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.skills_settings.market;
    if matches!(event, Event::Close) {
        *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
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
    if let Event::Query(query) = event {
        state.query = query;
        state.generation = state.generation.wrapping_add(1);
        state.busy = false;
        state.results = None;
        state.error = None;
        return Task::none();
    }
    if let Event::Completed(generation, result) = event {
        if generation != state.generation || !state.busy { return Task::none(); }
        state.busy = false;
        let configured_version = app.skills_settings.repositories.snapshot.as_ref()
            .filter(|s| Some(s.scope.project_id.as_ref()) == state.project.as_deref()
                && s.scope.target_host_id == state.host).map_or(0, |s| s.version);
        match result {
            Ok(Results::Discovery(report)) if report.version < state.version.max(configured_version) => {
                state.error = Some("仓库配置已更新，请重新发现".into());
            }
            Ok(Results::Search(report)) if report.query != state.query.trim() => {
                state.error = Some("搜索响应与当前查询不匹配，请重试".into());
            }
            Ok(results) => {
                if let Results::Discovery(report) = &results { state.version = report.version; }
                state.results = Some(results);
                state.error = None;
            }
            Err(error) => {
                state.error = Some(error.clone());
                return app.show_error_toast(error);
            }
        }
        return Task::none();
    }
    if state.busy { return Task::none(); }
    let Some(project) = state.project.clone() else {
        state.error = Some("请先打开已登记的项目".into()); return Task::none();
    };
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        state.error = Some("请先在应用设置中连接并授权目标主机".into()); return Task::none();
    }
    state.busy = true;
    state.results = None;
    state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Task::perform(crate::app::session_gateway::gateway_skills_market_request(
        project, state.host.clone(), state.query.trim().to_owned(), app.apps_catalog.token.clone()),
        move |result| message(Event::Completed(generation, result)))
}

#[cfg(test)]
#[path = "market_view_tests.rs"]
mod tests;
