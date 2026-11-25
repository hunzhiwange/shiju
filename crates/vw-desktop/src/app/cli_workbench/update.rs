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
use vw_api_types::cli_workbench::{
    CliWorkbenchProjects, CliWorkbenchDraft, CliWorkbenchDraftAction, CliWorkbenchError, CliWorkbenchErrorCode as Code,
    CliWorkbenchMessage, CliWorkbenchProcessState as Process, CliWorkbenchSession,
    CliWorkbenchTarget, CliWorkbenchTerminalCapability, CliWorkbenchTerminalOutput, SendCliWorkbenchMessage,
    StartCliWorkbenchSession,
};
use vw_gateway_client::GatewayClient;
use vw_api_project::{ListProjectsRequest, ResolveProjectRequest};

use super::{
    ssh, APP_TERMINAL_SIDEBAR_COLLAPSED, App, Dialog, Message, Project, SessionKey, Surface, Turn,
    Workbench,
};

pub(super) const TAB_ID: &str = "cli_workbench";

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

    pub(super) fn matching_backend(&self, filter: Option<&str>) -> Option<&str> {
        match &self.runtime {
            config_agent_profile::AgentRuntime::Acp { backend }
                if filter.is_none_or(|filter| filter == backend.as_str()) =>
            {
                Some(backend.as_str())
            }
            _ => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ProjectUpdate {
    project: String,
    approvals: HashMap<String, PendingPermissionRequestDto>,
    approval_error: bool,
    drafts: Option<Vec<CliWorkbenchDraft>>,
    turns: Vec<(SessionKey, String, Result<CliWorkbenchMessage, CliWorkbenchError>)>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    AppTerminal(Box<Event>),
    Terminal(Box<Event>),
    Open { backend: Option<String>, native_app: Option<String> },
    OpenSsh,
    Tick,
    TerminalTick,
    Window(window::Id, Action),
    Response(window::Id, u64, Action),
}

#[derive(Clone)]
pub(crate) enum Action {
    Ssh(ssh::Action),
    ShowTerminalGroup,
    Refresh,
    ProjectsLoaded(u64, u64, Result<(CliWorkbenchProjects, bool), Code>),
    NativeAppsLoaded(u64, CliWorkbenchTerminalCapability),
    SessionsLoaded(u64, u64, Result<CliWorkbenchProjects, ()>),
    LegacySessionsLoaded(u64, u64, Vec<(String, Result<Vec<CliWorkbenchSession>, CliWorkbenchError>)>),
    SessionDetailsLoaded(u64, Vec<ProjectUpdate>),
    SelectProject(String),
    SelectSession(SessionKey),
    AgentsLoaded(u64, Result<Vec<Agent>, ()>),
    Start(String, u64, CliWorkbenchTarget, pane_grid::Pane),
    Started(String, u64, CliWorkbenchTarget, pane_grid::Pane, Option<pane_grid::Direction>, Result<CliWorkbenchSession, CliWorkbenchError>),
    Hide(SessionKey),
    Stop(SessionKey),
    Stopped(SessionKey, Result<CliWorkbenchSession, CliWorkbenchError>),
    Rename(SessionKey),
    Compose(Option<String>),
    Draft(String),
    Target(String),
    Confirm,
    Dismiss,
    ShowDraft(SessionKey, String),
    CopyDraft(SessionKey, String),
    PasteDraft(SessionKey, String),
    ResolveDraft(SessionKey, String, CliWorkbenchDraftAction),
    DraftResolved(SessionKey, String, Result<CliWorkbenchMessage, CliWorkbenchError>),
    TerminalEvent(SessionKey, iced_term::Event),
    TerminalOutput(SessionKey, u64, Result<CliWorkbenchTerminalOutput, CliWorkbenchError>),
    TerminalWritten(SessionKey, Result<(), CliWorkbenchError>),
    Sent(SessionKey, String, Result<CliWorkbenchMessage, CliWorkbenchError>),
    Approve(SessionKey, String, PendingPermissionReplyDto),
    Approved(SessionKey, String, bool),
    Panel(String, u64, desktop_terminal::message::Message),
    GroupAgentsByProject(bool),
    ToggleProjectsSidebar,
    ResizeSidebarWidth(iced::widget::pane_grid::ResizeEvent),
    ResizeSidebarSections(iced::widget::pane_grid::ResizeEvent),
    NewLayout(String),
    SelectLayout(String, u64),
    CloseLayout(String, u64),
    OpenLayoutMenu(String, u64, iced::Point),
    CloseLayoutMenu(String),
    RenameLayout(String, u64, String),
    TerminalClipboard(SessionKey, u64, Option<String>),
}

// UI events may contain message drafts, project paths or approval details.
impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CLI workbench window action")
    }
}

pub(super) fn response(id: window::Id, scope_generation: u64, action: Action) -> Message {
    Message::CliWorkbench(Event::Response(id, scope_generation, action))
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::AppTerminal(event) => update_surface(app, Surface::AppTerminal, *event)
            .map(|message| Surface::AppTerminal.route_message(message)),
        Event::Terminal(event) => update_surface(app, Surface::Terminal, *event)
            .map(|message| Surface::Terminal.route_message(message)),
        event => update_surface(app, Surface::Workbench, event),
    }
}

fn update_surface(app: &mut App, surface: Surface, mut event: Event) -> Task<Message> {
    use crate::app::Screen;

    let navigation_target = match surface {
        Surface::Workbench => Some((Screen::CliWorkbench, TAB_ID, "我的工作台")),
        Surface::AppTerminal => Some((Screen::AppTerminal, "app_terminal", "应用终端")),
        Surface::Terminal => None,
    };
    let embedded = surface == Surface::Terminal;
    let visible = navigation_target.map_or_else(
        || super::terminal_panel_visible(app),
        |(screen, _, _)| app.navigation_tabs.screen == screen,
    );
    let id = match (app.main_window_id, &event) {
        (Some(id), _) => id,
        (None, Event::Response(id, ..)) => *id,
        (None, _) => return Task::none(),
    };
    if let Some((screen, tab_id, title)) = navigation_target
        && matches!(&event, Event::Open { .. })
    {
        let navigation = &mut app.navigation_tabs;
        if !navigation.open_tabs.iter().any(|tab| tab.id == tab_id) {
            navigation.open_tabs.push(desktop_navigation::components::top_bar::types::AppTab {
                id: tab_id.into(),
                title: title.into(),
                screen,
                project_path: None,
            });
        }
        navigation.active_tab_id = Some(tab_id.into());
        navigation.screen = screen;
        app.navigation_panels.show_system_settings = false;
    }
    let open_ssh = matches!(&event, Event::OpenSsh);
    let terminal_directory = if embedded {
        app.terminal_project_path().map(str::to_owned)
    } else {
        None
    };
    let terminal_directory = terminal_directory.as_deref();
    let terminal_tick = matches!(&event, Event::TerminalTick);
    let terminal_visible = visible
        && (!embedded || app.terminal_ssh.terminal_group_selected(terminal_directory));
    let theme = (terminal_tick && terminal_visible).then(|| app.theme());
    if let Event::Response(_, _,
        Action::ProjectsLoaded(_, _, Ok((snapshot, _))) | Action::SessionsLoaded(_, _, Ok(snapshot)),
    ) = &mut event
        && let Some(removed) = app.workbench_projects.active_removed()
    {
        snapshot.projects.retain(|project| !removed.contains(&project.directory));
    }
    let state = match surface {
        Surface::Workbench => &mut app.cli_workbench,
        Surface::AppTerminal => &mut app.app_terminal,
        Surface::Terminal => &mut app.terminal_ssh,
    };
    if embedded {
        if visible && matches!(&event, Event::Tick | Event::OpenSsh) {
            let scope_changed = bind_scope(state);
            let project_changed = state.terminal_directory.as_deref() != terminal_directory;
            if project_changed {
                state.terminal_directory = terminal_directory.map(str::to_owned);
                state.terminal_group_selected = false;
                state.selected = None;
                state.dialog = None;
            }
            if open_ssh {
                ssh::show(state);
            }
            if scope_changed || project_changed || open_ssh {
                return reload_projects(state, id);
            }
        }
        if matches!(&event, Event::Open { .. } | Event::OpenSsh)
            || (matches!(&event, Event::Window(..))
                && state.terminal_directory.as_deref() != terminal_directory)
        {
            return Task::none();
        }
    }
    if let Event::Open { backend, native_app } = event {
        state.backend = backend;
        state.native_app = native_app;
        bind_scope(state);
        state.default_shell_pending = true;
        state.preferred_directory = if state.selected.is_none() {
            app.navigation_tabs.project_path.clone()
        } else {
            None
        };
        state.dialog = None;
        state.agents.clear();
        state.agents_epoch += 1;
        state.agents_loading = false;
        state.error = None;
        return reload_projects(state, id);
    }

    if terminal_tick {
        return theme.map_or_else(Task::none, |theme| super::terminal::tick(state, id, &theme));
    }

    // Request receipts belong to the session even when the workbench tab is hidden.
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
    if matches!(&event, Event::Response(_, generation, _) if *generation != state.scope_generation)
    {
        return Task::none();
    }
    match event {
        Event::Tick if visible => {
            // Never apply an in-flight response from the previous gateway or account.
            if bind_scope(state)
                || (!state.discovering && matches!(state.error, Some("无法读取工作台项目" | "工作台与网关断开连接")))
            {
                return reload_projects(state, id);
            }
            state.checkpoint_layout();
            refresh_sessions(state, id)
        }
        Event::Window(_, _) if !visible => Task::none(),
        Event::Window(source, _) if source != id => Task::none(),
        Event::Window(_, Action::ToggleProjectsSidebar) if surface == Surface::AppTerminal => {
            let collapsed = !state.sidebar_layout.collapsed();
            state.sidebar_layout.set_collapsed(collapsed);
            app.save_desktop_preference(APP_TERMINAL_SIDEBAR_COLLAPSED, collapsed.into())
        }
        Event::Window(_, action) | Event::Response(_, _, action) => {
            if !matches!(
                &action,
                Action::Draft(_)
                    | Action::ResizeSidebarWidth(_)
                    | Action::ResizeSidebarSections(_)
                    | Action::Target(_)
                    | Action::TerminalOutput(..)
                    | Action::TerminalWritten(..)
                    | Action::TerminalEvent(..)
            ) && bind_scope(state)
            {
                return reload_projects(state, id);
            }
            if !terminal_visible
                && matches!(&action, Action::TerminalEvent(..) | Action::Panel(..) | Action::Start(..))
            {
                return Task::none();
            }
            let visible = match &action {
                Action::TerminalOutput(..) | Action::TerminalWritten(..) | Action::TerminalClipboard(..) => terminal_visible,
                _ => visible && (!embedded || state.terminal_directory.as_deref() == terminal_directory),
            };
            let show_start_error = embedded && visible && matches!(&action, Action::Started(..));
            let dismiss = matches!(&action, Action::Dismiss);
            let task = update_window(state, id, action, visible);
            if show_start_error && let Some(error) = state.error {
                return Task::batch([
                    task,
                    app.show_localized_toast(crate::app::state::ToastKind::Error, error, vec![]),
                ]);
            }
            if embedded && visible
                && (dismiss || terminal_visible)
                && !state.terminal_group_selected(terminal_directory)
            {
                return Task::batch([
                    task,
                    crate::app::message::terminal::event::focus_active_terminal(app),
                ]);
            }
            task
        }
        _ => Task::none(),
    }
}

pub(super) fn current_scope(state: &Workbench) -> bool {
    state.scope.as_ref() == Some(&Scope(super::super::config::gateway_client_endpoint()))
}

fn bind_scope(state: &mut Workbench) -> bool {
    let scope = Scope(super::super::config::gateway_client_endpoint());
    if state.scope.as_ref() == Some(&scope) && state.client.is_some() {
        return false;
    }
    state.checkpoint_layout();
    state.workspace = super::workspace::Store::open(&scope.0, state.surface);
    state.client = GatewayClient::new(scope.0.clone()).ok();
    state.scope = Some(scope);
    state.terminal_available = false;
    state.terminal_group_selected = false;
    state.ssh = ssh::State::default();
    state.native_apps.clear();
    state.scope_generation += 1;
    state.projects.clear();
    state.legacy_projects = false;
    state.legacy_poll_cursor = 0;
    state.selected = None;
    state.preferred_directory = None;
    state.agents.clear();
    state.dialog = None;
    state.refreshing = false;
    state.details_refreshing = false;
    state.discovering = false;
    state.starting = false;
    state.default_shell_pending = false;
    state.agents_loading = false;
    state.epoch += 1;
    state.agents_epoch += 1;
    true
}

pub(super) fn client(state: &Workbench) -> Option<GatewayClient> {
    state.client.clone()
}

fn reload_projects(state: &mut Workbench, id: window::Id) -> Task<Message> {
    let preferred = if state.surface == Surface::Terminal {
        state.terminal_directory.clone()
    } else {
        state.preferred_directory.clone()
    };
    bind_scope(state);
    state.preferred_directory.clone_from(&preferred);
    state.discovering = true;
    state.projects_epoch += 1;
    let epoch = state.projects_epoch;
    let session_epoch = state.epoch;
    let generation = state.scope_generation;
    state.error = None;
    let client = client(state);
    let catalog_client = client.clone().filter(|client| {
        state.surface != Surface::Terminal && client.endpoint().is_loopback()
    });
    let catalog = catalog_client.map_or_else(Task::none, |client| {
        Task::perform(
            async move { client.cli_workbench_terminal_catalog().await.unwrap_or_default() },
            move |catalog| response(id, generation, Action::NativeAppsLoaded(epoch, catalog)),
        )
    });
    let projects = Task::perform(
        async move {
            let client = client.ok_or(Code::Disconnected)?;
            match client.cli_workbench_projects(preferred.as_deref()).await {
                Ok(snapshot) => Ok((snapshot, false)),
                Err(error) if error.code == Code::NotFound => {
                    legacy_projects(&client, preferred).await.map(|snapshot| (snapshot, true))
                }
                Err(error) => Err(error.code),
            }
        },
        move |result| response(id, generation, Action::ProjectsLoaded(epoch, session_epoch, result)),
    );
    let ssh = ssh::load(state, id);
    Task::batch([catalog, projects, ssh])
}

async fn legacy_projects(
    client: &GatewayClient,
    preferred: Option<String>,
) -> Result<CliWorkbenchProjects, Code> {
    let project_client = gateway_project::GatewayProjectClient::new(client.transport().clone());
    let scope = client.endpoint().tenant_scope();
    let mut query = ListProjectsRequest { limit: Some(200), ..Default::default() };
    let mut projects = Vec::new();
    let mut seen = HashSet::new();
    loop {
        let page = project_client.project_list(scope, &query).await.map_err(|_| Code::GatewayFailure)?;
        projects.extend(page.items.into_iter().map(|project| vw_api_types::cli_workbench::CliWorkbenchProject {
            id: project.id.as_ref().to_owned(), name: project.name, directory: project.directory,
            sessions: Vec::new(),
        }));
        let Some(next) = page.next_cursor else { break };
        if !seen.insert(next.clone()) {
            return Err(Code::GatewayFailure);
        }
        query.cursor = Some(next);
    }
    let mut preferred_project = None;
    if let Some(directory) = preferred {
        preferred_project = projects.iter().find(|project| project.directory == directory)
            .map(|project| project.id.clone());
        if preferred_project.is_none()
            && let Ok(result) = project_client.project_resolve(
                scope, &ResolveProjectRequest { directory, create_if_missing: false },
            ).await
        {
            let id = result.project.id.as_ref().to_owned();
            if !projects.iter().any(|project| project.id == id) {
                projects.push(vw_api_types::cli_workbench::CliWorkbenchProject {
                    id: id.clone(), name: result.project.name, directory: result.project.directory,
                    sessions: Vec::new(),
                });
            }
            preferred_project = Some(id);
        }
    }
    let terminal_available = client.endpoint().is_loopback()
        && client.cli_workbench_terminal_capability().await.is_ok_and(|value| value.available);
    Ok(CliWorkbenchProjects { projects, preferred_project, terminal_available })
}

fn load_agents(state: &mut Workbench, id: window::Id) -> Task<Message> {
    if state.surface != Surface::Workbench {
        return Task::none();
    }
    state.agents_epoch += 1;
    let epoch = state.agents_epoch;
    let generation = state.scope_generation;
    state.agents.clear();
    let Some(project) = state.selected.as_ref().and_then(|id| state.projects.get(id)) else {
        state.agents_loading = false;
        return Task::none();
    };
    state.agents_loading = true;
    let directory = project.directory.clone();
    let backend_filter = state.backend.clone();
    let client = client(state);
    Task::perform(
        async move {
            let client = client.ok_or(())?;
            let config = gateway_config::GatewayConfigClient::new(client.transport().clone());
            let profiles = config.agent_profiles(Some(&directory), false).await.map_err(|_| ())?;
            let mut agents: Vec<Agent> = serde_json::from_value(profiles).map_err(|_| ())?;
            let backends: HashSet<_> = agents
                .iter()
                .filter_map(|agent| agent.matching_backend(backend_filter.as_deref()))
                .map(str::to_owned)
                .collect();
            let results = stream::iter(backends)
                .map(|backend| {
                    let client = client.clone();
                    async move {
                        let capabilities = client.acp_model_source_capabilities(&backend).await?;
                        let supported = capabilities.model_source_capabilities.contains(
                            &AcpModelSourceCapability::Supported { source: AcpModelSource::CliLocal },
                        );
                        Ok::<_, String>((backend, supported))
                    }
                })
                .buffer_unordered(6)
                .collect::<Vec<_>>()
                .await;
            let supported: HashSet<_> = results
                .iter()
                .filter_map(|result| match result {
                    Ok((backend, true)) => Some(backend.as_str()),
                    _ => None,
                })
                .collect();
            if supported.is_empty() && results.iter().any(Result::is_err) {
                return Err(());
            }
            agents.retain(|agent| {
                agent
                    .matching_backend(backend_filter.as_deref())
                    .is_some_and(|backend| supported.contains(backend))
            });
            agents.sort_by(|left, right| {
                left.label().cmp(right.label()).then(left.key.cmp(&right.key))
            });
            Ok(agents)
        },
        move |result| response(id, generation, Action::AgentsLoaded(epoch, result)),
    )
}

fn refresh_sessions(state: &mut Workbench, id: window::Id) -> Task<Message> {
    if state.refreshing || state.discovering {
        return Task::none();
    }
    let Some(client) = client(state) else { return Task::none() };
    let epoch = state.epoch;
    let projects_epoch = state.projects_epoch;
    let generation = state.scope_generation;
    if state.legacy_projects {
        let projects = legacy_poll_targets(state);
        if projects.is_empty() {
            return Task::none();
        }
        state.refreshing = true;
        return Task::perform(
            async move {
                stream::iter(projects).map(|project| {
                    let client = client.clone();
                    async move {
                        let sessions = client.cli_workbench_list(&project).await;
                        (project, sessions)
                    }
                }).buffer_unordered(6).collect::<Vec<_>>().await
            },
            move |result| response(id, generation, Action::LegacySessionsLoaded(epoch, projects_epoch, result)),
        );
    }
    state.refreshing = true;
    Task::perform(
        async move { client.cli_workbench_projects(None).await.map_err(|_| ()) },
        move |result| response(id, generation, Action::SessionsLoaded(epoch, projects_epoch, result)),
    )
}

pub(super) fn legacy_poll_targets(state: &mut Workbench) -> Vec<String> {
    let candidates: Vec<_> = state.projects.keys()
        .filter(|id| state.selected.as_ref() != Some(*id))
        .collect();
    let mut targets: Vec<_> = state.selected.iter().cloned().collect();
    if !candidates.is_empty() {
        let count = candidates.len().min(6 - targets.len());
        let offset = state.legacy_poll_cursor % candidates.len();
        targets.extend((0..count).map(|step| candidates[(offset + step) % candidates.len()].to_owned()));
        state.legacy_poll_cursor = (offset + count) % candidates.len();
    }
    targets
}

fn refresh_details(state: &mut Workbench, id: window::Id) -> Task<Message> {
    if state.details_refreshing {
        return Task::none();
    }
    let Some(client) = client(state) else { return Task::none() };
    let mut projects = Vec::new();
    for (project_id, project) in &state.projects {
        if project.disconnected {
            continue;
        }
        let mut waiting = HashMap::new();
        let mut drafts = false;
        let mut requests = Vec::new();
        for (session_id, tab) in &project.tabs {
            if tab.session.state == Process::WaitingForApproval {
                waiting.entry(tab.session.directory.clone()).or_insert_with(HashSet::new)
                    .insert(session_id.clone());
            }
            drafts |= state.terminal_available && tab.session.capabilities.delivery
                == vw_api_types::cli_workbench::CliWorkbenchDeliveryCapability::Draft;
            requests.extend(tab.turns.iter()
                .filter(|turn| turn.recover && turn.submitting.is_none())
                .map(|turn| (session_id.clone(), turn.request.clone())));
        }
        if drafts || !waiting.is_empty() || !requests.is_empty() {
            projects.push((project_id.clone(), waiting, drafts, requests));
        }
    }
    if projects.is_empty() {
        return Task::none();
    }
    state.details_refreshing = true;
    let epoch = state.epoch;
    let generation = state.scope_generation;
    Task::perform(
        async move {
            stream::iter(projects)
                .map(|(project, waiting, drafts, requests)| {
                    project_details(client.clone(), project, waiting, drafts, requests)
                })
                .buffer_unordered(6)
                .collect::<Vec<_>>()
                .await
        },
        move |updates| response(id, generation, Action::SessionDetailsLoaded(epoch, updates)),
    )
}

async fn project_details(
    client: GatewayClient,
    project: String,
    waiting: HashMap<String, HashSet<String>>,
    include_drafts: bool,
    requests: Vec<(String, String)>,
) -> ProjectUpdate {
    let drafts = if include_drafts { client.cli_workbench_drafts(&project).await.ok() } else { None };
    let mut approvals = HashMap::new();
    let mut approval_error = false;
    let interaction = GatewayInteractionClient::new(client.transport().clone());
    let mut responses = stream::iter(waiting)
        .map(|(directory, sessions)| {
            let interaction = interaction.clone();
            async move { (sessions, interaction.permission_list(Some(&directory)).await) }
        })
        .buffer_unordered(4);
    while let Some((sessions, result)) = responses.next().await {
        match result {
            Ok(requests) => {
                for request in requests {
                    if sessions.contains(&request.session_id) {
                        approvals.entry(request.session_id.clone()).or_insert(request);
                    }
                }
            }
            Err(_) => approval_error = true,
        }
    }
    let turns = stream::iter(requests)
        .map(|(session, request)| {
            let client = client.clone();
            let project = project.clone();
            async move {
                let result = client.cli_workbench_receive(&project, &session, &request).await;
                (SessionKey { project, session }, request, result)
            }
        })
        .buffer_unordered(4)
        .collect()
        .await;
    ProjectUpdate { project, approvals, approval_error, drafts, turns }
}

pub(super) fn apply_projects(
    state: &mut Workbench,
    snapshot: CliWorkbenchProjects,
    apply_sessions: bool,
) {
    let known: HashSet<_> = snapshot.projects.iter().map(|project| project.id.as_str()).collect();
    state.projects.retain(|id, _| known.contains(id.as_str()));
    for project in snapshot.projects {
        let entry = state.projects.entry(project.id.clone())
            .or_insert_with(|| Project::new(project.name.clone(), project.directory.clone()));
        entry.name = project.name;
        entry.directory = project.directory;
        if apply_sessions {
            let sessions = project.sessions.into_iter()
                .filter(|session| state.surface.includes_session(session))
                .collect();
            state.workspace.merge_sessions(&project.id, entry, sessions);
            entry.disconnected = false;
            if !entry.tabs.values().any(|tab| tab.session.state == Process::WaitingForApproval) {
                entry.approval_error = false;
            }
        }
    }
    state.selected = snapshot.preferred_project.filter(|id| state.projects.contains_key(id))
        .or_else(|| state.selected.take().filter(|id| state.projects.contains_key(id)))
        .or_else(|| {
            if state.surface == Surface::Terminal {
                return None;
            }
            state.workspace.selected()
                .filter(|id| state.projects.contains_key(*id))
                .map(str::to_owned)
        });
    state.terminal_available = snapshot.terminal_available;
}

fn apply_projects_updates(state: &mut Workbench, batch: Vec<ProjectUpdate>) {
    for update in batch {
        let Some(project) = state.projects.get_mut(&update.project) else { continue };
        for (id, tab) in &mut project.tabs {
            if tab.session.state == Process::WaitingForApproval {
                tab.approval = update.approvals.get(id).cloned();
            }
        }
        project.approval_error = update.approval_error;
        if let Some(drafts) = update.drafts {
            for tab in project.tabs.values_mut() {
                tab.drafts.clear();
            }
            for draft in drafts {
                if let Some(tab) = project.tabs.get_mut(&draft.id.target_session_id) {
                    tab.drafts.push(draft);
                }
            }
        }
        for (key, request, result) in update.turns {
            state.apply_turn(&key, &request, result, false);
        }
    }
}

fn update_window(state: &mut Workbench, id: window::Id, action: Action, visible: bool) -> Task<Message> {
    match action {
        Action::Ssh(action) => return ssh::update(state, id, action),
        Action::ShowTerminalGroup if state.surface == Surface::Terminal
            && state.terminal_group_available(state.terminal_directory.as_deref()) =>
        {
            state.terminal_group_selected = true;
            return super::terminal::focus(state);
        }
        Action::Refresh if !state.discovering => return reload_projects(state, id),
        Action::ProjectsLoaded(epoch, session_epoch, result) if epoch == state.projects_epoch => {
            state.discovering = false;
            match result {
                Ok((mut snapshot, legacy)) => {
                    if state.preferred_directory.take().is_none() {
                        snapshot.preferred_project = None;
                    }
                    if state.surface == Surface::Terminal {
                        state.selected = None;
                    }
                    state.legacy_projects = legacy;
                    state.legacy_poll_cursor = 0;
                    let apply_sessions = !legacy && session_epoch == state.epoch;
                    apply_projects(state, snapshot, apply_sessions);
                    let shell = if apply_sessions {
                        start_default_shell(state, id, visible)
                    } else {
                        Task::none()
                    };
                    let sessions = if legacy { refresh_sessions(state, id) } else { refresh_details(state, id) };
                    return Task::batch([load_agents(state, id), sessions, shell]);
                }
                Err(code @ (Code::Unauthorized | Code::Disconnected)) => state.error = Some(error_label(code)),
                Err(_) => state.error = Some("无法读取工作台项目"),
            }
        }
        Action::NativeAppsLoaded(epoch, catalog) if epoch == state.projects_epoch => {
            state.native_apps = catalog.native_apps;
        }
        Action::SessionsLoaded(epoch, projects_epoch, result) => {
            state.refreshing = false;
            if epoch == state.epoch && projects_epoch == state.projects_epoch {
                match result {
                    Ok(snapshot) => {
                        apply_projects(state, snapshot, true);
                        return Task::batch([
                            refresh_details(state, id),
                            start_default_shell(state, id, visible),
                        ]);
                    }
                    Err(()) => {
                        for project in state.projects.values_mut() {
                            project.disconnected = true;
                        }
                    }
                }
            }
        }
        Action::LegacySessionsLoaded(epoch, projects_epoch, updates) => {
            state.refreshing = false;
            if state.legacy_projects && epoch == state.epoch && projects_epoch == state.projects_epoch {
                for (id, result) in updates {
                    let Some(project) = state.projects.get_mut(&id) else { continue };
                    match result {
                        Ok(sessions) => {
                            let sessions = sessions.into_iter()
                                .filter(|session| state.surface.includes_session(session))
                                .collect();
                            state.workspace.merge_sessions(&id, project, sessions);
                            project.disconnected = false;
                            if state.selected.as_ref() == Some(&id)
                                && state.error == Some("当前网关未提供工作台会话接口，请更新网关")
                            {
                                state.error = None;
                            }
                        }
                        Err(error) => {
                            project.disconnected = true;
                            if state.selected.as_ref() == Some(&id) && error.code == Code::NotFound {
                                state.error = Some("当前网关未提供工作台会话接口，请更新网关");
                            }
                        }
                    }
                }
                return Task::batch([
                    refresh_details(state, id),
                    start_default_shell(state, id, visible),
                ]);
            }
        }
        Action::SessionDetailsLoaded(epoch, updates) => {
            state.details_refreshing = false;
            if epoch == state.epoch {
                apply_projects_updates(state, updates);
                if matches!(&state.dialog, Some(Dialog::ReadDraft(key, request)) if draft_text(state, key, request).is_none())
                {
                    state.dialog = None;
                    return super::terminal::focus(state);
                }
            }
        }
        Action::SelectProject(project) if state.projects.contains_key(&project) => {
            state.default_shell_pending = true;
            if state.selected.as_ref() != Some(&project) {
                state.selected = Some(project);
                state.preferred_directory = None;
                state.dialog = None;
                let sessions = if state.legacy_projects {
                    refresh_sessions(state, id)
                } else {
                    Task::none()
                };
                return Task::batch([
                    load_agents(state, id),
                    sessions,
                    start_default_shell(state, id, visible),
                ]);
            }
            return start_default_shell(state, id, visible);
        }
        Action::SelectSession(key) => {
            let changed = state.selected.as_ref() != Some(&key.project);
            if state.locate(&key) {
                state.default_shell_pending = false;
                if changed {
                    state.dialog = None;
                    return Task::batch([load_agents(state, id), super::terminal::focus(state)]);
                }
                return super::terminal::focus(state);
            }
        }
        Action::AgentsLoaded(epoch, result) if epoch == state.agents_epoch => {
            state.agents_loading = false;
            match result {
                Ok(agents) => {
                    state.agents = agents;
                }
                Err(()) => state.error = Some("无法读取可用的 ACP 智能体"),
            }
        }
        Action::Start(project, layout, target, pane)
            if state.selected.as_ref() == Some(&project)
                && state.projects.get(&project).is_some_and(|project| project.active_layout == layout) =>
        {
            return start(state, id, target, pane, None);
        }
        Action::TerminalEvent(key, event) => return super::terminal::event(state, id, key, event),
        Action::TerminalOutput(key, cursor, result) => {
            return super::terminal::output(state, id, key, cursor, result, visible);
        }
        Action::TerminalWritten(key, result) => {
            return super::terminal::written(state, id, key, result, visible);
        }
        Action::Started(project_id, layout, target, pane, direction, result) => {
            state.starting = false;
            state.epoch += 1;
            match result {
                Ok(session) if session.project_id == project_id && session.target == target
                    && state.surface.includes_session(&session) =>
                {
                    if let Some(project) = state.projects.get_mut(&project_id) {
                        let terminal = project.place_started(session, layout, pane, direction);
                        if terminal && state.selected.as_ref() == Some(&project_id) {
                            if visible && state.surface == Surface::Terminal {
                                state.terminal_group_selected = true;
                            }
                            return Task::done(Message::CliWorkbench(Event::TerminalTick));
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
                if state.selected.as_ref() == Some(&key.project) {
                    state.default_shell_pending = false;
                }
            }
            if state.surface == Surface::Terminal && !state.terminal_group_available(state.terminal_directory.as_deref()) {
                state.select_local_terminal_group();
            }
            return super::terminal::focus(state);
        }
        Action::Stop(key) => return stop(state, id, key),
        Action::Stopped(key, result) => {
            state.epoch += 1;
            if let Some(tab) = state.tab_mut(&key) {
                tab.finish_stop(result);
            }
        }
        Action::Rename(key) => {
            if let Some(tab) = state.projects.get(&key.project).and_then(|project| project.tabs.get(&key.session)) {
                let name = super::view::tab_label(state, tab);
                state.dialog = Some(Dialog::Rename(key, name));
                return iced::widget::operation::focus(super::view::DIALOG_INPUT);
            }
        }
        Action::Compose(target) => {
            state.dialog = Some(Dialog::Compose { target, draft: String::new() });
            return iced::widget::operation::focus(super::view::DIALOG_INPUT);
        }
        Action::Draft(value) => match state.dialog.as_mut() {
            Some(Dialog::Compose { draft, .. } | Dialog::Rename(_, draft)
                | Dialog::RenameLayout(_, _, draft)) => *draft = value,
            _ => {}
        },
        Action::Target(value) => {
            if let Some(Dialog::Compose { target, .. }) = state.dialog.as_mut() {
                *target = Some(value);
            }
        }
        Action::Dismiss => {
            state.dialog = None;
            return super::terminal::focus(state);
        }
        Action::ShowDraft(key, request) => {
            let changed = state.selected.as_ref() != Some(&key.project);
            if draft_text(state, &key, &request).is_some() && state.locate(&key) {
                state.dialog = Some(Dialog::ReadDraft(key, request));
                if changed {
                    return load_agents(state, id);
                }
            }
        }
        Action::CopyDraft(key, request) => {
            if let Some(text) = draft_text(state, &key, &request) {
                return iced::clipboard::write(text.to_owned());
            }
        }
        Action::PasteDraft(key, request) => {
            if matches!(&state.dialog, Some(Dialog::ReadDraft(target, current)) if target == &key && current == &request)
                && let Some(text) = draft_text(state, &key, &request).map(str::to_owned)
            {
                return super::terminal::paste_prompt(state, id, key, text, false);
            }
        }
        Action::ResolveDraft(key, request, action) => {
            return resolve_draft(state, id, key, request, action);
        }
        Action::DraftResolved(key, request, result) => {
            state.epoch += 1;
            if let Some(tab) = state.tab_mut(&key) {
                tab.resolving_draft = None;
                match result {
                    Ok(message)
                        if message.id.request_id == request
                            && message.id.target_session_id == key.session =>
                    {
                        tab.drafts.retain(|draft| draft.id.request_id != request);
                        if matches!(&state.dialog, Some(Dialog::ReadDraft(target, current)) if target == &key && current == &request)
                        {
                            state.dialog = None;
                        }
                        state.apply_turn(&key, &request, Ok(message), false);
                        return Task::batch([
                            super::terminal::focus(state),
                            refresh_sessions(state, id),
                        ]);
                    }
                    Ok(_) => state.error = Some("网关返回无效的工作台会话"),
                    Err(error) => state.error = Some(error_label(error.code)),
                }
            }
        }
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
        Action::Panel(project, layout, message) => return panel(state, id, &project, layout, message),
        Action::GroupAgentsByProject(grouped) => state.group_agents_by_project = grouped,
        Action::ResizeSidebarWidth(event) => {
            state.sidebar_layout.resize_width(event);
        }
        Action::ResizeSidebarSections(event) => {
            state.sidebar_layout.sections.resize(event.split, event.ratio.clamp(0.23, 0.77));
        }
        Action::NewLayout(project) if state.selected.as_ref() == Some(&project) => {
            if let Some(project) = state.projects.get_mut(&project) {
                project.new_layout();
            }
        }
        Action::SelectLayout(project, layout) if state.selected.as_ref() == Some(&project) => {
            if state.projects.get_mut(&project).is_some_and(|project| project.select_layout(layout)) {
                return super::terminal::focus(state);
            }
        }
        Action::CloseLayout(project, layout) if state.selected.as_ref() == Some(&project) => {
            if state.projects.get_mut(&project).is_some_and(|project| project.close_layout(layout)) {
                state.default_shell_pending = false;
                return super::terminal::focus(state);
            }
        }
        Action::OpenLayoutMenu(project, layout, point)
            if state.selected.as_ref() == Some(&project) && state.dialog.is_none() =>
        {
            if let Some(project) = state.projects.get_mut(&project).filter(|project| {
                project.active_layout == layout || project.parked_layouts.contains_key(&layout)
            }) {
                project.tab_menu = None;
                project.content_menu = None;
                project.panes.split_menu = None;
                project.layout_menu = Some((layout, point));
            }
        }
        Action::CloseLayoutMenu(project) => {
            if let Some(project) = state.projects.get_mut(&project) {
                project.layout_menu = None;
            }
        }
        Action::RenameLayout(project, layout, title)
            if state.selected.as_ref() == Some(&project) && state.dialog.is_none() =>
        {
            if let Some(current) = state.projects.get_mut(&project).filter(|current| {
                current.active_layout == layout || current.parked_layouts.contains_key(&layout)
            }) {
                current.layout_menu = None;
                state.dialog = Some(Dialog::RenameLayout(project, layout, title));
                return iced::widget::operation::focus(super::view::DIALOG_INPUT);
            }
        }
        Action::TerminalClipboard(key, terminal_id, text) if visible => {
            if let Some(text) = text {
                return super::terminal::event(state, id, key, iced_term::Event::BackendCall(
                    terminal_id, iced_term::BackendCommand::Paste(text),
                ));
            }
        }
        _ => {}
    }
    Task::none()
}

fn panel(
    state: &mut Workbench,
    id: window::Id,
    project_id: &str,
    layout: u64,
    event: desktop_terminal::message::Message,
) -> Task<Message> {
    use desktop_terminal::message::{ContentAction, Message as Panel, TerminalMessage as Term};

    if state.selected.as_deref() != Some(project_id) || state.dialog.is_some()
        || state.projects.get(project_id).is_none_or(|project| project.active_layout != layout)
    {
        return Task::none();
    }
    let event = match event {
        Panel::Terminal(event) => event,
        Panel::Batch(events) => {
            return Task::batch(events.into_iter().map(|event| panel(state, id, project_id, layout, event)));
        }
        Panel::Hide => return Task::none(),
    };
    let Some(project) = state.projects.get_mut(project_id) else { return Task::none() };
    let key = |view_id| project.tab_for_view(view_id).map(|tab| SessionKey {
        project: project_id.into(), session: tab.session.id.clone(),
    });
    match event {
        Term::Select(view_id) => {
            if let Some(key) = key(view_id) {
                return update_window(state, id, Action::SelectSession(key), true);
            }
        }
        Term::Close(view_id) => {
            if let Some(key) = key(view_id) {
                return update_window(state, id, Action::Hide(key), true);
            }
        }
        Term::RenameStart(view_id) => {
            if let Some(key) = key(view_id) {
                project.tab_menu = None;
                return update_window(state, id, Action::Rename(key), true);
            }
        }
        Term::TabContextOpen(view_id, x, y) => project.tab_menu = Some((view_id, x, y)),
        Term::TabContextClose => project.tab_menu = None,
        Term::ContentMenuOpen(view_id, position) => {
            project.panes.select(view_id);
            project.content_menu = Some((view_id, position));
            return super::terminal::focus(state);
        }
        Term::ContentMenuClose => project.content_menu = None,
        Term::ClipboardShortcut(widget_id, action) => {
            let Some(tab) = project.active_tab().filter(|tab| project.panes.is_visible_tab(tab.view_id)) else {
                return Task::none();
            };
            let Some(view) = tab.terminal.view.as_ref().filter(|view| view.widget_id() == widget_id) else {
                return Task::none();
            };
            let key = SessionKey { project: project_id.into(), session: tab.session.id.clone() };
            return terminal_clipboard(id, state.scope_generation, key, view, action);
        }
        Term::ToggleSplitMenu(pane) => {
            project.panes.split_menu = (project.panes.split_menu != Some(pane)).then_some(pane);
        }
        Term::CloseSplitMenu => project.panes.split_menu = None,
        Term::ResizePane(instance, event) if instance == project.panes.instance => {
            project.panes.resize(event);
        }
        Term::DockTab(instance, pane, tab, target) if instance == project.panes.instance => {
            if project.panes.dock_tab(pane, tab, target) {
                project.tab_menu = None;
                project.content_menu = None;
                state.checkpoint_layout();
                return super::terminal::focus(state);
            }
        }
        Term::TogglePaneMaximized(instance, pane) if instance == project.panes.instance => {
            if project.panes.grid.get(pane).is_some() {
                if project.panes.grid.maximized() == Some(pane) {
                    project.panes.grid.restore();
                } else {
                    project.panes.grid.maximize(pane);
                }
                project.panes.focused = pane;
                return super::terminal::focus(state);
            }
        }
        Term::Split(instance, pane, direction) if instance == project.panes.instance => {
            project.panes.split_menu = None;
            if let Some(target) = split_target(state, &state.projects[project_id], pane) {
                return start(state, id, target, pane, Some(direction));
            }
        }
        Term::ContentMenuAction(view_id, action) => {
            let Some(key) = key(view_id) else { return Task::none() };
            project.content_menu = None;
            let pane = project.panes.focused;
            match action {
                ContentAction::NewTerminal | ContentAction::Split(_) => {
                    let direction = match action {
                        ContentAction::Split(direction) => Some(direction),
                        _ => None,
                    };
                    let target = project.tab_for_view(view_id)
                        .map(|tab| &tab.session.target)
                        .filter(|target| matches!(target, CliWorkbenchTarget::Ssh { .. }))
                        .cloned()
                        .unwrap_or(CliWorkbenchTarget::Shell);
                    return start(state, id, target, pane, direction);
                }
                ContentAction::CloseTab => return update_window(state, id, Action::Hide(key), true),
                ContentAction::Copy | ContentAction::Paste | ContentAction::PasteText => {
                    if let Some(view) = project.active_tab().filter(|tab| tab.view_id == view_id)
                        .and_then(|tab| tab.terminal.view.as_ref())
                    {
                        return terminal_clipboard(id, state.scope_generation, key, view, action);
                    }
                }
                ContentAction::SelectAll | ContentAction::Clear => {
                    let command = if matches!(action, ContentAction::SelectAll) {
                        iced_term::BackendCommand::SelectAll
                    } else {
                        iced_term::BackendCommand::Clear
                    };
                    return super::terminal::backend_action(state, id, key, command);
                }
                ContentAction::AskInChat | ContentAction::InsertInChat => {}
            }
        }
        _ => {}
    }
    Task::none()
}

fn terminal_clipboard(
    id: window::Id,
    generation: u64,
    key: SessionKey,
    view: &desktop_terminal::attached::AttachedTerminal,
    action: desktop_terminal::message::ContentAction,
) -> Task<Message> {
    use desktop_terminal::message::ContentAction;

    match action {
        ContentAction::Copy => {
            let text = view.terminal().selected_text();
            if text.is_empty() { Task::none() } else { iced::clipboard::write(text) }
        }
        ContentAction::Paste | ContentAction::PasteText => {
            let terminal_id = view.terminal().id;
            iced::clipboard::read().map(move |text| {
                response(id, generation, Action::TerminalClipboard(key.clone(), terminal_id, text))
            })
        }
        _ => Task::none(),
    }
}

pub(super) fn split_target(state: &Workbench, project: &Project, pane: pane_grid::Pane) -> Option<CliWorkbenchTarget> {
    let current = project.tab_for_view(project.panes.grid.get(pane)?.active_id?)?;
    let target = if matches!(current.session.target, CliWorkbenchTarget::Ssh { .. }) {
        current.session.target.clone()
    } else if state.terminal_available {
        CliWorkbenchTarget::Shell
    } else {
        current.session.target.clone()
    };
    launch_target_available(state, &target).then_some(target)
}

pub(super) fn launch_target_available(state: &Workbench, target: &CliWorkbenchTarget) -> bool {
    match target {
        CliWorkbenchTarget::Shell => state.surface != Surface::Terminal && state.terminal_available,
        CliWorkbenchTarget::Ssh { connection_id } => state.ssh.connection(connection_id).is_some(),
        CliWorkbenchTarget::NativeCli { app_id } => {
            // Keep the workbench's older gateway fallback. The gateway authorizes the launch.
            let legacy_codex = state.surface == Surface::Workbench && app_id == super::CODEX_APP_ID;
            state.surface != Surface::Terminal && state.terminal_available
                && (legacy_codex || state.native_apps.iter().any(|app| &app.id == app_id))
        }
        CliWorkbenchTarget::Acp { agent } => {
            state.surface == Surface::Workbench && state.agents.iter().any(|profile| &profile.key == agent)
        }
    }
}

fn start_default_shell(state: &mut Workbench, id: window::Id, visible: bool) -> Task<Message> {
    if !visible || !state.default_shell_pending || state.discovering {
        return Task::none();
    }
    let Some(project) = state.selected.as_ref().and_then(|id| state.projects.get(id)) else {
        return Task::none();
    };
    if !project.layout_loaded || project.disconnected {
        return Task::none();
    }

    if state.dialog.is_some() || project.tabs.values().any(|tab| !tab.hidden) {
        state.default_shell_pending = false;
        return Task::none();
    }
    if state.starting {
        return Task::none();
    }
    let pane = project.panes.focused;
    state.default_shell_pending = false;
    start(state, id, CliWorkbenchTarget::Shell, pane, None)
}

pub(super) fn start(
    state: &mut Workbench,
    id: window::Id,
    target: CliWorkbenchTarget,
    pane: pane_grid::Pane,
    direction: Option<pane_grid::Direction>,
) -> Task<Message> {
    let (Some(project), Some(client)) = (state.selected.clone(), client(state)) else {
        return Task::none();
    };
    let Some(project_state) = state.projects.get(&project) else { return Task::none() };
    let Some(pane_state) = project_state.panes.grid.get(pane) else { return Task::none() };
    if state.starting || project_state.disconnected || !launch_target_available(state, &target) {
        return Task::none();
    }
    let directory = if direction.is_some() {
        let Some(tab) = pane_state.active_id.and_then(|id| project_state.tab_for_view(id)) else {
            return Task::none();
        };
        Some(tab.session.directory.clone())
    } else {
        None
    };
    let layout = project_state.active_layout;
    state.starting = true;
    let generation = state.scope_generation;
    state.error = None;
    let ssh_surface = (matches!(&target, CliWorkbenchTarget::Ssh { .. })
        && state.surface != Surface::AppTerminal).then(|| state.surface.ssh_surface());
    let request = StartCliWorkbenchSession { target, directory, ssh_surface };
    Task::perform(
        async move {
            let result = client.cli_workbench_start(&project, &request).await;
            (project, request.target, result)
        },
        move |(project, target, result)| response(id, generation, Action::Started(project, layout, target, pane, direction, result)),
    )
}

fn stop(state: &mut Workbench, id: window::Id, key: SessionKey) -> Task<Message> {
    let Some(client) = client(state) else { return Task::none() };
    let generation = state.scope_generation;
    if state.projects.get(&key.project).is_none_or(|project| project.disconnected) {
        return Task::none();
    }
    let allowed = state.projects.get(&key.project)
        .and_then(|project| project.tabs.get(&key.session))
        .is_some_and(|tab| !tab.session.capabilities.terminal || state.pty_available(&tab.session.target));
    let Some(tab) = state.tab_mut(&key).filter(|_| allowed) else {
        return Task::none();
    };
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
        Some(Dialog::RenameLayout(_, _, draft)) if !draft.trim().is_empty() => {
            if let Some(Dialog::RenameLayout(project, layout, draft)) = state.dialog.take()
                && let Some(project) = state.projects.get_mut(&project)
                && (project.active_layout == layout || project.parked_layouts.contains_key(&layout))
            {
                project.layout_names.insert(layout, draft.trim().chars().take(80).collect());
            }
            return super::terminal::focus(state);
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
            if state.projects.get(&key.project).and_then(|project| project.tabs.get(&key.session))
                .is_some_and(|tab| tab.session.capabilities.terminal)
            {
                return super::terminal::paste_prompt(state, id, key, prompt, true);
            }
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
                prompt: Some(prompt.clone()),
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

pub(super) fn draft_text<'a>(
    state: &'a Workbench,
    key: &SessionKey,
    request: &str,
) -> Option<&'a str> {
    state
        .projects
        .get(&key.project)?
        .tabs
        .get(&key.session)?
        .drafts
        .iter()
        .find(|draft| draft.id.request_id == request)
        .map(|draft| draft.text.as_str())
}

fn resolve_draft(
    state: &mut Workbench,
    id: window::Id,
    key: SessionKey,
    request: String,
    action: CliWorkbenchDraftAction,
) -> Task<Message> {
    if !state.terminal_available
        || state.projects.get(&key.project).is_none_or(|project| project.disconnected)
        || !matches!(&state.dialog, Some(Dialog::ReadDraft(target, current)) if target == &key && current == &request)
        || draft_text(state, &key, &request).is_none()
    {
        return Task::none();
    }
    let Some(client) = client(state) else { return Task::none() };
    let Some(tab) = state.tab_mut(&key) else { return Task::none() };
    if tab.resolving_draft.is_some() {
        return Task::none();
    }
    tab.resolving_draft = Some(request.clone());
    state.error = None;
    let generation = state.scope_generation;
    Task::perform(
        async move {
            let result = client
                .cli_workbench_resolve_draft(&key.project, &key.session, &request, action)
                .await;
            (key, request, result)
        },
        move |(key, request, result)| {
            response(id, generation, Action::DraftResolved(key, request, result))
        },
    )
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
