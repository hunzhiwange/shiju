use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use desktop_terminal::workspace::{read_json, write_json};
use desktop_workspace::views::modals::recent_project_remove;
use iced::{Element, Point, Task};
use sha2::{Digest, Sha256};
use vw_api_project::{GetProjectResponse, ProjectDto};
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use crate::app::message::{ProjectMessage, project};
use crate::app::state::ToastKind;
use crate::app::{App, Message, RecentProjectMeta, Screen};

type RemovedProjects = BTreeMap<String, BTreeSet<String>>;

#[derive(Clone)]
pub(crate) enum Event {
    OpenMenu(Option<String>, String, Point),
    CloseMenu,
    Edit,
    EditLoaded(u64, Result<GetProjectResponse, String>),
    Remove,
    RemoveCanceled,
    RemoveConfirmed(String),
    Add,
    Added(String, Result<Option<(ProjectDto, String)>, String>),
    RefreshTerminal,
    TerminalProjectsLoaded(u64, String, bool, Result<Vec<ProjectDto>, String>),
    SearchToggled(Screen),
    SearchChanged(Screen, String),
    SearchClosed(Screen),
}

pub(crate) const SEARCH_INPUT: &str = "workbench-project-search";

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Workbench project event")
    }
}

pub(crate) struct Menu {
    pub(crate) directory: String,
    pub(crate) point: Point,
    id: Option<String>,
    screen: Screen,
    scope: String,
}

pub(crate) struct Search {
    screen: Screen,
    pub(crate) query: String,
    pub(crate) open: bool,
}

#[derive(Default)]
pub(crate) struct State {
    pub(crate) menu: Option<Menu>,
    pub(crate) adding: bool,
    search: Option<Search>,
    terminal_projects: Option<BTreeMap<String, ProjectDto>>,
    pub(crate) terminal_loading: bool,
    terminal_request: u64,
    removing: Option<(String, String)>,
    editing: Option<(String, Screen, String)>,
    edit_request: u64,
    removed: RemovedProjects,
    storage: Option<PathBuf>,
    active_scope: String,
}

impl State {
    pub(crate) fn load() -> Self {
        let storage = if cfg!(test) {
            None
        } else {
            crate::app::project_dirs().map(|dirs| {
                dirs.data_local_dir().join("workbench-layouts").join("removed-projects.json")
            })
        };
        let removed = storage.as_deref().and_then(|path| match read_json(path) {
            Ok(removed) => removed,
            Err(error) => {
                tracing::warn!(kind = ?error.kind(), "failed to read removed workbench projects");
                None
            }
        });
        Self {
            storage,
            removed: removed.unwrap_or_default(),
            active_scope: scope_key(&crate::app::config::gateway_client_endpoint()),
            ..Self::default()
        }
    }

    pub(crate) fn bind_endpoint(&mut self, endpoint: &GatewayEndpoint) {
        let scope = scope_key(endpoint);
        if self.active_scope != scope {
            self.active_scope = scope;
            self.menu = None;
            self.removing = None;
            self.editing = None;
            self.search = None;
            self.terminal_projects = None;
            self.terminal_loading = false;
            self.terminal_request = self.terminal_request.wrapping_add(1);
        }
    }

    pub(crate) fn removed(&self, endpoint: &GatewayEndpoint) -> Option<&BTreeSet<String>> {
        self.removed.get(&scope_key(endpoint))
    }

    pub(crate) fn active_removed(&self) -> Option<&BTreeSet<String>> {
        self.removed.get(&self.active_scope)
    }

    pub(crate) fn menu(&self, screen: Screen) -> Option<&Menu> {
        self.menu.as_ref().filter(|menu| menu.screen == screen)
    }

    pub(crate) fn search(&self, screen: Screen) -> Option<&Search> {
        self.search.as_ref().filter(|search| search.screen == screen)
    }

    pub(crate) fn terminal_projects(&self) -> Option<&BTreeMap<String, ProjectDto>> {
        self.terminal_projects.as_ref()
    }

    fn save(&mut self, removed: RemovedProjects) -> std::io::Result<()> {
        if removed != self.removed {
            if let Some(path) = &self.storage {
                write_json(path, &removed)?;
            }
            self.removed = removed;
        }
        Ok(())
    }
}

fn scope_key(endpoint: &GatewayEndpoint) -> String {
    let tenant = endpoint.tenant_scope();
    let scope = serde_json::json!([
        endpoint.normalized_host(),
        endpoint.port,
        tenant.platform_id(),
        tenant.company_id(),
        endpoint.user_id(),
    ]);
    hex::encode(Sha256::digest(scope.to_string().as_bytes()))
}

pub(crate) fn with_remove_confirmation<'a>(
    app: &'a App,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    recent_project_remove::with_recent_project_remove_confirm(
        app.workbench_projects.removing.as_ref().map(|(_, directory)| directory),
        &app.workspace_state.recent_projects,
        &app.workspace_state.recent_projects_edits,
        &app.workspace_state.recent_projects_meta,
        app.general_settings.desktop_locale,
        |event| {
            Message::WorkbenchProjects(match event {
                recent_project_remove::Event::RecentRemoveCanceled => Event::RemoveCanceled,
                recent_project_remove::Event::RecentRemoveConfirmed(path) => {
                    Event::RemoveConfirmed(path)
                }
            })
        },
        base,
    )
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::OpenMenu(id, directory, point) => {
            if let Some(search) = &mut app.workbench_projects.search {
                search.open = false;
            }
            app.workbench_projects.menu = Some(Menu {
                id,
                directory,
                point,
                screen: app.navigation_tabs.screen,
                scope: scope_key(&crate::app::config::gateway_client_endpoint()),
            });
        }
        Event::CloseMenu => app.workbench_projects.menu = None,
        Event::Edit => return edit(app),
        Event::EditLoaded(request, result) if request == app.workbench_projects.edit_request => {
            if let Some((scope, screen, path)) = app.workbench_projects.editing.take()
                && screen == app.navigation_tabs.screen
                && scope == scope_key(&crate::app::config::gateway_client_endpoint())
                && app.workbench_projects.removing.is_none()
                && app.project_editor.project_edit_path.is_none()
            {
                match result {
                    Ok(detail) => {
                        let meta = remember_project_metadata(app, &detail.project, &path);
                        if let Some(icon) = detail.icon {
                            meta.icon = icon.override_icon;
                            meta.icon_color = icon.color;
                        }
                        meta.worktree_start_command =
                            detail.commands.and_then(|command| command.start);
                        update_recent_labels(app);
                        return project::update(app, ProjectMessage::ProjectEditOpened(path));
                    }
                    Err(_) => {
                        return app.show_localized_toast(ToastKind::Error, "读取项目失败", vec![]);
                    }
                }
            }
        }
        Event::Remove => {
            if let Some(menu) = app.workbench_projects.menu.take()
                && menu.screen == app.navigation_tabs.screen
                && menu.scope == scope_key(&crate::app::config::gateway_client_endpoint())
                && !desktop_workspace::projects::is_builtin_workspace_project(&menu.directory)
            {
                app.workbench_projects.removing = Some((menu.scope, menu.directory));
            }
        }
        Event::RemoveCanceled => app.workbench_projects.removing = None,
        Event::RemoveConfirmed(path) => return remove(app, path),
        Event::Add if !app.workbench_projects.adding => return add(app),
        Event::Added(scope, result) => {
            app.workbench_projects.adding = false;
            if scope != scope_key(&crate::app::config::gateway_client_endpoint()) {
                return Task::none();
            }
            match result {
                Ok(Some((project, picked))) => {
                    let mut removed = app.workbench_projects.removed.clone();
                    if let Some(paths) = removed.get_mut(&scope) {
                        paths.remove(&picked);
                        paths.remove(&project.directory);
                        if paths.is_empty() {
                            removed.remove(&scope);
                        }
                    }
                    if app.workbench_projects.save(removed).is_err() {
                        return app.show_localized_toast(ToastKind::Error, "添加项目失败", vec![]);
                    }
                    let path = project.directory.clone();
                    remember_project_metadata(app, &project, &path);
                    crate::app::projects::ensure_recent_project(
                        &mut app.workspace_state.recent_projects,
                        path,
                    );
                    update_recent_labels(app);
                    crate::app::projects::save_recent_projects_meta_background(
                        app.workspace_state.recent_projects_meta.clone(),
                    );
                    return refresh_lists(app);
                }
                Ok(None) => {}
                Err(_) => {
                    return app.show_localized_toast(ToastKind::Error, "添加项目失败", vec![]);
                }
            }
        }
        Event::RefreshTerminal => {
            let removed = app.workbench_projects.active_removed();
            let recent = &mut app.workspace_state.recent_projects;
            for path in crate::app::projects::load_recent_projects() {
                if removed.is_none_or(|paths| !paths.contains(&path)) && !recent.contains(&path) {
                    recent.push(path);
                }
            }
            update_recent_labels(app);
            return refresh_terminal(app, true);
        }
        Event::TerminalProjectsLoaded(request, scope, show_error, result)
            if request == app.workbench_projects.terminal_request
                && scope == app.workbench_projects.active_scope =>
        {
            app.workbench_projects.terminal_loading = false;
            match result {
                Ok(projects) => {
                    app.workbench_projects.terminal_projects = Some(
                        projects
                            .into_iter()
                            .filter(|project| !project.directory.is_empty())
                            .map(|project| (project.directory.clone(), project))
                            .collect(),
                    );
                }
                Err(_) if show_error => {
                    return app.show_localized_toast(ToastKind::Error, "读取项目失败", vec![]);
                }
                Err(_) => {}
            }
        }
        Event::SearchToggled(screen) if screen == app.navigation_tabs.screen => {
            app.workbench_projects.menu = None;
            let search = &mut app.workbench_projects.search;
            if let Some(search) = search.as_mut().filter(|search| search.screen == screen) {
                search.open = !search.open;
            } else {
                *search = Some(Search { screen, query: String::new(), open: true });
            }
            if search.as_ref().is_some_and(|search| search.open) {
                return iced::widget::operation::focus(SEARCH_INPUT);
            }
        }
        Event::SearchChanged(screen, query) => {
            if let Some(search) =
                app.workbench_projects.search.as_mut().filter(|search| search.screen == screen)
            {
                search.query = query;
                if search.open && search.query.is_empty() {
                    return iced::widget::operation::focus(SEARCH_INPUT);
                }
            }
        }
        Event::SearchClosed(screen) => {
            if let Some(search) =
                app.workbench_projects.search.as_mut().filter(|search| search.screen == screen)
            {
                search.open = false;
            }
        }
        _ => {}
    }
    Task::none()
}

fn edit(app: &mut App) -> Task<Message> {
    let Some(Menu { id, directory, screen, scope, .. }) = app.workbench_projects.menu.take() else {
        return Task::none();
    };
    let endpoint = crate::app::config::gateway_client_endpoint();
    if screen != app.navigation_tabs.screen || scope != scope_key(&endpoint) {
        return Task::none();
    }
    app.workbench_projects.edit_request = app.workbench_projects.edit_request.wrapping_add(1);
    app.workbench_projects.editing = None;
    let Some(id) = id else {
        return project::update(app, ProjectMessage::ProjectEditOpened(directory));
    };
    let request = app.workbench_projects.edit_request;
    app.workbench_projects.editing = Some((scope, screen, directory));
    Task::perform(
        async move {
            let client = GatewayClient::new(endpoint)?;
            let scope = vw_api_transport::id::ProjectScope::new(
                client.endpoint().tenant_scope().clone(),
                id.as_str().into(),
            )
            .map_err(|error| error.to_string())?;
            gateway_project::GatewayProjectClient::new(client.transport().clone())
                .project_get(&scope)
                .await
        },
        move |result| Message::WorkbenchProjects(Event::EditLoaded(request, result)),
    )
}

fn add(app: &mut App) -> Task<Message> {
    let endpoint = crate::app::config::gateway_client_endpoint();
    if !desktop_settings_workspace::environment::can_add(&endpoint) {
        return app.show_localized_toast(ToastKind::Error, "远程网关不支持选择本机目录。", vec![]);
    }
    app.workbench_projects.adding = true;
    let scope = scope_key(&endpoint);
    Task::perform(
        async move {
            let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await else {
                return Ok(None);
            };
            let directory = folder.path().to_str().ok_or("Invalid project path")?.to_owned();
            let client = GatewayClient::new(endpoint)?;
            let project = desktop_workspace::project_message::gateway::resolve_gateway_project(
                &client,
                client.endpoint().tenant_scope(),
                &directory,
            )
            .await?;
            Ok(Some((project, directory)))
        },
        move |result| Message::WorkbenchProjects(Event::Added(scope, result)),
    )
}

fn remove(app: &mut App, path: String) -> Task<Message> {
    let scope = scope_key(&crate::app::config::gateway_client_endpoint());
    if !app
        .workbench_projects
        .removing
        .as_ref()
        .is_some_and(|(owner, directory)| owner == &scope && directory == &path)
    {
        app.workbench_projects.removing = None;
        return Task::none();
    }
    let mut removed = app.workbench_projects.removed.clone();
    removed.entry(scope).or_default().insert(path.clone());
    if app.workbench_projects.save(removed).is_err() {
        return app.show_localized_toast(ToastKind::Error, "移除项目失败", vec![]);
    }
    app.workbench_projects.removing = None;
    super::cli_workbench::remove_project(app, &path);
    super::git_workbench::remove_project(app, &path);
    if app.terminal_project_override.as_deref() == Some(&path) {
        if let Some(directory) = app.navigation_tabs.project_path.clone() {
            app.select_terminal_project(directory);
        } else {
            app.switch_project_terminal(Some(path.clone()), None);
        }
    }
    Task::batch([
        project::update(app, ProjectMessage::RecentRemoveConfirmed(path)),
        refresh_lists(app),
    ])
}

pub(crate) fn refresh_lists(app: &mut App) -> Task<Message> {
    if !app.terminal.is_visible || app.terminal_projects_collapsed {
        let state = &mut app.workbench_projects;
        state.terminal_projects = None;
        state.terminal_loading = false;
        state.terminal_request = state.terminal_request.wrapping_add(1);
    }
    Task::batch([
        super::cli_workbench::refresh_projects(app),
        super::git_workbench::projects_changed(app),
        refresh_terminal(app, false),
    ])
}

pub(crate) fn ensure_terminal_projects(app: &mut App) -> Task<Message> {
    if app.workbench_projects.terminal_projects.is_some() || app.workbench_projects.terminal_loading
    {
        return Task::none();
    }
    refresh_terminal(app, false)
}

fn refresh_terminal(app: &mut App, show_error: bool) -> Task<Message> {
    if !app.terminal.is_visible || app.terminal_projects_collapsed {
        return Task::none();
    }
    let endpoint = crate::app::config::gateway_client_endpoint();
    let state = &mut app.workbench_projects;
    state.bind_endpoint(&endpoint);
    if !endpoint.is_loopback() {
        return Task::none();
    }
    state.terminal_request = state.terminal_request.wrapping_add(1);
    state.terminal_loading = true;
    let request = state.terminal_request;
    let scope = state.active_scope.clone();
    Task::perform(
        async move {
            let client = GatewayClient::new(endpoint)?;
            desktop_workspace::project_message::gateway::load_recent_projects(
                &client,
                client.endpoint().tenant_scope(),
            )
            .await
        },
        move |result| {
            Message::WorkbenchProjects(Event::TerminalProjectsLoaded(
                request, scope, show_error, result,
            ))
        },
    )
}

fn remember_project_metadata<'a>(
    app: &'a mut App,
    project: &ProjectDto,
    path: &str,
) -> &'a mut RecentProjectMeta {
    let meta = &mut app.workspace_state.recent_projects_meta;
    if let Some(index) = meta.iter().position(|entry| entry.path == path) {
        meta[index].name.clone_from(&project.name);
        return &mut meta[index];
    }
    meta.push(RecentProjectMeta {
        path: path.to_owned(),
        name: project.name.clone(),
        task_board_settings: None,
        session_auto_refresh: crate::app::state::default_recent_project_session_auto_refresh(),
        session_refresh_interval_seconds:
            crate::app::state::default_recent_project_session_refresh_interval_seconds(),
        icon: None,
        icon_color: None,
        worktree_start_command: None,
    });
    meta.last_mut().expect("metadata was inserted")
}

fn update_recent_labels(app: &mut App) {
    let workspace = &mut app.workspace_state;
    workspace.recent_projects_edits = desktop_workspace::projects::recent_project_labels(
        &workspace.recent_projects,
        &workspace.recent_projects_meta,
    );
}
