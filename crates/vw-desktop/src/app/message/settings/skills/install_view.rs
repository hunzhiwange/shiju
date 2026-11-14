//! One explicit host/source request per confirmation; credentials never enter events.
use crate::app::{App, Message, message::SettingsMessage, state::AgentAppsContext};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_skills::{inventory_api::*, install_api::*, repos_api::SkillReposQuery};

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub inventory: Option<SkillInventoryResponse>,
    pub selected: Option<String>,
    pub repo: String,
    pub selection: String,
    pub directory: String,
    pub error: Option<String>,
    generation: u64,
    context: Option<AgentAppsContext>,
    scope: Option<SkillReposQuery>,
    catalog: u64,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Refresh, Select(String), Repo(String), Selection(String), Directory(String), Install,
    Loaded(u64, Result<SkillInventoryResponse, String>),
    Installed(u64, Result<(), String>),
    Submitted(u64, Result<InstallSkillResponse, String>),
}
pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::SkillsInstall(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    if matches!(event, Event::Install) && app.skills_settings.jobs.task_id.is_some()
        && app.skills_settings.jobs.snapshot.as_ref().is_none_or(|job| !super::jobs_view::terminal(job.phase)) {
        return app.show_error_toast("请先确认当前技能任务的最终状态");
    }
    let s = &mut app.skills_settings.install;
    if matches!(event, Event::Close) {
        // Keep a submitted operation visible until its terminal response.
        if !s.busy { *s = State { generation: s.generation.wrapping_add(1), ..Default::default() }; }
        return Task::none();
    }
    let Ok(client) = crate::app::config::gateway_client() else { return Task::none(); };
    let context = AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    if matches!(event, Event::Open) && !s.open {
        let Some(project) = app.project_id.clone() else { return app.show_error_toast("请先打开已登记的项目"); };
        *s = State { open: true, context: Some(context.clone()), catalog: app.apps_catalog.generation,
            scope: Some(SkillReposQuery { project_id: project.into(), target_host_id: app.apps_catalog.host.clone() }),
            generation: s.generation.wrapping_add(1), ..Default::default() };
    }
    if !s.open { return Task::none(); }
    if s.context.as_ref() != Some(&context) || s.catalog != app.apps_catalog.generation
        || s.scope.as_ref().is_none_or(|scope| Some(scope.project_id.as_ref()) != app.project_id.as_deref()
            || scope.target_host_id != app.apps_catalog.host) {
        *s = State { generation: s.generation.wrapping_add(1), ..Default::default() };
        return Task::none();
    }
    match event {
        Event::Submitted(g, result) if g == s.generation && s.busy => {
            s.busy = false;
            match result {
                Ok(response) if Some(&response.scope) == s.scope.as_ref() => {
                    s.inventory = None;
                    return super::jobs_view::submitted(app, response.scope, response.task_id);
                }
                Ok(_) => s.error = Some("安装响应目标不匹配".into()),
                Err(e) => { s.error = Some(e.clone()); return app.show_error_toast(e); }
            }
            return Task::none();
        }
        Event::Submitted(..) => return Task::none(),
        Event::Loaded(g, result) if g == s.generation && s.busy => {
            s.busy = false;
            match result {
                Ok(data) if Some(&data.scope) == s.scope.as_ref() => { s.inventory = Some(data); s.error = None; }
                Ok(_) => s.error = Some("技能响应目标不匹配".into()),
                Err(e) => { s.error = Some(e.clone()); return app.show_error_toast(e); }
            }
            return Task::none();
        }
        Event::Installed(g, result) if g == s.generation && s.busy => {
            s.busy = false;
            if let Err(e) = result { s.error = Some(e.clone()); return app.show_error_toast(e); }
            s.inventory = None;
            let project = app.project_path.clone();
            let refresh = update(app, Event::Refresh);
            return Task::batch(vec![refresh, super::refresh_task(project)]);
        }
        Event::Loaded(..) | Event::Installed(..) => return Task::none(),
        _ if s.busy => return Task::none(),
        Event::Select(id) => { s.selected = Some(id); return Task::none(); }
        Event::Repo(v) => { s.repo = v; return Task::none(); }
        Event::Selection(v) => { s.selection = v; return Task::none(); }
        Event::Directory(v) => { s.directory = v; return Task::none(); }
        _ => {}
    }
    if app.apps_catalog.context.as_ref() != Some(&context) || app.apps_catalog.token.is_empty() {
        s.error = Some("请先在应用设置中连接并授权目标主机".into()); return Task::none();
    }
    let scope = s.scope.clone().expect("open scope");
    let request = if matches!(event, Event::Install) {
        let Some(data) = &s.inventory else { return Task::none(); };
        if s.repo.trim().is_empty() || s.selection.trim().is_empty() || s.directory.trim().is_empty() {
            s.error = Some("请选择仓库、来源目录与安装目录".into()); return Task::none();
        }
        Some(InstallSkillRequest { scope: scope.clone(), expected_version: data.version,
            source: SkillInstallSource::Git { repo_id: s.repo.trim().into(),
                selection: s.selection.trim().into(), directory: s.directory.trim().into() } })
    } else { None };
    s.inventory = None;
    s.busy = true; s.error = None; s.generation = s.generation.wrapping_add(1);
    let g = s.generation;
    let token = app.apps_catalog.token.clone();
    if let Some(request) = request {
        return Task::perform(crate::app::session_gateway::gateway_skills_submit(request, token),
            move |r| message(Event::Submitted(g, r)));
    }
    Task::perform(async move {
        crate::app::session_gateway::gateway_skills_install_view(scope, None, token).await
    }, move |result| match result {
        Ok(Some(data)) => message(Event::Loaded(g, Ok(data))),
        Ok(None) => message(Event::Installed(g, Ok(()))),
        Err(e) => message(Event::Installed(g, Err(e))),
    })
}

#[cfg(test)]
#[path = "install_view_tests.rs"]
mod tests;

/// Only expose inventory from the active authenticated catalog generation.
pub(crate) fn sync_inventory(app: &App) -> Option<&SkillInventoryResponse> {
    let state = &app.skills_settings.install;
    (state.context.as_ref() == app.apps_catalog.context.as_ref()
        && state.catalog == app.apps_catalog.generation)
        .then_some(state.inventory.as_ref()).flatten()
}
