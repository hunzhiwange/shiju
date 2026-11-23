use iced::Task;

use crate::app::{App, Message};

use super::{
    Event, Page, api,
    shared_api::{self, Directory, Operation},
    tasks,
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
            Self::All => "全部状态",
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
    Chat(String),
    Dispatch(String),
    Configure(String),
    OpenOperation(String, Operation),
    TemplateName(String),
    CloseOperation,
    SubmitOperation,
    Operated(u64, Result<&'static str, String>),
}

pub(super) struct Confirmation {
    employee_id: String,
    pub employee_name: String,
    pub operation: Operation,
    pub template_name: String,
    pub submitting: bool,
    pub error: Option<String>,
}

pub(super) struct Shared {
    pub confirmation: Option<Confirmation>,
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
            confirmation: None,
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
        ListEvent::Refresh => {
            return Task::batch([refresh(app), super::tasks::load_departments(app)]);
        }
        ListEvent::Chat(id) => {
            let selection = state.directory.as_ref().and_then(|directory| directory.employees.iter()
                .find(|employee| employee.employee_id == id && employee.can_chat()))
                .map(super::chat::Selection::shared);
            return super::chat::select(app, Page::Shared, selection);
        }
        ListEvent::Dispatch(id) => {
            if app.digital_employees.page != Page::Shared
                || app.digital_employees.tasks.busy
                || !state.directory.as_ref().is_some_and(|directory| {
                    directory.employees.iter().any(|employee| {
                        employee.employee_id == id && directory.access.can_dispatch(employee)
                    })
                })
            {
                return Task::none();
            }
            // Refresh the task form options because an employee may have just become available.
            app.digital_employees.tasks.directory = None;
            let page = super::select_page(app, Page::Tasks);
            let draft = tasks::update(app, tasks::CenterEvent::New(tasks::Trigger::Now, id));
            return Task::batch([page, draft]);
        }
        ListEvent::Configure(id) => {
            if app.digital_employees.page == Page::Shared
                && state.directory.as_ref().is_some_and(|directory| {
                    directory.employees.iter().any(|employee| {
                        employee.employee_id == id && directory.access.can_configure(employee)
                    })
                })
            {
                return crate::app::message::view::update(
                    app,
                    crate::app::message::view::ViewMessage::OpenUrlExternal(
                        api::employee_config_url(&id),
                    ),
                );
            }
            return Task::none();
        }
        ListEvent::OpenOperation(id, operation) => {
            if app.digital_employees.page == Page::Shared
                && !state.loading
                && state.confirmation.is_none()
            {
                if let Some(employee) = state.directory.as_ref().and_then(|directory| {
                    directory.employees.iter().find(|employee| {
                        employee.employee_id == id
                            && directory.access.can_manage(employee, operation)
                    })
                }) {
                    state.confirmation = Some(Confirmation {
                        employee_id: id,
                        employee_name: employee.name.clone(),
                        operation,
                        template_name: format!("{}模板", employee.name),
                        submitting: false,
                        error: None,
                    });
                }
            }
            return Task::none();
        }
        ListEvent::TemplateName(name) => {
            if let Some(dialog) = state.confirmation.as_mut().filter(|dialog| !dialog.submitting) {
                dialog.template_name = name;
            }
            return Task::none();
        }
        ListEvent::CloseOperation => {
            if !state.confirmation.as_ref().is_some_and(|dialog| dialog.submitting) {
                state.confirmation = None;
            }
            return Task::none();
        }
        ListEvent::SubmitOperation => return submit_operation(app),
        ListEvent::Operated(generation, result) => {
            if generation == state.generation {
                if let Some(dialog) = state.confirmation.as_mut().filter(|dialog| dialog.submitting)
                {
                    match result {
                        Ok(notice) => {
                            state.confirmation = None;
                            let toast =
                                app.show_toast(crate::app::state::ToastKind::Success, notice);
                            return Task::batch([toast, refresh(app)]);
                        }
                        Err(error) => {
                            dialog.submitting = false;
                            dialog.error = Some(error);
                        }
                    }
                }
            }
            return Task::none();
        }
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
    if state.loading || state.confirmation.is_some() {
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

fn submit_operation(app: &mut App) -> Task<Message> {
    if app.digital_employees.page != Page::Shared {
        return Task::none();
    }
    let Some(account) = app.account.active_account() else { return Task::none() };
    let state = &mut app.digital_employees.shared;
    let Some(dialog) = state.confirmation.as_mut().filter(|dialog| !dialog.submitting) else {
        return Task::none();
    };
    if dialog.error.is_some() {
        return Task::none();
    }
    let Some(employee) = state.directory.as_ref().and_then(|directory| {
        directory.employees.iter().find(|employee| {
            employee.employee_id == dialog.employee_id
                && directory.access.can_manage(employee, dialog.operation)
        })
    }) else {
        dialog.error = Some("当前员工无法执行此操作，请刷新后重试".into());
        return Task::none();
    };
    let operation = dialog.operation;
    let name = dialog.template_name.trim().to_owned();
    if operation == Operation::SaveTemplate && name.is_empty() {
        return Task::none();
    }
    let version = employee.version;
    let id = dialog.employee_id.clone();
    let credential = account.skill_hub_credential();
    let generation = state.generation;
    dialog.submitting = true;
    Task::perform(
        async move { shared_api::operate(credential.expose(), &id, operation, version, &name).await },
        move |result| message(ListEvent::Operated(generation, result)),
    )
}
