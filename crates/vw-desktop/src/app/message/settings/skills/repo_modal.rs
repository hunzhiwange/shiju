//! Versioned repository editing; no optimistic list updates or discovery orchestration.
use crate::app::{App, Message, message::SettingsMessage};
use crate::app::session_gateway::{gateway_skill_repos_request, gateway_skill_repos_scope};
use iced::Task;
use vw_gateway_client::vw_api_types::agent_apps_skills::repos_api::*;

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub open: bool,
    pub busy: bool,
    pub identity: String,
    pub source_link: String,
    pub source_preview: Option<super::deeplink::Request>,
    pub branch: String,
    pub snapshot: Option<SkillReposResponse>,
    pub error: Option<String>,
    generation: u64,
    directory: Option<String>,
    context: Option<crate::app::state::AgentAppsContext>,
    project: Option<String>,
    host: String,
}
#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open, Close, Identity(String), Branch(String), Refresh, Add,
    Toggle(usize), Remove(usize),
    SourceLink(String), PreviewSource, CancelSource, ConfirmSource,
    Completed { generation: u64, added: bool, result: Result<SkillReposResponse, String> },
}
fn message(event: Event) -> Message {
    Message::Settings(SettingsMessage::SkillRepositories(event))
}
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let Ok(client) = crate::app::config::gateway_client() else {
        return app.show_error_toast("无法连接仓库服务");
    };
    let context = crate::app::state::AgentAppsContext { gateway: client.endpoint().base_url(),
        tenant: client.endpoint().tenant_scope().clone(), project: app.project_path.clone() };
    let state = &mut app.skills_settings.repositories;
    if matches!(event, Event::Open) {
        if state.open { return Task::none(); }
        *state = State { open: true, generation: state.generation.wrapping_add(1),
            context: Some(context.clone()), project: app.project_id.clone(),
            directory: app.project_path.clone(), host: app.apps_catalog.host.clone(),
            ..Default::default() };
    } else if matches!(event, Event::Close) {
        // Keep writes visible until settled; closing a read invalidates its response.
        if !state.busy {
            state.open = false;
            state.generation = state.generation.wrapping_add(1);
        }
        return Task::none();
    }
    if !state.open { return Task::none(); }
    if state.context.as_ref() != Some(&context) || state.project != app.project_id
        || state.directory != app.project_path || state.host != app.apps_catalog.host {
        state.open = false;
        state.generation = state.generation.wrapping_add(1);
        return Task::none();
    }
    if let Event::Completed { generation, added, result } = event {
        if generation != state.generation || !state.busy { return Task::none(); }
        state.busy = false;
        match result {
            Ok(response) => {
                if Some(response.scope.project_id.as_ref()) != state.project.as_deref()
                    || response.scope.target_host_id != state.host {
                    state.error = Some("仓库响应目标不匹配，请刷新".into());
                    return Task::none();
                }
                if state.snapshot.as_ref().is_some_and(|old|
                    old.scope != response.scope || old.version > response.version) {
                    state.error = Some("仓库响应已过期，请刷新后重试".into());
                    return Task::none();
                }
                state.snapshot = Some(response);
                state.error = None;
                if added { state.identity.clear(); state.branch.clear();
                    state.source_link.clear(); state.source_preview = None; }
            }
            Err(error) => {
                state.error = Some(error.clone());
                return app.show_toast(crate::app::state::ToastKind::Error, error);
            }
        }
        return Task::none();
    }
    if state.busy { return Task::none(); }
    match event {
        Event::SourceLink(value) => {
            state.source_link = value; state.source_preview = None; state.error = None;
            return Task::none();
        }
        Event::PreviewSource => {
            state.source_preview = None;
            let Some(directory) = state.directory.clone() else {
                state.error = Some("请先打开项目".into()); return Task::none();
            };
            match super::deeplink::parse(&state.source_link, directory, state.host.clone()) {
                Ok(request) => { state.source_preview = Some(request); state.error = None; }
                Err(error) => { state.error = Some(error); }
            }
            return Task::none();
        }
        Event::CancelSource => { state.source_preview = None; return Task::none(); }
        Event::Identity(value) => { state.identity = value; return Task::none(); }
        Event::Branch(value) => { state.branch = value; return Task::none(); }
        _ => {}
    }
    let added = matches!(event, Event::Add | Event::ConfirmSource);
    let change = match event {
        Event::ConfirmSource => {
            let Some(request) = &state.source_preview else { return Task::none(); };
            if Some(&request.directory) != state.directory.as_ref() || request.host != state.host {
                state.source_preview = None; return Task::none();
            }
            Some((request.repo.clone(), false))
        }
        Event::Add => {
            let Some((owner, name)) = state.identity.trim().split_once('/') else {
                state.error = Some("请输入 owner/name 格式的仓库".into());
                return Task::none();
            };
            if owner.is_empty() || name.is_empty() || !owner.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
                state.error = Some("仓库名称无效".into()); return Task::none();
            }
            Some((SkillRepoEntry { owner: owner.into(), name: name.into(),
                branch: state.branch.trim().to_owned(), enabled: true }, false))
        }
        Event::Toggle(index) | Event::Remove(index) => {
            let Some(mut repo) = state.snapshot.as_ref().and_then(|s| s.repos.get(index)).cloned() else {
                return Task::none();
            };
            let remove = matches!(event, Event::Remove(_));
            if !remove { repo.enabled = !repo.enabled; }
            Some((repo, remove))
        }
        Event::Open | Event::Refresh => None,
        _ => return Task::none(),
    };
    if change.is_some() && state.snapshot.is_none() { return Task::none(); }
    let Some(directory) = state.directory.clone() else {
        state.error = Some("请先打开项目".into()); return Task::none();
    };
    let host = state.host.clone();
    let snapshot = state.snapshot.clone();
    state.busy = true;
    state.error = None;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    Task::perform(async move {
        let scope = match snapshot.as_ref() {
            Some(s) => s.scope.clone(),
            None => gateway_skill_repos_scope(&directory, host).await?,
        };
        gateway_skill_repos_request(scope, snapshot.map_or(0, |s| s.version), change).await
    }, move |result| message(Event::Completed { generation, added, result }))
}

#[cfg(test)]
#[path = "repo_modal_tests.rs"]
mod tests;

/// A repository selection must belong to the active gateway, project and host.
pub(crate) fn sync_inventory(app: &App) -> Option<&SkillReposResponse> {
    let state = &app.skills_settings.repositories;
    (state.context.as_ref() == app.apps_catalog.context.as_ref()
        && state.project.as_deref() == app.project_id.as_deref()
        && state.host == app.apps_catalog.host)
        .then_some(state.snapshot.as_ref()).flatten()
}
