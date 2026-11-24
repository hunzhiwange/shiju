use std::collections::{HashMap, HashSet};

use gateway_interaction::{
    GatewayInteractionClient, PendingPermissionReplyDto, PendingPermissionRequestDto,
};
use iced::{
    Task,
    futures::{StreamExt, stream},
    widget::pane_grid,
    window,
};
use vw_api_acp_model::acp_model_source::{AcpModelSource, AcpModelSourceCapability};
use vw_api_project::{ListProjectsRequest, ProjectDto, ResolveProjectRequest};
use vw_api_types::cli_workbench::{
    CliWorkbenchError, CliWorkbenchErrorCode as Code, CliWorkbenchMessage,
    CliWorkbenchProcessState as Process, CliWorkbenchSession, CliWorkbenchTarget,
    SendCliWorkbenchMessage, StartCliWorkbenchSession,
};
use vw_gateway_client::GatewayClient;

use super::{App, Dialog, Message, Project, SessionKey, Turn, Workbench};

#[derive(Clone)]
pub(crate) struct Scope(gateway_endpoint::GatewayEndpoint);

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = (&self.0, &other.0);
        a.normalized_host() == b.normalized_host()
            && a.port == b.port
            && a.tenant_scope() == b.tenant_scope()
            && a.user_id() == b.user_id()
            && a.auth.as_ref().and_then(|auth| auth.skey.as_ref())
                == b.auth.as_ref().and_then(|auth| auth.skey.as_ref())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct Agent {
    pub(super) key: String,
    label: Option<String>,
    #[serde(default)]
    runtime: config_agent_profile::AgentRuntime,
}

impl Agent {
    pub(super) fn label(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.key)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectUpdate {
    project: String,
    sessions: Result<Vec<CliWorkbenchSession>, CliWorkbenchError>,
    approvals: HashMap<String, PendingPermissionRequestDto>,
    approval_error: bool,
    turns: Vec<(SessionKey, String, Result<CliWorkbenchMessage, CliWorkbenchError>)>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Open(String),
    Tick,
    Window(window::Id, Action),
    Response(window::Id, u64, Action),
}

#[derive(Clone)]
pub(crate) enum Action {
    Refresh,
    ProjectsLoaded(u64, Result<(Vec<ProjectDto>, Option<String>), ()>),
    SessionsLoaded(u64, Vec<ProjectUpdate>),
    SelectProject(String),
    SelectSession(SessionKey),
    SelectAgent(String),
    AgentsLoaded(u64, Result<Vec<Agent>, ()>),
    Start,
    Started(String, Result<CliWorkbenchSession, CliWorkbenchError>),
    Hide(SessionKey),
    Stop(SessionKey),
    Stopped(SessionKey, Result<CliWorkbenchSession, CliWorkbenchError>),
    Rename(SessionKey),
    Compose(Option<String>),
    Draft(String),
    Target(String),
    Confirm,
    Dismiss,
    Sent(SessionKey, String, Result<CliWorkbenchMessage, CliWorkbenchError>),
    Approve(SessionKey, String, PendingPermissionReplyDto),
    Approved(SessionKey, String, bool),
    Split(String, pane_grid::Axis),
    Focus(String, pane_grid::Pane),
    ClosePane(String, pane_grid::Pane),
    Resize(String, pane_grid::ResizeEvent),
    Drag(String, pane_grid::DragEvent),
}

// UI events may contain message drafts, project paths or approval details.
impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CLI workbench window action")
    }
}

pub(super) fn event(id: window::Id, action: Action) -> Message {
    Message::CliWorkbench(Event::Window(id, action))
}

fn response(id: window::Id, scope_generation: u64, action: Action) -> Message {
    Message::CliWorkbench(Event::Response(id, scope_generation, action))
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    if let Event::Open(backend) = event {
        let current_directory = app.navigation_tabs.project_path.clone();
        let (id, open) = app.open_cli_workbench_window();
        let state = &mut app.cli_workbench;
        state.backend = backend;
        state.preferred_directory = current_directory;
        state.selected = None;
        state.dialog = None;
        state.agents.clear();
        state.agent = None;
        state.agents_epoch += 1;
        state.agents_loading = false;
        state.error = None;
        return Task::batch([open, reload_projects(state, id)]);
    }

    let state = &mut app.cli_workbench;
    // Request receipts belong to the session. Finish a POST even when its window was closed.
    if let Event::Response(source, generation, Action::Sent(key, request, result)) = event {
        if generation == state.scope_generation
            && state.scope.as_ref() == Some(&Scope(super::super::config::gateway_client_endpoint()))
            && state.tab_mut(&key).is_some_and(|tab| {
                tab.turns
                    .iter()
                    .any(|turn| turn.request == request && turn.submitting == Some(source))
            })
        {
            state.epoch += 1;
            state.apply_turn(&key, &request, result, true);
        }
        return Task::none();
    }
    let Some(id) = state.window else { return Task::none() };
    if matches!(&event, Event::Response(_, generation, _) if *generation != state.scope_generation)
    {
        return Task::none();
    }
    match event {
        Event::Tick => {
            // Never apply an in-flight response from the previous gateway or account.
            if bind_scope(state)
                || (!state.discovering && state.error == Some("无法读取工作台项目"))
            {
                return reload_projects(state, id);
            }
            refresh_sessions(state, id)
        }
        Event::Window(source, action) | Event::Response(source, _, action) if source == id => {
            if !matches!(
                &action,
                Action::Draft(_)
                    | Action::Target(_)
                    | Action::Resize(..)
                    | Action::Drag(..)
                    | Action::Focus(..)
            ) && bind_scope(state)
            {
                return reload_projects(state, id);
            }
            update_window(state, id, action)
        }
        _ => Task::none(),
    }
}

fn bind_scope(state: &mut Workbench) -> bool {
    let scope = Scope(super::super::config::gateway_client_endpoint());
    if state.scope.as_ref() == Some(&scope) {
        return false;
    }
    state.scope = Some(scope);
    state.scope_generation += 1;
    state.projects.clear();
    state.selected = None;
    state.preferred_directory = None;
    state.agents.clear();
    state.agent = None;
    state.dialog = None;
    state.refreshing = false;
    state.discovering = false;
    state.starting = false;
    state.agents_loading = false;
    state.epoch += 1;
    state.agents_epoch += 1;
    true
}

fn client(state: &Workbench) -> Option<GatewayClient> {
    GatewayClient::new(state.scope.as_ref()?.0.clone()).ok()
}

fn reload_projects(state: &mut Workbench, id: window::Id) -> Task<Message> {
    let preferred = state.preferred_directory.clone();
    bind_scope(state);
    state.preferred_directory.clone_from(&preferred);
    state.discovering = true;
    state.projects_epoch += 1;
    let epoch = state.projects_epoch;
    let generation = state.scope_generation;
    state.error = None;
    let client = client(state);
    Task::perform(
        async move {
            let client = client.ok_or(())?;
            let projects = gateway_project::GatewayProjectClient::new(client.transport().clone());
            let scope = client.endpoint().tenant_scope();
            let mut query = ListProjectsRequest { limit: Some(100), ..Default::default() };
            let mut all = Vec::new();
            loop {
                let page = projects.project_list(scope, &query).await.map_err(|_| ())?;
                all.extend(page.items);
                let Some(next) = page.next_cursor else { break };
                if query.cursor.as_ref() == Some(&next) {
                    return Err(());
                }
                query.cursor = Some(next);
            }
            let mut selected = None;
            if let Some(directory) = preferred {
                if let Some(project) = all.iter().find(|project| project.directory == directory) {
                    selected = Some(project.id.as_ref().to_owned());
                } else if let Ok(resolved) = projects
                    .project_resolve(
                        scope,
                        &ResolveProjectRequest { directory, create_if_missing: false },
                    )
                    .await
                {
                    selected = Some(resolved.project.id.as_ref().to_owned());
                    if !all.iter().any(|project| project.id == resolved.project.id) {
                        all.push(resolved.project);
                    }
                }
            }
            Ok((all, selected))
        },
        move |result| response(id, generation, Action::ProjectsLoaded(epoch, result)),
    )
}

fn load_agents(state: &mut Workbench, id: window::Id) -> Task<Message> {
    state.agents_epoch += 1;
    let epoch = state.agents_epoch;
    let generation = state.scope_generation;
    state.agents.clear();
    state.agent = None;
    let Some(project) = state.selected.as_ref().and_then(|id| state.projects.get(id)) else {
        state.agents_loading = false;
        return Task::none();
    };
    state.agents_loading = true;
    let directory = project.directory.clone();
    let backend = state.backend.clone();
    let client = client(state);
    Task::perform(
        async move {
            let client = client.ok_or(())?;
            let capabilities =
                client.acp_model_source_capabilities(&backend).await.map_err(|_| ())?;
            if !capabilities
                .model_source_capabilities
                .contains(&AcpModelSourceCapability::Supported { source: AcpModelSource::CliLocal })
            {
                return Ok(Vec::new());
            }
            let config = gateway_config::GatewayConfigClient::new(client.transport().clone());
            let profiles = config.agent_profiles(Some(&directory), false).await.map_err(|_| ())?;
            let mut agents: Vec<Agent> = serde_json::from_value(profiles).map_err(|_| ())?;
            agents.retain(|agent| matches!(
                &agent.runtime,
                config_agent_profile::AgentRuntime::Acp { backend: candidate } if candidate == &backend
            ));
            agents.sort_by(|left, right| {
                left.label().cmp(right.label()).then(left.key.cmp(&right.key))
            });
            Ok(agents)
        },
        move |result| response(id, generation, Action::AgentsLoaded(epoch, result)),
    )
}

fn refresh_sessions(state: &mut Workbench, id: window::Id) -> Task<Message> {
    if state.refreshing || state.projects.is_empty() {
        return Task::none();
    }
    let Some(client) = client(state) else {
        state.error = Some("网关不可用");
        return Task::none();
    };
    state.refreshing = true;
    let epoch = state.epoch;
    let generation = state.scope_generation;
    let projects: Vec<_> = state
        .projects
        .iter()
        .map(|(project, state)| {
            let requests: Vec<_> = state
                .tabs
                .iter()
                .flat_map(|(session, tab)| {
                    tab.turns
                        .iter()
                        .filter(|turn| turn.recover && turn.submitting.is_none())
                        .map(move |turn| (session.clone(), turn.request.clone()))
                })
                .collect();
            (project.clone(), requests)
        })
        .collect();
    Task::perform(
        async move {
            stream::iter(projects)
                .map(|(project, requests)| project_update(client.clone(), project, requests))
                .buffer_unordered(6)
                .collect::<Vec<_>>()
                .await
        },
        move |updates| response(id, generation, Action::SessionsLoaded(epoch, updates)),
    )
}

async fn project_update(
    client: GatewayClient,
    project: String,
    requests: Vec<(String, String)>,
) -> ProjectUpdate {
    let sessions = client.cli_workbench_list(&project).await;
    let mut approvals = HashMap::new();
    let mut approval_error = false;
    let mut turns = Vec::new();
    if let Ok(sessions) = &sessions {
        let waiting: HashSet<_> = sessions
            .iter()
            .filter(|session| session.state == Process::WaitingForApproval)
            .map(|session| session.directory.as_str())
            .collect();
        let interaction = GatewayInteractionClient::new(client.transport().clone());
        let mut responses = stream::iter(waiting)
            .map(|directory| interaction.permission_list(Some(directory)))
            .buffer_unordered(4);
        while let Some(result) = responses.next().await {
            match result {
                Ok(requests) => {
                    for request in requests {
                        if sessions.iter().any(|session| {
                            session.state == Process::WaitingForApproval
                                && session.id == request.session_id
                        }) {
                            approvals.entry(request.session_id.clone()).or_insert(request);
                        }
                    }
                }
                Err(_) => approval_error = true,
            }
        }
        let project_id = &project;
        let gateway = &client;
        turns = stream::iter(requests)
            .map(|(session, request)| async move {
                let result = gateway.cli_workbench_receive(project_id, &session, &request).await;
                (SessionKey { project: project_id.clone(), session }, request, result)
            })
            .buffer_unordered(4)
            .collect()
            .await;
    }
    ProjectUpdate { project, sessions, approvals, approval_error, turns }
}

fn apply_projects(state: &mut Workbench, projects: Vec<ProjectDto>, selected: Option<String>) {
    let known: HashSet<_> = projects.iter().map(|project| project.id.as_ref()).collect();
    state.projects.retain(|id, _| known.contains(id.as_str()));
    for project in &projects {
        let entry = state
            .projects
            .entry(project.id.as_ref().to_owned())
            .or_insert_with(|| Project::new(project.name.clone(), project.directory.clone()));
        entry.name.clone_from(&project.name);
        entry.directory.clone_from(&project.directory);
    }
    state.selected =
        selected.or_else(|| state.selected.take().filter(|id| state.projects.contains_key(id)));
}

fn apply_projects_updates(state: &mut Workbench, batch: Vec<ProjectUpdate>) {
    for update in batch {
        let Some(project) = state.projects.get_mut(&update.project) else { continue };
        match update.sessions {
            Ok(sessions) => {
                project.merge_sessions(sessions);
                project.disconnected = false;
                for (id, tab) in &mut project.tabs {
                    tab.approval = update.approvals.get(id).cloned();
                }
                project.approval_error = update.approval_error;
            }
            Err(_) => {
                project.disconnected = true;
                continue;
            }
        }
        for (key, request, result) in update.turns {
            state.apply_turn(&key, &request, result, false);
        }
    }
}

fn update_window(state: &mut Workbench, id: window::Id, action: Action) -> Task<Message> {
    match action {
        Action::Refresh if !state.discovering => return reload_projects(state, id),
        Action::ProjectsLoaded(epoch, result) if epoch == state.projects_epoch => {
            state.discovering = false;
            match result {
                Ok((projects, selected)) => {
                    state.preferred_directory = None;
                    apply_projects(state, projects, selected);
                    return Task::batch([load_agents(state, id), refresh_sessions(state, id)]);
                }
                Err(()) => state.error = Some("无法读取工作台项目"),
            }
        }
        Action::SessionsLoaded(epoch, updates) => {
            state.refreshing = false;
            if epoch == state.epoch {
                apply_projects_updates(state, updates);
            }
        }
        Action::SelectProject(project) if state.projects.contains_key(&project) => {
            if state.selected.as_ref() != Some(&project) {
                state.selected = Some(project);
                state.preferred_directory = None;
                state.dialog = None;
                return load_agents(state, id);
            }
        }
        Action::SelectSession(key) => {
            let changed = state.selected.as_ref() != Some(&key.project);
            if state.locate(&key) && changed {
                state.dialog = None;
                return load_agents(state, id);
            }
        }
        Action::AgentsLoaded(epoch, result) if epoch == state.agents_epoch => {
            state.agents_loading = false;
            match result {
                Ok(agents) => {
                    state.agent = agents.first().map(|agent| agent.key.clone());
                    state.agents = agents;
                }
                Err(()) => state.error = Some("无法读取可用的 ACP 智能体"),
            }
        }
        Action::SelectAgent(key) if state.agents.iter().any(|agent| agent.key == key) => {
            state.agent = Some(key);
        }
        Action::Start => return start(state, id),
        Action::Started(project_id, result) => {
            state.starting = false;
            state.epoch += 1;
            match result {
                Ok(session) if session.project_id == project_id => {
                    if let Some(project) = state.projects.get_mut(&project_id) {
                        let session_id = session.id.clone();
                        project
                            .tabs
                            .entry(session_id.clone())
                            .or_insert_with(|| super::Tab::new(session));
                        if state.selected.as_ref() == Some(&project_id) {
                            project.select(&session_id);
                        }
                    }
                }
                Ok(_) => state.error = Some("网关返回无效的工作台会话"),
                Err(error) => state.error = Some(error_label(error.code)),
            }
        }
        Action::Hide(key) => {
            if let Some(project) = state.projects.get_mut(&key.project) {
                project.hide(&key.session);
            }
        }
        Action::Stop(key) => return stop(state, id, key),
        Action::Stopped(key, result) => {
            state.epoch += 1;
            if let Some(tab) = state.tab_mut(&key) {
                tab.finish_stop(result);
            }
        }
        Action::Rename(key) => {
            if let Some(tab) = state.tab_mut(&key) {
                let name = tab.name.clone().unwrap_or_else(|| super::view::tab_label(tab));
                state.dialog = Some(Dialog::Rename(key, name));
                return iced::widget::operation::focus(super::view::DIALOG_INPUT);
            }
        }
        Action::Compose(target) => {
            state.dialog = Some(Dialog::Compose { target, draft: String::new() });
            return iced::widget::operation::focus(super::view::DIALOG_INPUT);
        }
        Action::Draft(value) => match state.dialog.as_mut() {
            Some(Dialog::Compose { draft, .. } | Dialog::Rename(_, draft)) => *draft = value,
            None => {}
        },
        Action::Target(value) => {
            if let Some(Dialog::Compose { target, .. }) = state.dialog.as_mut() {
                *target = Some(value);
            }
        }
        Action::Dismiss => state.dialog = None,
        Action::Confirm => return confirm(state, id),
        Action::Approve(key, request, reply) => return approve(state, id, key, request, reply),
        Action::Approved(key, request, applied) => {
            state.epoch += 1;
            if let Some(tab) = state.tab_mut(&key)
                && tab.approving.as_deref() == Some(&request)
            {
                tab.approving = None;
                if applied && tab.approval.as_ref().is_some_and(|approval| approval.id == request) {
                    tab.approval = None;
                }
                if !applied {
                    tab.action_error = Some("提交审批失败，请重试");
                }
            }
        }
        Action::Split(project, axis) => {
            if let Some(project) = state.projects.get_mut(&project) {
                project.split(axis);
            }
        }
        Action::Focus(project, pane) => {
            if let Some(project) = state.projects.get_mut(&project)
                && project.panes.get(pane).is_some()
            {
                project.active_pane = pane;
            }
        }
        Action::ClosePane(project, pane) => {
            if let Some(project) = state.projects.get_mut(&project) {
                project.close_pane(pane);
            }
        }
        Action::Resize(project, resize) => {
            if let Some(project) = state.projects.get_mut(&project) {
                project.panes.resize(resize.split, resize.ratio.clamp(0.2, 0.8));
            }
        }
        Action::Drag(project, pane_grid::DragEvent::Dropped { pane, target }) => {
            if let Some(project) = state.projects.get_mut(&project) {
                project.panes.drop(pane, target);
            }
        }
        _ => {}
    }
    Task::none()
}

fn start(state: &mut Workbench, id: window::Id) -> Task<Message> {
    let (Some(project), Some(agent), Some(client)) =
        (state.selected.clone(), state.agent.clone(), client(state))
    else {
        return Task::none();
    };
    if state.starting || state.projects.get(&project).is_none_or(|project| project.disconnected) {
        return Task::none();
    }
    state.starting = true;
    let generation = state.scope_generation;
    state.error = None;
    let request =
        StartCliWorkbenchSession { target: CliWorkbenchTarget::Acp { agent }, directory: None };
    Task::perform(
        async move {
            let result = client.cli_workbench_start(&project, &request).await;
            (project, result)
        },
        move |(project, result)| response(id, generation, Action::Started(project, result)),
    )
}

fn stop(state: &mut Workbench, id: window::Id, key: SessionKey) -> Task<Message> {
    let Some(client) = client(state) else { return Task::none() };
    let generation = state.scope_generation;
    if state.projects.get(&key.project).is_none_or(|project| project.disconnected) {
        return Task::none();
    }
    let Some(tab) = state.tab_mut(&key) else { return Task::none() };
    if !tab.begin_stop() {
        return Task::none();
    }
    state.error = None;
    Task::perform(
        async move {
            let result = client.cli_workbench_stop(&key.project, &key.session).await;
            (key, result)
        },
        move |(key, result)| response(id, generation, Action::Stopped(key, result)),
    )
}

fn confirm(state: &mut Workbench, id: window::Id) -> Task<Message> {
    match state.dialog.as_ref() {
        Some(Dialog::Rename(_, draft)) if !draft.trim().is_empty() => {
            if let Some(Dialog::Rename(key, draft)) = state.dialog.take()
                && let Some(tab) = state.tab_mut(&key)
            {
                tab.name = Some(draft.trim().chars().take(80).collect());
            }
        }
        Some(Dialog::Compose { target: Some(session), draft }) if !draft.trim().is_empty() => {
            let (Some(project), Some(client)) = (state.selected.clone(), client(state)) else {
                return Task::none();
            };
            let key = SessionKey { project, session: session.clone() };
            if state.projects.get(&key.project).is_none_or(|project| project.disconnected) {
                return Task::none();
            }
            let prompt = draft.clone();
            let generation = state.scope_generation;
            let Ok(request) = ordered_id::ascending(ordered_id::Prefix::Message, None) else {
                state.error = Some("无法生成工作台请求编号");
                return Task::none();
            };
            let Some(tab) = state.tab_mut(&key).filter(|tab| tab.can_send()) else {
                return Task::none();
            };
            tab.action_error = None;
            tab.turns.push(Turn {
                request: request.clone(),
                prompt: prompt.clone(),
                snapshot: None,
                error: None,
                recover: true,
                submitting: Some(id),
            });
            state.dialog = None;
            state.epoch += 1;
            // Polling must wait for the POST result before an absent receipt can end recovery.
            return Task::perform(
                async move {
                    let payload =
                        SendCliWorkbenchMessage { request_id: request.clone(), text: prompt };
                    let result =
                        client.cli_workbench_send(&key.project, &key.session, &payload).await;
                    (key, request, result)
                },
                move |(key, request, result)| {
                    response(id, generation, Action::Sent(key, request, result))
                },
            );
        }
        _ => {}
    }
    Task::none()
}

fn approve(
    state: &mut Workbench,
    id: window::Id,
    key: SessionKey,
    request: String,
    reply: PendingPermissionReplyDto,
) -> Task<Message> {
    let Some(client) = client(state) else { return Task::none() };
    let generation = state.scope_generation;
    if state.projects.get(&key.project).is_none_or(|project| project.disconnected)
        || matches!(reply, PendingPermissionReplyDto::Always)
    {
        return Task::none();
    }
    let Some(tab) = state.tab_mut(&key) else { return Task::none() };
    if tab.approving.is_some()
        || tab.approval.as_ref().is_none_or(|approval| approval.id != request)
    {
        return Task::none();
    }
    tab.approving = Some(request.clone());
    tab.action_error = None;
    let directory = tab.session.directory.clone();
    Task::perform(
        async move {
            let interaction = GatewayInteractionClient::new(client.transport().clone());
            let applied = interaction
                .permission_reply(&request, reply, Some(&directory), None)
                .await
                .unwrap_or(false);
            (key, request, applied)
        },
        move |(key, request, applied)| {
            response(id, generation, Action::Approved(key, request, applied))
        },
    )
}

pub(super) fn error_label(code: Code) -> &'static str {
    match code {
        Code::InvalidRequest => "工作台请求无效",
        Code::NotFound | Code::Closed => "工作台会话或请求已结束",
        Code::RequestConflict => "工作台请求编号冲突",
        Code::AwaitingApproval => "目标会话正在等待审批",
        Code::Busy => "目标会话忙，请稍后重试",
        Code::Unavailable => "目标暂不可用",
        Code::Disconnected => "工作台与网关断开连接",
        Code::Cancelled => "工作台请求已取消",
        Code::TimedOut => "工作台请求超时",
        Code::TurnFailed => "ACP 请求失败",
        Code::Unauthorized => "网关拒绝了工作台请求",
        Code::GatewayFailure => "工作台网关请求失败",
    }
}
