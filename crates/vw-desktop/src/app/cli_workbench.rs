//! The desktop owns workbench views, while the gateway owns their sessions.
use std::collections::{BTreeMap, HashSet};

use gateway_interaction::PendingPermissionRequestDto;
use iced::{Task, widget::pane_grid, window};
use desktop_terminal::panes::TerminalPanes;
use vw_api_types::cli_workbench::{
    CliWorkbenchDeliveryCapability, CliWorkbenchDraft, CliWorkbenchError, CliWorkbenchMessage, CliWorkbenchNativeApp,
    CliWorkbenchProcessState as Process, CliWorkbenchSession, CliWorkbenchTarget, SshSessionSurface,
};

use super::{App, Message};

mod ssh;
mod terminal;
mod update;
mod view;
mod workspace;

pub(crate) use update::{Action, Event};
pub(crate) use view::{terminal_overlay, terminal_ssh_view, view};

const CODEX_APP_ID: &str = "codex";
const APP_TERMINAL_SIDEBAR_COLLAPSED: &str = "app_terminal_sidebar_collapsed";

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum Surface {
    #[default]
    Workbench,
    AppTerminal,
    Terminal,
}

impl Surface {
    pub(crate) fn from_screen(screen: super::Screen) -> Option<Self> {
        match screen {
            super::Screen::CliWorkbench => Some(Self::Workbench),
            super::Screen::AppTerminal => Some(Self::AppTerminal),
            _ => None,
        }
    }

    pub(crate) fn state(self, app: &App) -> &Workbench {
        match self {
            Self::Workbench => &app.cli_workbench,
            Self::AppTerminal => &app.app_terminal,
            Self::Terminal => &app.terminal_ssh,
        }
    }

    pub(crate) fn message(self, event: Event) -> Message {
        Message::CliWorkbench(self.event(event))
    }

    pub(super) fn clipboard_shortcut(
        self,
        app: &App,
        action: desktop_terminal::message::ContentAction,
    ) -> Option<(iced::widget::Id, Message)> {
        use desktop_terminal::message::{Message as Panel, TerminalMessage};

        let state = self.state(app);
        if state.dialog.is_some() {
            return None;
        }
        let (project_id, project) = state.selected_project()?;
        let tab = project.active_tab()?;
        if !project.panes.is_visible_tab(tab.view_id) {
            return None;
        }
        let widget_id = tab.terminal.view.as_ref()?.widget_id();
        let event = Panel::Terminal(TerminalMessage::ClipboardShortcut(widget_id.clone(), action));
        let message = self.message(Event::Window(
            app.main_window_id?,
            Action::Panel(project_id.to_owned(), project.active_layout, event),
        ));
        Some((widget_id, message))
    }

    fn event(self, event: Event) -> Event {
        match self {
            Self::Workbench => event,
            Self::AppTerminal => Event::AppTerminal(Box::new(event)),
            Self::Terminal => Event::Terminal(Box::new(event)),
        }
    }

    fn route_message(self, message: Message) -> Message {
        match message {
            Message::CliWorkbench(event) => self.message(event),
            message => message,
        }
    }

    fn ssh_surface(self) -> SshSessionSurface {
        match self {
            Self::Workbench => SshSessionSurface::Workbench,
            Self::AppTerminal => SshSessionSurface::AppTerminal,
            Self::Terminal => SshSessionSurface::Terminal,
        }
    }

    fn includes_session(self, session: &CliWorkbenchSession) -> bool {
        match &session.target {
            CliWorkbenchTarget::Ssh { .. } => {
                self.ssh_surface() == session.ssh_surface.unwrap_or(SshSessionSurface::AppTerminal)
            }
            CliWorkbenchTarget::Acp { .. } => self == Self::Workbench,
            _ => self != Self::Terminal,
        }
    }
}

pub(crate) fn terminal_panel_visible(app: &App) -> bool {
    app.navigation_tabs.screen == super::Screen::Project
        && !app.navigation_panels.show_system_settings
        && app.terminal.is_visible
}

#[derive(Default)]
pub(crate) struct Workbench {
    surface: Surface,
    scope: Option<update::Scope>,
    client: Option<vw_gateway_client::GatewayClient>,
    selected: Option<String>,
    preferred_directory: Option<String>,
    terminal_directory: Option<String>,
    projects: BTreeMap<String, Project>,
    workspace: workspace::Store,
    legacy_projects: bool,
    legacy_poll_cursor: usize,
    backend: Option<String>,
    native_app: Option<String>,
    native_apps: Vec<CliWorkbenchNativeApp>,
    terminal_available: bool,
    ssh: ssh::State,
    agents: Vec<update::Agent>,
    group_agents_by_project: bool,
    sidebar_layout: SidebarLayout,
    dialog: Option<Dialog>,
    error: Option<&'static str>,
    refreshing: bool,
    details_refreshing: bool,
    discovering: bool,
    starting: bool,
    default_shell_pending: bool,
    epoch: u64,
    scope_generation: u64,
    projects_epoch: u64,
    agents_epoch: u64,
    agents_loading: bool,
}

struct SidebarLayout {
    workspace: pane_grid::State<bool>,
    terminal_pane: pane_grid::Pane,
    sections: pane_grid::State<bool>,
    minimum_width_ratio: f32,
}

impl SidebarLayout {
    const DEFAULT_WIDTH_RATIO: f32 = 0.20;
    const MIN_WIDTH_RATIO: f32 = 0.10;
    const MAX_WIDTH_RATIO: f32 = 0.65;

    fn collapsed(&self) -> bool {
        self.workspace.maximized() == Some(self.terminal_pane)
    }

    fn set_collapsed(&mut self, collapsed: bool) {
        if collapsed {
            self.workspace.maximize(self.terminal_pane);
        } else {
            self.workspace.restore();
        }
    }

    fn resize_width(&mut self, event: pane_grid::ResizeEvent) {
        self.workspace.resize(
            event.split,
            event.ratio.clamp(self.minimum_width_ratio, Self::MAX_WIDTH_RATIO),
        );
    }
}

impl Default for SidebarLayout {
    fn default() -> Self {
        Self::new(Self::DEFAULT_WIDTH_RATIO, Self::MIN_WIDTH_RATIO)
    }
}

impl SidebarLayout {
    fn new(width_ratio: f32, minimum_width_ratio: f32) -> Self {
        let (mut workspace, sidebar) = pane_grid::State::new(false);
        let (terminal_pane, split) = workspace.split(pane_grid::Axis::Vertical, sidebar, true)
            .expect("the sidebar pane exists");
        workspace.resize(split, width_ratio);

        let (mut sections, projects) = pane_grid::State::new(false);
        sections.split(pane_grid::Axis::Horizontal, projects, true)
            .expect("the projects pane exists");

        Self { workspace, terminal_pane, sections, minimum_width_ratio }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionKey {
    project: String,
    session: String,
}

struct Project {
    name: String,
    directory: String,
    tabs: BTreeMap<String, Tab>,
    panes: TerminalPanes,
    parked_layouts: BTreeMap<u64, TerminalPanes>,
    active_layout: u64,
    next_layout: u64,
    layout_names: BTreeMap<u64, String>,
    layout_loaded: bool,
    layout_menu: Option<(u64, iced::Point)>,
    tab_menu: Option<(u64, f32, f32)>,
    content_menu: Option<(u64, iced::Point)>,
    disconnected: bool,
    approval_error: bool,
}

struct Tab {
    view_id: u64,
    session: CliWorkbenchSession,
    name: Option<String>,
    hidden: bool,
    stopping: bool,
    approval: Option<PendingPermissionRequestDto>,
    approving: Option<String>,
    turns: Vec<Turn>,
    action_error: Option<&'static str>,
    terminal: terminal::Attachment,
    drafts: Vec<CliWorkbenchDraft>,
    resolving_draft: Option<String>,
}

struct Turn {
    request: String,
    prompt: Option<String>,
    snapshot: Option<CliWorkbenchMessage>,
    error: Option<&'static str>,
    recover: bool,
    submitting: Option<window::Id>,
}

enum Dialog {
    Rename(SessionKey, String),
    RenameLayout(String, u64, String),
    Compose { target: Option<String>, draft: String },
    ReadDraft(SessionKey, String),
    Ssh(ssh::Panel),
}

impl Tab {
    fn new(session: CliWorkbenchSession) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_VIEW_ID: AtomicU64 = AtomicU64::new(1);
        let mut tab = Self {
            view_id: NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed),
            session,
            name: None,
            hidden: false,
            stopping: false,
            approval: None,
            approving: None,
            turns: Vec::new(),
            action_error: None,
            terminal: terminal::Attachment::default(),
            drafts: Vec::new(),
            resolving_draft: None,
        };
        tab.discover_turns();
        tab
    }

    fn discover_turns(&mut self) {
        for request in &self.session.recent_request_ids {
            if self.turns.iter().any(|turn| &turn.request == request) {
                continue;
            }
            self.turns.push(Turn {
                request: request.clone(),
                prompt: None,
                snapshot: None,
                error: None,
                recover: true,
                submitting: None,
            });
        }
    }

    fn is_agent(&self) -> bool {
        matches!(
            self.session.target,
            CliWorkbenchTarget::Acp { .. } | CliWorkbenchTarget::NativeCli { .. }
        )
    }

    fn can_send(&self) -> bool {
        !self.stopping
            && match self.session.capabilities.delivery {
                CliWorkbenchDeliveryCapability::Structured => {
                    self.session.state == Process::Ready
                        && self.turns.iter().all(|turn| turn.prompt.is_none() || !turn.recover)
                }
                CliWorkbenchDeliveryCapability::Draft => self.session.state == Process::Running,
                CliWorkbenchDeliveryCapability::Unavailable => false,
            }
    }

    fn can_stop(&self) -> bool {
        !self.stopping
            && !matches!(
                self.session.state,
                Process::Stopping | Process::Exited | Process::Disconnected
            )
    }

    fn begin_stop(&mut self) -> bool {
        if !self.can_stop() {
            return false;
        }
        self.stopping = true;
        self.action_error = None;
        true
    }

    fn finish_stop(&mut self, result: Result<CliWorkbenchSession, CliWorkbenchError>) {
        self.stopping = false;
        match result {
            Ok(session)
                if session.id == self.session.id
                    && session.project_id == self.session.project_id =>
            {
                self.session = session;
                self.approval = None;
            }
            Ok(_) => self.action_error = Some("网关返回无效的工作台会话"),
            Err(error) => self.action_error = Some(update::error_label(error.code)),
        }
    }
}

impl Project {
    fn new(name: String, directory: String) -> Self {
        Self {
            name,
            directory,
            tabs: BTreeMap::new(),
            panes: Default::default(),
            parked_layouts: BTreeMap::new(),
            active_layout: 1,
            next_layout: 2,
            layout_names: BTreeMap::new(),
            layout_loaded: false,
            layout_menu: None,
            tab_menu: None,
            content_menu: None,
            disconnected: false,
            approval_error: false,
        }
    }

    fn tab_for_view(&self, view_id: u64) -> Option<&Tab> {
        self.tabs.values().find(|tab| tab.view_id == view_id)
    }

    fn active_tab(&self) -> Option<&Tab> {
        self.tab_for_view(self.panes.grid.get(self.panes.focused)?.active_id?)
    }

    fn layout_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.parked_layouts.range(..self.active_layout).map(|(&id, _)| id)
            .chain(std::iter::once(self.active_layout))
            .chain(self.parked_layouts.range(self.active_layout..).map(|(&id, _)| id))
    }

    fn new_layout(&mut self) {
        let id = self.next_layout;
        let Some(next) = id.checked_add(1) else { return };
        self.next_layout = next;
        self.parked_layouts.insert(id, TerminalPanes::default());
        self.select_layout(id);
    }

    fn select_layout(&mut self, id: u64) -> bool {
        self.layout_menu = None;
        if self.active_layout == id {
            return true;
        }
        let Some(next) = self.parked_layouts.remove(&id) else { return false };
        self.panes.split_menu = None;
        let old = std::mem::replace(&mut self.panes, next);
        self.parked_layouts.insert(self.active_layout, old);
        self.active_layout = id;
        self.tab_menu = None;
        self.content_menu = None;
        true
    }

    fn close_layout(&mut self, id: u64) -> bool {
        self.layout_menu = None;
        if id == self.active_layout {
            let Some((&next, _)) = self.parked_layouts.range(id..).next()
                .or_else(|| self.parked_layouts.last_key_value()) else { return false };
            self.select_layout(next);
        }
        let Some(closed) = self.parked_layouts.remove(&id) else { return false };
        self.layout_names.remove(&id);
        let view_ids: HashSet<_> = closed.grid.iter()
            .flat_map(|(_, pane)| pane.tabs.iter().copied())
            .collect();
        for tab in self.tabs.values_mut().filter(|tab| view_ids.contains(&tab.view_id)) {
            tab.hidden = true;
        }
        true
    }

    fn layout_with_tab(&self, tab: u64) -> Option<u64> {
        let contains = |panes: &TerminalPanes| panes.grid.iter().any(|(_, pane)| pane.tabs.contains(&tab));
        if contains(&self.panes) {
            return Some(self.active_layout);
        }
        self.parked_layouts.iter().find_map(|(&id, panes)| contains(panes).then_some(id))
    }

    fn layout_mut(&mut self, id: u64) -> Option<&mut TerminalPanes> {
        if self.active_layout == id { Some(&mut self.panes) } else { self.parked_layouts.get_mut(&id) }
    }

    fn select(&mut self, id: &str) {
        let Some(tab) = self.tabs.get_mut(id) else { return };
        tab.hidden = false;
        let view_id = tab.view_id;
        if let Some(layout) = self.layout_with_tab(view_id) {
            self.select_layout(layout);
            self.panes.select(view_id);
        } else {
            self.panes.add(self.panes.focused, view_id);
        }
    }

    fn hide(&mut self, id: &str) {
        let Some(tab) = self.tabs.get_mut(id) else { return };
        tab.hidden = true;
        if self.tab_menu.is_some_and(|(view_id, ..)| view_id == tab.view_id) {
            self.tab_menu = None;
        }
        if self.content_menu.is_some_and(|(view_id, _)| view_id == tab.view_id) {
            self.content_menu = None;
        }
        let view_id = tab.view_id;
        if let Some(layout) = self.layout_with_tab(view_id).and_then(|id| self.layout_mut(id)) {
            layout.remove(view_id);
        }
    }

    fn place_started(
        &mut self,
        session: CliWorkbenchSession,
        layout: u64,
        pane: iced::widget::pane_grid::Pane,
        direction: Option<iced::widget::pane_grid::Direction>,
    ) -> bool {
        let session_id = session.id.clone();
        let view_id = self.tabs.entry(session_id.clone()).or_insert_with(|| Tab::new(session)).view_id;
        if self.active_layout != layout && !self.parked_layouts.contains_key(&layout) {
            self.hide(&session_id);
            return false;
        }
        if let Some(owner) = self.layout_with_tab(view_id)
            && owner != layout
            && let Some(panes) = self.layout_mut(owner)
        {
            panes.remove(view_id);
        }
        let Some(tab) = self.tabs.get_mut(&session_id) else { return false };
        let terminal = tab.session.capabilities.terminal;
        if terminal && tab.terminal.view.is_none() {
            tab.terminal.fresh = true;
        }
        tab.hidden = false;
        let visible = self.active_layout == layout;
        let Some(panes) = self.layout_mut(layout) else { return false };
        let pane = if panes.grid.get(pane).is_some() { pane } else { panes.focused };
        let destination = direction.and_then(|direction| {
            let has_other_tab = panes.grid.get(pane)?
                .tabs.iter().any(|id| *id != view_id);
            if !has_other_tab {
                return None;
            }
            panes.grid.restore();
            panes.split(pane, direction)
        }).unwrap_or(pane);
        panes.place(destination, view_id);
        terminal && visible
    }

    fn merge_sessions(&mut self, sessions: Vec<CliWorkbenchSession>) {
        let confirmed: HashSet<_> = sessions.iter().map(|session| session.id.clone()).collect();
        for (id, tab) in &mut self.tabs {
            if !confirmed.contains(id) {
                tab.session.state = Process::Disconnected;
                tab.approval = None;
            }
        }
        for session in sessions {
            let id = session.id.clone();
            if let Some(tab) = self.tabs.get_mut(&id) {
                tab.session = session;
                tab.discover_turns();
                if tab.session.state != Process::WaitingForApproval {
                    tab.approval = None;
                }
            } else if session.capabilities.available {
                let tab = Tab::new(session);
                let view_id = tab.view_id;
                self.tabs.insert(id, tab);
                self.add_discovered_tab(view_id);
            }
        }
    }

    fn add_discovered_tab(&mut self, tab: u64) {
        let first_layout = self.layout_ids().next().expect("project has an active layout");
        let panes = self.layout_mut(first_layout).expect("first layout exists");
        let focused = panes.focused;
        let active = panes.grid.get(focused).and_then(|pane| pane.active_id);
        panes.add(focused, tab);
        if let Some(active) = active {
            panes.select(active);
        }
    }
}

impl Workbench {
    pub(crate) fn terminal_ssh() -> Self {
        Self { surface: Surface::Terminal, ..Self::default() }
    }

    pub(crate) fn terminal_ssh_visible(&self, directory: Option<&str>, group: u64) -> bool {
        directory.is_some()
            && self.terminal_directory.as_deref() == directory
            && self.selected_project().is_some_and(|(_, project)| {
                project.active_layout == group
                    && project.panes.grid.iter().any(|(_, pane)| !pane.tabs.is_empty())
            })
    }

    pub(crate) fn terminal_needs_refresh(&self, directory: Option<&str>) -> bool {
        directory.is_some()
            && (self.terminal_directory.as_deref() != directory
                || self.dialog.is_some()
                || self.selected_project().is_some_and(|(_, project)| {
                    project.tabs.values().any(|tab| !tab.hidden)
                }))
    }

    pub(crate) fn sync_terminal_group(
        &mut self,
        directory: Option<&str>,
        terminal: &super::TerminalState,
        closed: Option<u64>,
    ) {
        let Some(directory) = directory else { return };
        if self.terminal_directory.as_deref() != Some(directory) {
            return;
        }
        let project_id = self.selected.as_deref().or_else(|| {
            self.projects.iter().find_map(|(id, project)| {
                (project.directory == directory).then_some(id.as_str())
            })
        });
        let Some(project) = project_id.and_then(|id| self.projects.get_mut(id)) else {
            return;
        };
        let group = terminal.active_group();
        if project.active_layout != group {
            if project.active_layout == 1
                && project.parked_layouts.is_empty()
                && closed.is_none()
                && terminal.groups().all(|(id, _)| id != 1)
            {
                // Legacy SSH sessions were all saved under layout 1.
                project.active_layout = group;
            } else {
                project.parked_layouts.entry(group).or_default();
                project.select_layout(group);
            }
        }
        if let Some(closed) = closed.filter(|id| terminal.groups().all(|(group, _)| group != *id))
            && project.close_layout(closed)
        {
            self.checkpoint_layout();
        }
    }

    fn selected_project(&self) -> Option<(&str, &Project)> {
        let id = self.selected.as_deref()?;
        Some((id, self.projects.get(id)?))
    }

    pub(crate) fn has_working_agent(&self) -> bool {
        self.projects.values().any(|project| {
            project.tabs.values().any(|tab| {
                view::AgentStatus::for_tab(project, tab) == Some(view::AgentStatus::Working)
            })
        })
    }

    pub(crate) fn app_terminal() -> Self {
        Self {
            surface: Surface::AppTerminal,
            sidebar_layout: SidebarLayout::new(SidebarLayout::DEFAULT_WIDTH_RATIO / 2.0, 0.07),
            ..Self::default()
        }
    }

    pub(crate) fn restore_app_terminal_preferences(&mut self, cfg: &serde_json::Value) {
        if let Some(collapsed) = cfg.get(APP_TERMINAL_SIDEBAR_COLLAPSED).and_then(|value| value.as_bool())
        {
            self.sidebar_layout.set_collapsed(collapsed);
        }
    }

    pub(super) fn has_visible_terminal(&self) -> bool {
        self.selected
            .as_ref()
            .and_then(|id| self.projects.get(id))
            .is_some_and(|project| {
                project.panes.grid.iter().any(|(_, pane)| {
                    pane.active_id.and_then(|id| project.tab_for_view(id)).is_some_and(|tab| {
                        tab.session.capabilities.terminal
                            && self.pty_available(&tab.session.target)
                            && project.panes.is_visible_tab(tab.view_id)
                    })
                })
            })
    }

    fn pty_available(&self, target: &CliWorkbenchTarget) -> bool {
        if matches!(target, CliWorkbenchTarget::Ssh { .. }) {
            self.ssh.catalog.is_some()
        } else {
            self.terminal_available
        }
    }

    fn tab_mut(&mut self, key: &SessionKey) -> Option<&mut Tab> {
        self.projects.get_mut(&key.project)?.tabs.get_mut(&key.session)
    }

    fn locate(&mut self, key: &SessionKey) -> bool {
        let Some(project) = self.projects.get_mut(&key.project) else { return false };
        if !project.tabs.contains_key(&key.session) {
            return false;
        }
        self.selected = Some(key.project.clone());
        project.select(&key.session);
        true
    }

    fn apply_turn(
        &mut self,
        key: &SessionKey,
        request: &str,
        result: Result<CliWorkbenchMessage, CliWorkbenchError>,
        submitted: bool,
    ) {
        let Some(tab) = self.tab_mut(key) else { return };
        let Some(turn) = tab.turns.iter_mut().find(|turn| turn.request == request) else { return };
        if submitted {
            turn.submitting = None;
        }
        match result {
            Ok(snapshot)
                if snapshot.id.target_session_id == key.session
                    && snapshot.id.request_id == request
                    && turn
                        .snapshot
                        .as_ref()
                        .is_none_or(|old| snapshot.revision >= old.revision) =>
            {
                turn.recover = !snapshot.is_settled();
                turn.error = None;
                turn.snapshot = Some(snapshot);
            }
            Ok(_) => {}
            Err(error) => {
                use vw_api_types::cli_workbench::CliWorkbenchErrorCode as Code;
                turn.error = Some(update::error_label(error.code));
                turn.recover = if submitted {
                    matches!(error.code, Code::Disconnected | Code::GatewayFailure)
                } else {
                    !matches!(error.code, Code::NotFound | Code::Closed)
                };
            }
        }
    }
}

pub(super) fn update(app: &mut App, event: Event) -> Task<Message> {
    update::update(app, event)
}

pub(super) fn refresh_projects(app: &mut App) -> Task<Message> {
    let (Some(surface), Some(id)) = (Surface::from_screen(app.navigation_tabs.screen), app.main_window_id) else {
        return Task::none();
    };
    update(app, surface.event(Event::Window(id, Action::Refresh)))
}

pub(super) fn remove_project(app: &mut App, directory: &str) {
    for state in [&mut app.cli_workbench, &mut app.app_terminal, &mut app.terminal_ssh] {
        if !update::current_scope(state) {
            continue;
        }
        state.checkpoint_layout();
        state.projects.retain(|_, project| project.directory != directory);
        if state.selected.as_ref().is_some_and(|id| !state.projects.contains_key(id)) {
            state.selected = None;
            state.default_shell_pending = false;
            state.dialog = None;
        }
    }
}

#[cfg(test)]
#[path = "cli_workbench_tests.rs"]
mod tests;
