//! Host-scoped global distribution; independent of ACP session selection.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_skills::{bindings_api::*, inventory_api::*, repos_api::SkillReposQuery};

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub snapshot: Option<SkillBindingSnapshot>,
    pub inventory: Option<SkillInventoryResponse>,
    pub targets: Vec<SkillBindingTarget>,
    pub selected: Option<String>,
    pub results: Vec<SkillBindingItem>,
    pub error: Option<String>,
    generation: u64,
    catalog: u64,
    context: Option<AgentAppsContext>,
    scope: Option<SkillReposQuery>,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Refresh, Select(String), Target(SkillBindingTarget, bool), Save,
    Bind(bool), Resync,
    Completed(u64, Result<Readback, String>),
}
#[derive(Debug, Clone)]
pub(crate) struct Readback {
    pub snapshot: Option<SkillBindingSnapshot>,
    pub inventory: Option<SkillInventoryResponse>,
    pub results: Vec<SkillBindingItem>,
    pub error: Option<String>,
    pub changed: bool,
}
pub(crate) enum Change { Save(SkillBindingPreferences), Bind(String, SkillBindingsRequest), Resync(SkillResyncRequest) }
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsBindings(event)) }

/// Missing, duplicate and unexpected results can never count as complete success.
pub(crate) fn complete_success(results: &[SkillBindingItem], targets: &[SkillBindingTarget], id: &str, enabled: bool) -> bool {
    !targets.is_empty() && results.len() == targets.len() && targets.iter().all(|target| {
        results.iter().filter(|item| &item.target == target && item.directory == id && item.status ==
            if enabled { SkillBindingStatus::Enabled } else { SkillBindingStatus::Disabled }).count() == 1
    })
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let s = &mut app.skills_settings.bindings;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Task::none();
    }
    let client = match crate::app::config::gateway_client() {
        Ok(client) => client,
        Err(error) => return app.show_error_toast(error),
    };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if matches!(event, Event::Open) {
        if s.open { return Task::none(); }
        let Some(project) = app.project_id.clone() else { return app.show_error_toast("请先打开已登记的项目"); };
        *s = State { open: true, context: Some(context.clone()), catalog: app.apps_catalog.generation,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: app.apps_catalog.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Task::none(); }
    if s.context.as_ref() != Some(&context) || s.catalog != app.apps_catalog.generation || s.scope.as_ref().is_none_or(|scope|
        Some(scope.project_id.as_ref()) != app.project_id.as_deref() || scope.target_host_id != app.apps_catalog.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    if let Event::Completed(generation, result) = event {
        if generation != s.generation || !s.busy { return Task::none(); }
        s.busy = false;
        match result {
            Ok(data) if data.snapshot.as_ref().is_none_or(|v| Some(&v.scope) == s.scope.as_ref())
                && data.inventory.as_ref().is_none_or(|v| Some(&v.scope) == s.scope.as_ref()) => {
                if let Some(snapshot) = data.snapshot { s.targets = snapshot.targets.clone(); s.snapshot = Some(snapshot); }
                if let Some(inventory) = data.inventory { s.inventory = Some(inventory); }
                s.results = data.results;
                s.error = data.error;
                if let Some(error) = s.error.clone() { return app.show_error_toast(error); }
                if data.changed { return app.show_toast(crate::app::state::ToastKind::Success, "操作完成，已重读目标状态"); }
            }
            Err(error) => {
                s.error = Some(error.clone());
                return app.show_error_toast(error);
            }
            Ok(_) => { s.error = Some("重读失败，请刷新确认；已发生的写入不会自动回滚".into());
                return app.show_error_toast("重读失败，请刷新确认目标状态"); }
        }
        return Task::none();
    }
    if s.busy { return Task::none(); }
    match event {
        Event::Select(id) => { if s.inventory.as_ref().is_some_and(|data| data.skills.iter().any(|item| item.id == id && item.present)) { s.selected = Some(id); } return Task::none(); }
        Event::Target(target, enabled) => {
            let supported = s.snapshot.as_ref().is_some_and(|data| data.capabilities.iter().any(|c| c.app_id == target.app_id && c.supported));
            let exists = app.apps_catalog.rows.iter().filter_map(|r| r.observation.as_ref().ok()).any(|r| r.installations.iter().any(|i|
                i.id.0 == target.installation_id && i.app_id == target.app_id && i.target_host_id == app.apps_catalog.host));
            if supported && exists { s.targets.retain(|t| t != &target); if enabled { s.targets.push(target); } }
            return Task::none();
        }
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        s.error = Some("请先在应用设置中连接并授权目标主机".into());
        return app.show_error_toast("请先在应用设置中连接并授权目标主机");
    }
    let scope = s.scope.clone().expect("open scope");
    let change = match event {
        Event::Save => { let Some(snapshot) = &s.snapshot else { return Task::none(); };
            Some(Change::Save(SkillBindingPreferences { scope: scope.clone(), expected_version: snapshot.version, targets: s.targets.clone() })) }
        Event::Bind(enabled) => { let Some(id) = &s.selected else { return Task::none(); };
            if s.targets.is_empty() { return Task::none(); }
            Some(Change::Bind(id.clone(), SkillBindingsRequest { scope: scope.clone(), targets: s.targets.clone(), enabled, mode: SkillBindingMode::Auto })) }
        Event::Resync => { if s.targets.is_empty() { return Task::none(); }
            Some(Change::Resync(SkillResyncRequest { scope: scope.clone(), targets: s.targets.clone() })) }
        Event::Open | Event::Refresh => None,
        _ => return Task::none(),
    };
    s.busy = true;
    s.error = None;
    s.generation = s.generation.wrapping_add(1);
    let generation = s.generation;
    let token = app.apps_catalog.token.clone();
    Task::perform(crate::app::session_gateway::gateway_skill_bindings_request(scope, token, change), move |result| message(Event::Completed(generation, result)))
}

#[cfg(test)]
#[path = "bindings_view_tests.rs"]
mod tests;
