//! The desktop owns workbench views, while the gateway owns their sessions.
use std::collections::{BTreeMap, HashSet};

use gateway_interaction::PendingPermissionRequestDto;
use iced::{Task, widget::pane_grid, window};
use vw_api_types::cli_workbench::{
    CliWorkbenchDeliveryCapability, CliWorkbenchDraft, CliWorkbenchError, CliWorkbenchMessage,
    CliWorkbenchProcessState as Process, CliWorkbenchSession,
};

use super::{App, Message};

mod terminal;
mod update;
mod view;

pub(crate) use update::Event;
pub(crate) use view::view;

#[derive(Default)]
pub(crate) struct Workbench {
    pub(super) window: Option<window::Id>,
    closing: HashSet<window::Id>,
    scope: Option<update::Scope>,
    client: Option<vw_gateway_client::GatewayClient>,
    selected: Option<String>,
    preferred_directory: Option<String>,
    projects: BTreeMap<String, Project>,
    backend: Option<String>,
    native_app: Option<String>,
    terminal_available: bool,
    agents: Vec<update::Agent>,
    agent: Option<String>,
    dialog: Option<Dialog>,
    error: Option<&'static str>,
    refreshing: bool,
    discovering: bool,
    starting: bool,
    epoch: u64,
    scope_generation: u64,
    projects_epoch: u64,
    agents_epoch: u64,
    agents_loading: bool,
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
    panes: pane_grid::State<Option<String>>,
    active_pane: pane_grid::Pane,
    disconnected: bool,
    approval_error: bool,
}

struct Tab {
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
    Compose { target: Option<String>, draft: String },
    ReadDraft(SessionKey, String),
}

impl Tab {
    fn new(session: CliWorkbenchSession) -> Self {
        Self {
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
        let (panes, active_pane) = pane_grid::State::new(None);
        Self {
            name,
            directory,
            tabs: BTreeMap::new(),
            panes,
            active_pane,
            disconnected: false,
            approval_error: false,
        }
    }

    fn select(&mut self, id: &str) {
        let Some(tab) = self.tabs.get_mut(id) else { return };
        tab.hidden = false;
        let pane = self
            .panes
            .iter()
            .find_map(|(pane, session)| (session.as_deref() == Some(id)).then_some(*pane));
        if let Some(pane) = pane {
            self.active_pane = pane;
        } else if let Some(session) = self.panes.get_mut(self.active_pane) {
            *session = Some(id.to_owned());
        }
    }

    fn hide(&mut self, id: &str) {
        let Some(tab) = self.tabs.get_mut(id) else { return };
        tab.hidden = true;
        let pane = self
            .panes
            .iter()
            .find_map(|(pane, session)| (session.as_deref() == Some(id)).then_some(*pane));
        if let Some(pane) = pane {
            if self.panes.len() > 1 {
                self.close_pane(pane);
            } else if let Some(session) = self.panes.get_mut(pane) {
                *session = self.tabs.iter().find(|(_, tab)| !tab.hidden).map(|(id, _)| id.clone());
            }
        }
    }

    fn close_pane(&mut self, pane: pane_grid::Pane) {
        if let Some((_, neighbor)) = self.panes.close(pane)
            && self.active_pane == pane
        {
            self.active_pane = neighbor;
        }
    }

    fn split(&mut self, axis: pane_grid::Axis) {
        if self.panes.len() == 1
            && let Some(id) = self.split_candidate()
        {
            self.panes.split(axis, self.active_pane, Some(id.to_owned()));
        }
    }

    fn split_candidate(&self) -> Option<&str> {
        let current = self.panes.get(self.active_pane)?.as_deref()?;
        self.tabs.iter().find_map(|(id, tab)| (!tab.hidden && id != current).then_some(id.as_str()))
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
                if tab.session.state != Process::WaitingForApproval {
                    tab.approval = None;
                }
            } else if session.capabilities.available {
                self.tabs.insert(id.clone(), Tab::new(session));
                if self.panes.get(self.active_pane).is_some_and(Option::is_none) {
                    self.select(&id);
                }
            }
        }
    }
}

impl Workbench {
    pub(super) fn has_visible_terminal(&self) -> bool {
        self.window.is_some()
            && self.terminal_available
            && self.selected.as_ref().and_then(|id| self.projects.get(id)).is_some_and(|project| {
                project.panes.iter().any(|(_, id)| {
                    id.as_ref()
                        .and_then(|id| project.tabs.get(id))
                        .is_some_and(|tab| tab.session.capabilities.terminal)
                })
            })
    }

    pub(super) fn owns_window(&self, id: window::Id) -> bool {
        self.window == Some(id) || self.closing.contains(&id)
    }

    pub(super) fn attach(&mut self, id: window::Id) -> window::Id {
        *self.window.get_or_insert(id)
    }

    fn detach(&mut self, id: window::Id) -> bool {
        if self.window != Some(id) {
            return false;
        }
        self.window = None;
        self.dialog = None;
        self.refreshing = false;
        self.discovering = false;
        self.agents_loading = false;
        self.starting = false;
        for tab in self.projects.values_mut().flat_map(|project| project.tabs.values_mut()) {
            tab.stopping = false;
            tab.approving = None;
            tab.resolving_draft = None;
            tab.terminal.detach();
        }
        true
    }

    pub(super) fn request_close(&mut self, id: window::Id) -> bool {
        if self.detach(id) {
            self.closing.insert(id);
            true
        } else {
            false
        }
    }

    pub(super) fn closed(&mut self, id: window::Id) {
        self.detach(id);
        self.closing.remove(&id);
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
