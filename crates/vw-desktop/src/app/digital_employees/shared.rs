use iced::Task;

use crate::app::{App, Message};

use super::{
    Event,
    shared_api::{self, Directory},
};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Kind {
    #[default]
    All,
    Manager,
    Worker,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Runtime {
    #[default]
    All,
    OnDuty,
    Abnormal,
}

impl Runtime {
    pub fn key(self) -> Option<&'static str> {
        match self {
            Self::All => None,
            Self::OnDuty => Some("on_duty"),
            Self::Abnormal => Some("abnormal"),
        }
    }
}

impl std::fmt::Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::All => "全部",
            Self::OnDuty => "在岗",
            Self::Abnormal => "异常",
        })
    }
}

#[derive(Clone)]
pub(crate) enum ListEvent {
    Refresh,
    Loaded(u64, Result<std::sync::Arc<Directory>, String>),
    Search(String),
    Kind(Kind),
    Archived(bool),
    Department(Option<String>),
    Runtime(Runtime),
    Table(bool),
    Page { managers: bool, index: usize },
    PageSize(usize),
}

pub(super) struct Shared {
    pub directory: Option<std::sync::Arc<Directory>>,
    pub error: Option<String>,
    pub loading: bool,
    generation: u64,
    pub query: String,
    pub kind: Kind,
    pub archived: bool,
    pub department: Option<String>,
    pub runtime: Runtime,
    pub table: bool,
    pub page: usize,
    pub manager_page: usize,
    pub card_size: usize,
    pub table_size: usize,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            directory: None,
            error: None,
            loading: false,
            generation: 0,
            query: String::new(),
            kind: Kind::All,
            archived: false,
            department: None,
            runtime: Runtime::All,
            table: false,
            page: 0,
            manager_page: 0,
            card_size: 9,
            table_size: 10,
        }
    }
}

impl Shared {
    pub fn reset_account(&self) -> Self {
        Self {
            table: self.table,
            card_size: self.card_size,
            table_size: self.table_size,
            generation: self.generation.wrapping_add(1),
            ..Self::default()
        }
    }
}

pub(super) fn message(event: ListEvent) -> Message {
    Message::DigitalEmployees(Event::Shared(event))
}

pub(super) fn update(app: &mut App, event: ListEvent) -> Task<Message> {
    let state = &mut app.digital_employees.shared;
    match event {
        ListEvent::Refresh => return refresh(app),
        ListEvent::Loaded(generation, result) => {
            if state.generation == generation && state.loading {
                state.loading = false;
                match result {
                    Ok(directory) => {
                        if state.department.as_deref().is_some_and(|path| {
                            !directory
                                .employees
                                .iter()
                                .any(|employee| employee.department() == Some(path))
                        }) {
                            state.department = None;
                        }
                        state.directory = Some(directory);
                    }
                    Err(error) => {
                        state.directory = None;
                        state.error = Some(error);
                    }
                }
            }
            return Task::none();
        }
        ListEvent::Search(query) => state.query = query,
        ListEvent::Kind(kind) => state.kind = kind,
        ListEvent::Archived(archived) => {
            state.archived = archived;
            state.runtime = Runtime::All;
        }
        ListEvent::Department(department) => state.department = department,
        ListEvent::Runtime(runtime) => state.runtime = runtime,
        ListEvent::Table(table) => state.table = table,
        ListEvent::Page { managers, index } => {
            if managers {
                state.manager_page = index;
            } else {
                state.page = index;
            }
            return Task::none();
        }
        ListEvent::PageSize(size) => {
            if state.table && [5, 10].contains(&size) {
                state.table_size = size;
            } else if !state.table && [3, 6, 9].contains(&size) {
                state.card_size = size;
            }
        }
    }
    state.page = 0;
    state.manager_page = 0;
    Task::none()
}

pub(super) fn refresh(app: &mut App) -> Task<Message> {
    let state = &mut app.digital_employees.shared;
    if state.loading {
        return Task::none();
    }
    let Some(account) = app.account.active_account() else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    state.loading = true;
    state.error = None;
    Task::perform(
        async move { shared_api::load(credential.expose()).await.map(std::sync::Arc::new) },
        move |result| message(ListEvent::Loaded(generation, result)),
    )
}
