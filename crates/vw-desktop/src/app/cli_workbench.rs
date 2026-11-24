//! The desktop owns workbench views, while the gateway owns their sessions.
use std::collections::{BTreeMap, HashSet};

use gateway_interaction::PendingPermissionRequestDto;
use iced::{Task, widget::pane_grid, window};
use desktop_terminal::panes::TerminalPanes;
use vw_api_types::cli_workbench::{
    CliWorkbenchDeliveryCapability, CliWorkbenchDraft, CliWorkbenchError, CliWorkbenchMessage, CliWorkbenchNativeApp,
    CliWorkbenchProcessState as Process, CliWorkbenchSession,
};

use super::{App, Message};

mod terminal;
mod update;
mod view;

pub(crate) use update::Event;
pub(crate) use view::view;

const CODEX_APP_ID: &str = "codex";

#[derive(Default)]
pub(crate) struct Workbench {
    scope: Option<update::Scope>,
    client: Option<vw_gateway_client::GatewayClient>,
    selected: Option<String>,
    preferred_directory: Option<String>,
    projects: BTreeMap<String, Project>,
    legacy_projects: bool,
    legacy_poll_cursor: usize,
    backend: Option<String>,
    native_app: Option<String>,
    native_apps: Vec<CliWorkbenchNativeApp>,
    terminal_available: bool,
    agents: Vec<update::Agent>,
    group_agents_by_project: bool,
    sidebar_layout: SidebarLayout,
    dialog: Option<Dialog>,
    error: Option<&'static str>,
    refreshing: bool,
    details_refreshing: bool,
    discovering: bool,
    starting: bool,
    epoch: u64,
    scope_generation: u64,
    projects_epoch: u64,
    agents_epoch: u64,
    agents_loading: bool,
}

struct SidebarLayout {
    workspace: pane_grid::State<bool>,
    sections: pane_grid::State<bool>,
}

impl Default for SidebarLayout {
    fn default() -> Self {
        let (mut workspace, sidebar) = pane_grid::State::new(false);
        let (_, split) = workspace.split(pane_grid::Axis::Vertical, sidebar, true)
            .expect("the sidebar pane exists");
        workspace.resize(split, 0.34);

        let (mut sections, projects) = pane_grid::State::new(false);
        sections.split(pane_grid::Axis::Horizontal, projects, true)
            .expect("the projects pane exists");

        Self { workspace, sections }
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
    prompt: String,
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
}

impl Tab {
    fn new(session: CliWorkbenchSession) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_VIEW_ID: AtomicU64 = AtomicU64::new(1);
        Self {
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
        }
    }

    fn can_send(&self) -> bool {
        !self.stopping
            && match self.session.capabilities.delivery {
                CliWorkbenchDeliveryCapability::Structured => {
                    self.session.state == Process::Ready
                        && self.turns.iter().all(|turn| !turn.recover)
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
        self.next_layout += 1;
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
        let first_layout = self.parked_layouts.first_key_value()
            .map_or(self.active_layout, |(&id, _)| id.min(self.active_layout));
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
                if tab.session.state != Process::WaitingForApproval {
                    tab.approval = None;
                }
            } else if session.capabilities.available {
                self.tabs.insert(id.clone(), Tab::new(session));
                let view_id = self.tabs[&id].view_id;
                if let Some(panes) = self.layout_mut(first_layout) {
                    let focused = panes.focused;
                    let active = panes.grid.get(focused).and_then(|pane| pane.active_id);
                    panes.add(focused, view_id);
                    if let Some(active) = active {
                        panes.select(active);
                    }
                }
            }
        }
    }
}

impl Workbench {
    pub(super) fn has_visible_terminal(&self) -> bool {
        self.terminal_available
            && self.selected.as_ref().and_then(|id| self.projects.get(id)).is_some_and(|project| {
                project.panes.grid.iter().any(|(_, pane)| {
                    pane.active_id.and_then(|id| project.tab_for_view(id)).is_some_and(|tab| {
                        tab.session.capabilities.terminal && project.panes.is_visible_tab(tab.view_id)
                    })
                })
            })
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

#[cfg(test)]
#[path = "cli_workbench_tests.rs"]
mod tests;
