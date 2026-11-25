//! Desktop state for the gateway's SSH connection manager.
use std::collections::BTreeSet;

use iced::{Task, window};
use vw_api_types::cli_workbench::{
    CliWorkbenchErrorCode as Code, CliWorkbenchTarget, SshConnection, SshConnectionChange,
    SshConnectionFields, SshConnections,
};

use super::{Dialog, Message, Workbench, update as workbench_update};

mod view;
pub(super) use view::view;

#[derive(Default)]
pub(super) struct State {
    pub(super) catalog: Option<SshConnections>,
    pending: bool,
    error: Option<Code>,
}

impl State {
    pub(super) fn connection(&self, id: &str) -> Option<&SshConnection> {
        self.catalog.as_ref()?.saved.iter().find(|entry| entry.id == id)
    }
}

pub(super) enum Panel {
    Saved,
    Import(BTreeSet<String>),
    Edit(Form),
    Details(String),
    Delete(String),
}

#[derive(Default)]
pub(super) struct Form {
    id: Option<String>,
    from_config: bool,
    name: String,
    host: String,
    port: String,
    identity: Option<String>,
}

impl Form {
    fn from_connection(connection: &SshConnection) -> Self {
        let fields = &connection.fields;
        Self {
            id: Some(connection.id.clone()),
            from_config: connection.from_config,
            name: fields.name.clone(),
            host: fields.host.clone(),
            port: fields.port.map_or_else(String::new, |port| port.to_string()),
            identity: fields.identity_file.clone(),
        }
    }

    fn fields(&self) -> Option<SshConnectionFields> {
        if self.name.trim().is_empty() || self.host.trim().is_empty() {
            return None;
        }
        let port = if self.port.trim().is_empty() {
            None
        } else {
            Some(self.port.trim().parse::<u16>().ok().filter(|port| *port > 0)?)
        };
        let identity_file = self.identity.as_deref().map(str::trim).map(str::to_owned);
        if identity_file.as_deref() == Some("") {
            return None;
        }
        Some(SshConnectionFields {
            name: self.name.trim().to_owned(),
            host: self.host.trim().to_owned(),
            port,
            identity_file,
        })
    }
}

#[derive(Clone)]
pub(crate) enum Action {
    Open,
    Back,
    Refresh,
    ShowImport,
    ToggleAlias(String, bool),
    Import,
    AddManual,
    Details(String),
    Edit(String),
    Name(String),
    Host(String),
    Port(String),
    UseIdentity(bool),
    Identity(String),
    Save,
    ConfirmDelete(String),
    Delete(String),
    Connect(String),
    Loaded { changed: bool, result: Result<SshConnections, Code> },
}

pub(super) fn show(state: &mut Workbench) {
    state.dialog = Some(Dialog::Ssh(Panel::Saved));
}

pub(super) fn load(state: &mut Workbench, id: window::Id) -> Task<Message> {
    request(state, id, None)
}

fn request(
    state: &mut Workbench,
    id: window::Id,
    change: Option<SshConnectionChange>,
) -> Task<Message> {
    if state.ssh.pending {
        return Task::none();
    }
    let Some(client) = workbench_update::client(state) else {
        state.ssh.error = Some(Code::Disconnected);
        return Task::none();
    };
    state.ssh.pending = true;
    state.ssh.error = None;
    let generation = state.scope_generation;
    let changed = change.is_some();
    Task::perform(
        async move {
            match change {
                Some(change) => client.cli_workbench_change_ssh_connection(&change).await,
                None => client.cli_workbench_ssh_connections().await,
            }
            .map_err(|error| error.code)
        },
        move |result| {
            workbench_update::response(
                id,
                generation,
                workbench_update::Action::Ssh(Action::Loaded { changed, result }),
            )
        },
    )
}

pub(super) fn can_connect(state: &Workbench) -> bool {
    !state.starting
        && !state.discovering
        && state
            .selected
            .as_ref()
            .and_then(|id| state.projects.get(id))
            .is_some_and(|project| !project.disconnected)
}

pub(super) fn update(state: &mut Workbench, id: window::Id, action: Action) -> Task<Message> {
    match action {
        Action::Loaded { changed, result } => {
            state.ssh.pending = false;
            match result {
                Ok(catalog) => {
                    state.ssh.catalog = Some(catalog);
                    if changed && matches!(state.dialog, Some(Dialog::Ssh(_))) {
                        show(state);
                    }
                    if let Some(Dialog::Ssh(Panel::Import(selected))) = &mut state.dialog
                        && let Some(catalog) = &state.ssh.catalog
                    {
                        selected.retain(|alias| catalog.aliases.contains(alias));
                    }
                }
                Err(code) => state.ssh.error = Some(code),
            }
            return Task::none();
        }
        Action::Open => {
            show(state);
            return load(state, id);
        }
        Action::Refresh => return load(state, id),
        _ => {}
    }

    if state.ssh.pending {
        return Task::none();
    }
    let Some(Dialog::Ssh(panel)) = &mut state.dialog else { return Task::none() };
    state.ssh.error = None;
    let change = match action {
        Action::Back => {
            *panel = Panel::Saved;
            None
        }
        Action::ShowImport => {
            *panel = Panel::Import(BTreeSet::new());
            None
        }
        Action::ToggleAlias(alias, included) => {
            if let Panel::Import(selected) = panel {
                if included {
                    selected.insert(alias);
                } else {
                    selected.remove(&alias);
                }
            }
            None
        }
        Action::Import => {
            let Panel::Import(selected) = panel else { return Task::none() };
            Some(SshConnectionChange::Import { aliases: selected.iter().cloned().collect() })
        }
        Action::AddManual => {
            *panel = Panel::Edit(Form::default());
            None
        }
        Action::Details(id) => {
            *panel = Panel::Details(id);
            None
        }
        Action::Edit(id) => {
            if let Some(connection) = state.ssh.connection(&id) {
                *panel = Panel::Edit(Form::from_connection(connection));
            }
            None
        }
        Action::Save => {
            let Panel::Edit(form) = panel else { return Task::none() };
            form.fields().map(|fields| SshConnectionChange::Save { id: form.id.clone(), fields })
        }
        Action::ConfirmDelete(id) => {
            *panel = Panel::Delete(id);
            None
        }
        Action::Delete(id) => Some(SshConnectionChange::Delete { id }),
        Action::Connect(connection_id) => {
            if !can_connect(state) || state.ssh.connection(&connection_id).is_none() {
                return Task::none();
            }
            let Some(project) = state.selected.as_ref().and_then(|id| state.projects.get(id))
            else {
                return Task::none();
            };
            let pane = project.panes.focused;
            state.dialog = None;
            return workbench_update::start(
                state,
                id,
                CliWorkbenchTarget::Ssh { connection_id },
                pane,
                None,
            );
        }
        action => {
            if let Panel::Edit(form) = panel {
                match action {
                    Action::Name(value) => form.name = value,
                    Action::Host(value) => form.host = value,
                    Action::Port(value) => form.port = value,
                    Action::UseIdentity(true) => form.identity = Some(String::new()),
                    Action::UseIdentity(false) => form.identity = None,
                    Action::Identity(value) => form.identity = Some(value),
                    _ => {}
                }
            }
            None
        }
    };
    change.map_or_else(Task::none, |change| request(state, id, Some(change)))
}
