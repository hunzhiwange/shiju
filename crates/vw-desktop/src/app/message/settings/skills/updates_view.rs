//! Scoped update results; refresh errors never replace a completed operation.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_api_app_skills::{repos_api::SkillReposQuery, updates_api::CheckSkillUpdatesRequest};

pub(crate) use desktop_settings_skills::updates_view::{State, Operation, Outcome, Event};
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsUpdates(event)) }

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(), tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let s = &mut app.skills_settings.updates;
    if matches!(event, Event::Close) {
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Task::none();
    }
    if matches!(event, Event::Open) && !s.open {
        let Some(project) = app.project_id.clone() else { return app.show_error_toast("请先打开已登记的项目"); };
        *s = State { open: true, context: Some(context.clone()), catalog: app.apps_catalog.generation,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: app.apps_catalog.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Task::none(); }
    if s.context.as_ref() != Some(&context) || s.catalog != app.apps_catalog.generation
        || s.scope.as_ref().is_none_or(|q| Some(q.project_id.as_ref()) != app.project_id.as_deref() || q.target_host_id != app.apps_catalog.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    match event {
        Event::Loaded(g, inventory, updates) if g == s.generation && s.busy => {
            s.busy = false; s.loaded(inventory, updates);
            return match s.error.clone() { Some(e) => app.show_error_toast(e), None => Task::none() };
        }
        Event::Finished(g, result) if g == s.generation && s.busy => {
            s.busy = false; s.finished(result);
            return Task::batch(vec![update(app, Event::Refresh), super::refresh_task(app.project_path.clone())]);
        }
        Event::Loaded(..) | Event::Finished(..) => return Task::none(),
        _ if s.busy => return Task::none(),
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        return app.show_error_toast("请先在应用设置中连接并授权目标主机");
    }
    let scope = s.scope.clone().expect("open scope");
    let operation = if let Event::Run(operation) = event {
        let Some(inventory) = &s.inventory else { return Task::none(); };
        let id = match &operation { Operation::Update(id) | Operation::Uninstall(id) => id };
        if !inventory.skills.iter().any(|skill| &skill.id == id) { return Task::none(); }
        let hash = match &operation {
            Operation::Update(id) => {
                let Some(cache) = &s.updates else { return Task::none(); };
                let Some(info) = cache.updates.iter().find(|u| &u.directory == id) else { return Task::none(); };
                let Some(hash) = info.current_hash.clone() else { return Task::none(); };
                Some(hash)
            }
            Operation::Uninstall(_) => None,
        };
        Some((operation, inventory.version, hash))
    } else { None };
    s.busy = true; s.error = None; s.generation = s.generation.wrapping_add(1);
    let g = s.generation;
    let token = app.apps_catalog.token.clone();
    if let Some(operation) = operation {
        s.backup = None; s.outcome = None;
        return Task::perform(crate::app::session_gateway::gateway_skills_mutation(scope, operation, token), move |r| message(Event::Finished(g, r)));
    }
    s.inventory = None; s.updates = None;
    Task::perform(async move {
        let inventory = client.agent_apps_skills_inventory(&scope, &token).await.map_err(|e| e.to_string());
        let updates = client.agent_apps_skills_check_updates(&CheckSkillUpdatesRequest { scope }, &token).await.map_err(|e| e.to_string());
        (inventory, updates)
    }, move |(i, u)| message(Event::Loaded(g, i, u)))
}
