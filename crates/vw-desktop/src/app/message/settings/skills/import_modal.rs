//! Unmanaged imports use server selections and explicit application choices.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::{agent_apps::AppId, agent_apps_skills::import_api::*};

pub(crate) const APPS: &[AppId] = &[AppId::Claude, AppId::Codex, AppId::Gemini,
    AppId::GrokBuild, AppId::OpenCode, AppId::Hermes, AppId::Pi, AppId::Mcode,
    AppId::Kimi, AppId::Dsh, AppId::Agy, AppId::Omp];
#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub apps: Vec<AppId>,
    pub selected: Vec<String>,
    pub report: Option<UnmanagedSkillsResponse>,
    pub outcomes: Vec<(String, Result<(), String>)>,
    pub error: Option<String>,
    generation: u64,
    context: Option<AgentAppsContext>,
    catalog: u64,
    project: Option<String>,
    host: String,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, App(AppId, bool), Select(String, bool), Scan, Import,
    Completed(u64, Vec<(String, Result<(), String>)>, Result<UnmanagedSkillsResponse, String>),
}
pub(crate) fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillsImport(event))
}
pub(crate) fn eligible(row: &UnmanagedSkillItem) -> bool {
    !row.managed && !row.conflict && !row.external_link && row.selection_id.is_some()
        && row.content_hash.is_some()
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let state = &mut app.skills_settings.import;
    if matches!(event, Event::Close) {
        if !state.busy { *state = State { generation: state.generation.wrapping_add(1), ..Default::default() }; }
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
        return Task::none();
    }
    if !state.open { return Task::none(); }
    if state.context.as_ref() != Some(&context) || state.catalog != app.apps_catalog.generation
        || state.project != app.project_id || state.host != app.apps_catalog.host {
        *state = State { generation: state.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    if let Event::Completed(generation, outcomes, result) = event {
        if generation != state.generation || !state.busy { return Task::none(); }
        state.busy = false;
        for (id, result) in outcomes {
            if result.is_ok() { state.selected.retain(|selected| selected != &id); }
            state.outcomes.retain(|(old, _)| old != &id);
            state.outcomes.push((id, result));
        }
        match result {
            Ok(report) if Some(report.scope.project_id.as_ref()) == state.project.as_deref()
                && report.scope.target_host_id == state.host => {
                state.report = Some(report); state.error = None;
            }
            Ok(_) => state.error = Some("导入响应作用域不匹配，请重新扫描".into()),
            Err(error) => { state.error = Some(error.clone()); return app.show_error_toast(error); }
        }
        return super::refresh_task(app.project_path.clone());
    }
    if state.busy { return Task::none(); }
    match event {
        Event::App(id, enabled) => {
            if !APPS.contains(&id) { return Task::none(); }
            state.apps.retain(|app| *app != id);
            if enabled { state.apps.push(id); }
            state.report = None; state.selected.clear(); state.error = None;
            return Task::none();
        }
        Event::Select(id, enabled) => {
            if !state.report.as_ref().is_some_and(|r| r.skills.iter()
                .any(|s| s.directory == id && eligible(s))) { return Task::none(); }
            state.selected.retain(|old| old != &id);
            if enabled { state.selected.push(id); }
            return Task::none();
        }
        Event::Scan | Event::Import => {}
        _ => return Task::none(),
    }
    let Some(project) = state.project.clone() else {
        state.error = Some("请先打开已登记的项目".into()); return Task::none();
    };
    if state.apps.is_empty() || app.apps_catalog.context.as_ref() != Some(&context)
        || app.apps_catalog.token.is_empty() {
        state.error = Some("请选择应用并在应用设置中授权目标主机".into()); return Task::none();
    }
    let selected: Vec<_> = if matches!(event, Event::Import) {
        state.report.as_ref().map(|r| r.skills.iter().filter(|s| eligible(s)
            && state.selected.contains(&s.directory)).cloned().collect()).unwrap_or_default()
    } else { Vec::new() };
    if matches!(event, Event::Import) && selected.is_empty() { return Task::none(); }
    state.busy = true; state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Task::perform(crate::app::session_gateway::gateway_skills_import_request(
        project, state.host.clone(), state.apps.clone(), selected, app.apps_catalog.token.clone()),
        move |(outcomes, result)| message(Event::Completed(generation, outcomes, result)))
}
