//! 独立标签页的项目、工作树和文件浏览状态。
use crate::app::message::git::{self, GitMessage, Source};
use crate::app::{App, Message, Screen};
use desktop_git_ui::state::State as GitState;
use iced::Task;
use iced::widget::pane_grid;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

mod files;
mod find;
mod view;
pub(crate) use files::{clipboard_shortcut, preview_diff_file, preview_snapshot, save_active};
pub(crate) use view::{inline_project_picker, view, with_overlays};

pub(crate) const TAB_ID: &str = "git_workbench";

#[derive(Clone)]
pub(crate) enum Event {
    Open,
    RefreshProjects,
    ProjectsReady(u64, Result<Vec<Project>, String>),
    SelectProject(String),
    SelectInlineProject(String),
    Resize(pane_grid::ResizeEvent),
    ShowEditor(bool),
    RefreshCheckout,
    RepositoryReady(u64, Option<String>),
    SwitchBranch(String),
    BranchesReady(u64, Result<Vec<String>, String>),
    BranchSwitched(u64, Result<(), String>),
    ToggleSummary,
    Git(u64, Box<GitMessage>),
    Files(u64, files::Event),
}

impl std::fmt::Debug for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Git workbench event")
    }
}

#[derive(Clone)]
pub(crate) struct Project {
    id: String,
    name: String,
    directory: String,
}

pub(crate) struct Workbench {
    endpoint: Option<GatewayEndpoint>,
    projects: Vec<Project>,
    projects_request: u64,
    projects_loading: bool,
    projects_dirty: bool,
    projects_error: Option<String>,
    selected: Option<String>,
    pending_directory: Option<String>,
    layout: pane_grid::State<bool>,
    show_editor: bool,
    files: files::State,
    pub(crate) git: GitState,
    epoch: u64,
    repo: Option<String>,
    repo_loading: bool,
    switching_branch: bool,
}

impl Default for Workbench {
    fn default() -> Self {
        let (mut layout, sidebar) = pane_grid::State::new(false);
        let (_, divider) =
            layout.split(pane_grid::Axis::Vertical, sidebar, true).expect("project sidebar exists");
        layout.resize(divider, 0.11);
        Self {
            endpoint: None,
            projects: Vec::new(),
            projects_request: 0,
            projects_loading: false,
            projects_dirty: false,
            projects_error: None,
            selected: None,
            pending_directory: None,
            layout,
            show_editor: false,
            files: Default::default(),
            git: GitState::new(true, true),
            epoch: 0,
            repo: None,
            repo_loading: false,
            switching_branch: false,
        }
    }
}

impl Workbench {
    fn project(&self) -> Option<&Project> {
        let selected = self.selected.as_deref()?;
        self.projects.iter().find(|project| project.id == selected)
    }

    pub(crate) fn project_directory(&self) -> Option<&str> {
        self.project().map(|project| project.directory.as_str())
    }

    pub(super) fn switcher_project_directory(&self) -> Option<&str> {
        self.endpoint.as_ref()
            .filter(|endpoint| same_endpoint(endpoint, &crate::app::config::gateway_client_endpoint()))?;
        self.project_directory()
    }

    fn checkout_directory(&self) -> Option<&str> {
        self.git.selected_git_worktree_directory.as_deref().or_else(|| self.project_directory())
    }

    pub(crate) fn repo_path(&self) -> Option<String> {
        self.repo.clone()
    }

    pub(crate) fn client(&self) -> Result<GatewayClient, String> {
        let endpoint = self.endpoint.clone().ok_or_else(|| "网关连接不可用".to_owned())?;
        // 拒绝在设置已切换后把旧项目的操作发送给任何网关。
        if !same_endpoint(&endpoint, &crate::app::config::gateway_client_endpoint()) {
            return Err("网关已切换，请刷新项目列表".into());
        }
        GatewayClient::new(endpoint)
    }

    fn local(&self) -> bool {
        self.endpoint.as_ref().is_some_and(GatewayEndpoint::is_loopback)
    }

    fn busy(&self) -> bool {
        self.switching_branch
            || self.files.interaction.file_tree_operation_in_progress
            || git_busy(&self.git)
    }

    pub(crate) fn cancel_file_rename(&mut self) {
        self.files.interaction.cancel_rename();
    }

    pub(crate) fn commit_form_visible(&self) -> bool {
        !self.show_editor
            && self.repo.is_some()
            && !self.switching_branch
            && !self.git.git_repository.history
            && self.git.show_git_diff_summary
            && !self.git.show_git_commit_help_modal
            && !self.git.show_git_filter_help_modal
            && !self.git.show_git_custom_diff_modal
            && !self.git.show_git_copy_modal
            && self.git.git_diff_comment_draft.is_none()
            && self.git.file_to_discard.is_none()
    }

    fn source(&self) -> Source {
        Source::Workbench(self.epoch)
    }

    pub(crate) fn editors_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut desktop_editor::components::editor::Editor> {
        self.files.previews.iter_mut().filter_map(|tab| tab.document().map(|doc| &mut doc.editor))
    }

    fn reset_checkout(&mut self, keep_worktrees: bool) {
        let fresh = GitState::new(self.git.show_git_diff_summary, self.git.show_git_diff_highlight);
        let old = std::mem::replace(&mut self.git, fresh);
        if keep_worktrees {
            self.git.git_worktree_options = old.git_worktree_options;
            self.git.git_worktree_options_project_path = old.git_worktree_options_project_path;
            self.git.selected_git_worktree_directory = old.selected_git_worktree_directory;
            self.git.selected_branch = old.selected_branch;
        }
        self.git.git_diff_scroll_id = iced::widget::Id::unique();
        self.epoch = self.epoch.wrapping_add(1);
        let show_all_files = self.files.show_all_files;
        self.files = files::State::default();
        self.files.show_all_files = show_all_files;
        self.repo = None;
        self.repo_loading = false;
    }
}

fn git_busy(git: &GitState) -> bool {
    git.git_commit_in_progress || git.git_repository.remote_busy
}

fn same_endpoint(left: &GatewayEndpoint, right: &GatewayEndpoint) -> bool {
    left.normalized_host() == right.normalized_host()
        && left.port == right.port
        && left.tenant_scope() == right.tenant_scope()
        && left.user_id() == right.user_id()
        && left.auth.as_ref().and_then(|auth| auth.skey.as_ref())
            == right.auth.as_ref().and_then(|auth| auth.skey.as_ref())
}

fn message(event: Event) -> Message {
    Message::GitWorkbench(event)
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Open => {
            open_tab(app);
            load_projects(app)
        }
        Event::RefreshProjects => load_projects(app),
        Event::ProjectsReady(request, result) => apply_projects(app, request, result),
        Event::SelectProject(id) => {
            let state = &mut app.git_workbench;
            if state.busy() || !state.projects.iter().any(|project| project.id == id) {
                return Task::none();
            }
            if state.selected.as_deref() != Some(&id) && state.files.has_unsaved() {
                return unsaved_toast(app);
            }
            state.pending_directory = None;
            select_project(state, id)
        }
        Event::SelectInlineProject(path) => select_inline_project(app, path),
        Event::Resize(event) => {
            app.git_workbench.layout.resize(event.split, event.ratio.clamp(0.08, 0.44));
            Task::none()
        }
        Event::ShowEditor(show) => {
            app.git_workbench.show_editor = show;
            app.git_workbench.files.editor_popover = None;
            Task::none()
        }
        Event::RefreshCheckout => {
            let activation = activate(app);
            Task::batch([activation, refresh_checkout(&mut app.git_workbench)])
        }
        Event::RepositoryReady(epoch, repo) if epoch == app.git_workbench.epoch => {
            let state = &mut app.git_workbench;
            state.repo_loading = false;
            state.repo = repo;
            let refresh = Task::done(state.source().message(GitMessage::RefreshGitPanelData));
            let Some(directory) = state.repo.clone() else {
                state.git.branches.clear();
                return refresh;
            };
            let branches = Task::perform(
                crate::app::message::spawn_blocking_opt(move || {
                    Some(
                        desktop_git_ui::git_panel::ops::list_branches(&directory)
                            .map_err(|error| error.to_string()),
                    )
                }),
                move |result| {
                    message(Event::BranchesReady(
                        epoch,
                        result.unwrap_or_else(|| Err("无法读取本地分支".into())),
                    ))
                },
            );
            Task::batch([refresh, branches])
        }
        Event::SwitchBranch(branch) => switch_branch(app, branch),
        Event::BranchesReady(epoch, result) if epoch == app.git_workbench.epoch => {
            match result {
                Ok(branches) => app.git_workbench.git.branches = branches,
                Err(error) => {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "读取 Git 分支失败：%s",
                        vec![error.into()],
                    );
                }
            }
            Task::none()
        }
        Event::BranchSwitched(epoch, result) if epoch == app.git_workbench.epoch => {
            let state = &mut app.git_workbench;
            state.switching_branch = false;
            match result {
                Ok(()) => {
                    state.reset_checkout(true);
                    refresh_checkout(state)
                }
                Err(error) => app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "切换 Git 分支失败：%s",
                    vec![error.into()],
                ),
            }
        }
        Event::ToggleSummary => {
            let git = &mut app.git_workbench.git;
            git.show_git_diff_summary = !git.show_git_diff_summary;
            Task::none()
        }
        Event::Git(epoch, event) if epoch == app.git_workbench.epoch => update_git(app, *event),
        Event::Files(epoch, event) if epoch == app.git_workbench.epoch => files::update(app, event),
        Event::Git(..)
        | Event::Files(..)
        | Event::RepositoryReady(..)
        | Event::BranchesReady(..)
        | Event::BranchSwitched(..) => Task::none(),
    }
}

fn open_tab(app: &mut App) {
    let nav = &mut app.navigation_tabs;
    if !nav.open_tabs.iter().any(|tab| tab.id == TAB_ID) {
        nav.open_tabs.push(desktop_navigation::components::top_bar::types::AppTab {
            id: TAB_ID.into(),
            title: crate::app::i18n::I18n::new(app.general_settings.desktop_locale)
                .__("Git 工作台")
                .into(),
            screen: Screen::GitWorkbench,
            project_path: None,
        });
    }
    nav.active_tab_id = Some(TAB_ID.into());
    nav.screen = Screen::GitWorkbench;
    app.navigation_panels.show_system_settings = false;
}

pub(super) fn open_project(app: &mut App, directory: String) -> Task<Message> {
    let state = &app.git_workbench;
    let same_project = state.switcher_project_directory() == Some(directory.as_str());
    if !same_project && state.files.has_unsaved() {
        return unsaved_toast(app);
    }
    if !same_project && state.busy() {
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "Git 工作台有正在进行的操作",
            vec![],
        );
    }
    open_tab(app);
    if same_project {
        return activate(app);
    }
    let state = &mut app.git_workbench;
    if state.endpoint.as_ref().is_some_and(|endpoint| {
        same_endpoint(endpoint, &crate::app::config::gateway_client_endpoint())
    }) && !state.projects_dirty
        && let Some(project) = state.projects.iter().find(|project| project.directory == directory)
    {
        let id = project.id.clone();
        state.pending_directory = None;
        return select_project(state, id);
    }
    state.selected = None;
    state.reset_checkout(false);
    let task = load_projects(app);
    app.git_workbench.pending_directory = Some(directory);
    task
}

pub(crate) fn activate(app: &mut App) -> Task<Message> {
    if app.git_workbench.projects_dirty || app.git_workbench.endpoint.as_ref().is_none_or(|endpoint| {
        !same_endpoint(endpoint, &crate::app::config::gateway_client_endpoint())
    }) {
        load_projects(app)
    } else {
        Task::none()
    }
}

pub(super) fn projects_changed(app: &mut App) -> Task<Message> {
    app.git_workbench.projects_dirty = true;
    if app.navigation_tabs.screen == Screen::GitWorkbench {
        load_projects(app)
    } else {
        Task::none()
    }
}

pub(super) fn remove_project(app: &mut App, directory: &str) {
    let state = &mut app.git_workbench;
    if state.endpoint.as_ref().is_none_or(|endpoint| {
        !same_endpoint(endpoint, &crate::app::config::gateway_client_endpoint())
    }) {
        return;
    }
    let was_selected = state.project_directory() == Some(directory);
    state.projects.retain(|project| project.directory != directory);
    if was_selected && !state.busy() {
        state.selected = None;
        state.reset_checkout(false);
    }
}

fn load_projects(app: &mut App) -> Task<Message> {
    let state = &mut app.git_workbench;
    let endpoint = crate::app::config::gateway_client_endpoint();
    if state.endpoint.as_ref().is_none_or(|previous| !same_endpoint(previous, &endpoint)) {
        state.selected = None;
        state.pending_directory = None;
        state.projects.clear();
        state.projects_loading = false;
        state.switching_branch = false;
        state.reset_checkout(false);
        state.endpoint = Some(endpoint.clone());
    }
    if state.busy() || state.projects_loading {
        return Task::none();
    }
    state.projects_request = state.projects_request.wrapping_add(1);
    state.projects_loading = true;
    state.projects_dirty = false;
    state.projects_error = None;
    let request = state.projects_request;
    Task::perform(
        async move {
            let client = GatewayClient::new(endpoint)?;
            Ok(client.cli_workbench_project_catalog().await?.into_iter().map(|project| Project {
                id: project.id,
                name: project.name,
                directory: project.directory,
            }).collect())
        },
        move |result| message(Event::ProjectsReady(request, result)),
    )
}

fn apply_projects(app: &mut App, request: u64, result: Result<Vec<Project>, String>) -> Task<Message> {
    let state = &mut app.git_workbench;
    if request != state.projects_request {
        return Task::none();
    }
    state.projects_loading = false;
    if state.endpoint.as_ref().is_some_and(|endpoint| {
        !same_endpoint(endpoint, &crate::app::config::gateway_client_endpoint())
    }) {
        return load_projects(app);
    }
    if state.projects_dirty && !state.busy() {
        return load_projects(app);
    }
    let mut projects = match result {
        Ok(projects) => projects,
        Err(error) => {
            state.projects_error = Some(error);
            return Task::none();
        }
    };
    if state.busy() {
        return Task::none();
    }
    if let Some(removed) = state.endpoint.as_ref().and_then(|endpoint| app.workbench_projects.removed(endpoint)) {
        projects.retain(|project| !removed.contains(&project.directory));
    }
    let previous_dir = state.project_directory().map(str::to_owned);
    state.projects = projects;
    if let Some(directory) = &state.pending_directory {
        if let Some(project) = state.projects.iter().find(|project| &project.directory == directory) {
            let id = project.id.clone();
            state.pending_directory = None;
            return select_project(state, id);
        }
        return app.show_localized_toast(
            crate::app::state::ToastKind::Error,
            "目标项目不在工作台项目列表中",
            vec![],
        );
    }
    if state.project().is_some() {
        if previous_dir.as_deref() != state.project_directory() {
            state.reset_checkout(false);
        }
        return refresh_checkout(state);
    }
    state.selected = None;
    let next = state
        .projects
        .iter()
        .find(|project| {
            Some(project.directory.as_str()) == app.navigation_tabs.project_path.as_deref()
        })
        .or_else(|| state.projects.first())
        .map(|project| project.id.clone());
    if let Some(id) = next {
        select_project(state, id)
    } else {
        state.reset_checkout(false);
        Task::none()
    }
}

fn select_project(state: &mut Workbench, id: String) -> Task<Message> {
    if state.busy()
        || state.selected.as_deref() == Some(&id)
        || !state.projects.iter().any(|project| project.id == id)
    {
        return Task::none();
    }
    state.selected = Some(id);
    state.reset_checkout(false);
    refresh_checkout(state)
}

fn select_inline_project(app: &mut App, path: String) -> Task<Message> {
    if app.navigation_tabs.screen != Screen::Project
        || git_busy(&app.git)
        || app.navigation_tabs.project_path.as_deref() == Some(&path)
        || !app.workspace_state.recent_projects.contains(&path)
    {
        return Task::none();
    }
    // 沿用项目导航，并清除旧仓库的变更选择、差异和操作弹窗。
    let mut git = GitState::new(app.git.show_git_diff_summary, app.git.show_git_diff_highlight);
    git.diff_theme = app.git.diff_theme;
    git.merge_view = app.git.merge_view;
    git.git_diff_fullscreen = app.git.git_diff_fullscreen;
    git.git_diff_half_fullscreen = app.git.git_diff_half_fullscreen;
    git.git_repository.generation = app.git.git_repository.generation.wrapping_add(1);
    app.git = git;
    let task = app.open_project_and_index(path);
    app.navigation_tabs.active_preview_path = None;
    task
}

fn refresh_checkout(state: &mut Workbench) -> Task<Message> {
    if state.busy() || state.project().is_none() {
        return Task::none();
    }
    state.files.root = state.checkout_directory().map(str::to_owned);
    let source = state.source();
    let mut tasks = Vec::new();
    if state.local() {
        if !state.repo_loading {
            state.repo_loading = true;
            let epoch = state.epoch;
            let worktree = state.git.selected_git_worktree_directory.clone();
            let project = state.project_directory().map(str::to_owned);
            tasks.push(Task::perform(
                crate::app::message::spawn_blocking_opt(move || {
                    desktop_git_ui::git_panel::ops::git_repo_path(
                        worktree.as_deref(),
                        None,
                        project.as_deref(),
                    )
                }),
                move |repo| message(Event::RepositoryReady(epoch, repo)),
            ));
        }
    } else {
        if !state.git.git_worktree_options_loading
            && let Some(path) = state.project_directory().map(str::to_owned)
        {
            state.git.git_worktree_options_loading = true;
            state.git.git_worktree_options_project_path = Some(path.clone());
            let client = state.client();
            tasks.push(Task::perform(
                async move {
                    let result = async {
                        let client = client?;
                        desktop_git_ui::message_git::shared::load_git_worktree_options(
                            &client,
                            client.endpoint().tenant_scope(),
                            path.clone(),
                        )
                        .await
                    }
                    .await;
                    (path, result)
                },
                move |(project_path, result)| {
                    source.message(GitMessage::WorktreeOptionsReady { project_path, result })
                },
            ));
        }
        tasks.push(files::refresh_remote_changes(state));
    }
    if state.files.show_all_files {
        tasks.push(files::refresh_tree(state));
    }
    Task::batch(tasks)
}

fn update_git(app: &mut App, event: GitMessage) -> Task<Message> {
    let state = &mut app.git_workbench;
    match event {
        GitMessage::SelectGitWorktree(option) => {
            if state.busy()
                || state.checkout_directory() == Some(option.directory.as_str())
                || !state
                    .git
                    .git_worktree_options
                    .iter()
                    .any(|item| item.directory == option.directory)
            {
                return Task::none();
            }
            if state.files.has_unsaved() {
                return unsaved_toast(app);
            }
            state.git.selected_git_worktree_directory = Some(option.directory);
            state.git.selected_branch = option.branch;
            state.reset_checkout(true);
            return refresh_checkout(state);
        }
        GitMessage::WorktreeOptionsReady { project_path, result } => {
            if state.project_directory() != Some(project_path.as_str()) {
                return Task::none();
            }
            state.git.git_worktree_options_loading = false;
            let options = match result {
                Ok(options) => options,
                Err(error) => {
                    return app.show_localized_toast(
                        crate::app::state::ToastKind::Error,
                        "读取 Git worktree 失败: %s",
                        vec![error.into()],
                    );
                }
            };
            let missing =
                state.git.selected_git_worktree_directory.as_ref().is_some_and(|directory| {
                    !options.iter().any(|option| &option.directory == directory)
                });
            state.git.git_worktree_options = options;
            if missing && !state.busy() {
                state.git.selected_git_worktree_directory = None;
                state.git.selected_branch = None;
                state.reset_checkout(true);
                return refresh_checkout(state);
            }
            return Task::none();
        }
        GitMessage::StageAll
        | GitMessage::CommitAll
        | GitMessage::CommitSelected
        | GitMessage::RunRemote(_)
        | GitMessage::DiscardFile(_)
        | GitMessage::DiscardHunk(..)
        | GitMessage::DiscardDiffSelection
        | GitMessage::RevertLineDelete(..)
        | GitMessage::RevertLineRestore(..) => {
            if state.busy() {
                return Task::none();
            }
            if let Err(error) = state.client() {
                return app.show_localized_toast(
                    crate::app::state::ToastKind::Error,
                    "Git 操作失败：%s",
                    vec![error.into()],
                );
            }
        }
        _ => {}
    }
    let source = state.source();
    let changes_ready = matches!(&event, GitMessage::ChangedFilesReady { repo_path, .. } if repo_path == &state.repo);
    let diff_ready = matches!(&event, GitMessage::DiffFileMetasReady { repo_path, .. } if repo_path == &state.repo);
    let scroll =
        matches!(&event, GitMessage::FocusFile(_)).then(|| state.git.git_diff_scroll_id.clone());
    let task = git::update_from(app, source, event);
    if changes_ready {
        let state = &mut app.git_workbench;
        state.files.expand_changes(&state.git.git_changed_files);
        let refresh = if !state.git.git_changed_files_loading
            && std::mem::take(&mut state.files.changes_refresh_pending)
        {
            Task::done(source.message(GitMessage::RefreshChangedFiles))
        } else {
            Task::none()
        };
        let feedback = if std::mem::take(&mut state.files.changes_refreshing) {
            app.show_localized_toast(
                crate::app::state::ToastKind::Success,
                "Git 更改已刷新",
                vec![],
            )
        } else {
            Task::none()
        };
        return Task::batch([task, refresh, feedback]);
    }
    if diff_ready
        && !app.git_workbench.git.git_diff_file_metas_loading
        && std::mem::take(&mut app.git_workbench.files.diff_refresh_pending)
    {
        return Task::batch([task, Task::done(source.message(GitMessage::RefreshDiffFileMetas))]);
    }
    if let Some(id) = scroll {
        Task::batch([
            task,
            iced::widget::operation::snap_to(id, iced::widget::scrollable::RelativeOffset::START),
        ])
    } else {
        task
    }
}

fn switch_branch(app: &mut App, branch: String) -> Task<Message> {
    let state = &mut app.git_workbench;
    if state.busy()
        || state.git.selected_branch.as_deref() == Some(&branch)
        || !state.git.branches.contains(&branch)
    {
        return Task::none();
    }
    let Some(directory) = state.repo_path() else {
        return Task::none();
    };
    if state.files.has_unsaved() {
        return unsaved_toast(app);
    }
    let client = state.client();
    state.switching_branch = true;
    let epoch = state.epoch;
    Task::perform(
        async move {
            let response = client?
                .git_command(&vw_api_git::GitCommandRequest {
                    directory,
                    args: vec![
                        "switch".into(),
                        "--no-guess".into(),
                        "--no-overwrite-ignore".into(),
                        "--".into(),
                        branch,
                    ],
                    timeout_secs: None,
                })
                .await?;
            if response.success { Ok(()) } else { Err(response.stderr.trim().to_owned()) }
        },
        move |result| message(Event::BranchSwitched(epoch, result)),
    )
}

fn unsaved_toast(app: &mut App) -> Task<Message> {
    app.show_localized_toast(crate::app::state::ToastKind::Error, "有未保存的修改", vec![])
}
