//! Native digital employee workbench, scoped to the active ShiJu account.

mod api;
mod assistants;
mod assistants_view;
mod board;
mod board_api;
mod board_view;
mod portraits;
mod shared;
mod shared_api;
mod shared_view;
mod task_api;
mod tasks;
mod tasks_view;
mod trend;
mod view;
mod workflows;
mod workflows_view;

use crate::app::{App, AppTab, Message, Screen};
use iced::{Subscription, Task};

pub(crate) fn subscription(app: &App) -> Subscription<Message> {
    Subscription::batch([board::subscription(app), tasks::subscription(app)])
}
pub(crate) use view::view;

const TAB_ID: &str = "shiju-digital-employees";

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Page {
    #[default]
    Workbench,
    Board,
    Tasks,
    Assistants,
    Shared,
    Workflows,
}

#[derive(Default)]
pub(crate) struct State {
    generation: u64,
    page: Page,
    assistants: assistants::List,
    board: board::Board,
    tasks: tasks::Center,
    shared: shared::Shared,
    workflows: workflows::List,
    summary: Option<Box<api::Summary>>,
    loading: bool,
    error: Option<String>,
    trend_values: [bool; 2],
}

#[derive(Clone)]
pub(crate) enum Event {
    Open,
    SelectPage(Page),
    Assistants(assistants::ListEvent),
    Board(board::BoardEvent),
    Tasks(tasks::CenterEvent),
    TaskRoute(tasks::Phase, Option<String>),
    NewTask(String),
    Shared(shared::ListEvent),
    Workflows(workflows::ListEvent),
    Refresh,
    Loaded(u64, Result<Box<api::Summary>, String>),
    ToggleValues(trend::Metric),
}

impl std::fmt::Debug for Event {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Open => "Open",
            Self::SelectPage(_) => "SelectPage",
            Self::Assistants(_) => "Assistants",
            Self::Board(_) => "Board",
            Self::Tasks(_) => "Tasks",
            Self::TaskRoute(..) => "TaskRoute",
            Self::NewTask(_) => "NewTask",
            Self::Shared(_) => "Shared",
            Self::Workflows(_) => "Workflows",
            Self::Refresh => "Refresh",
            Self::Loaded(..) => "Loaded",
            Self::ToggleValues(_) => "ToggleValues",
        })
    }
}

pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::SelectPage(page) => return select_page(app, page),
        Event::Assistants(event) => return assistants::update(app, event),
        Event::Board(event) => return board::update(app, event),
        Event::Tasks(event) => return tasks::update(app, event),
        Event::TaskRoute(phase, id) => return tasks::route(app, phase, id),
        Event::NewTask(employee) => {
            let page = tasks::route(app, tasks::Phase::All, None);
            let draft = tasks::update(app, tasks::CenterEvent::New(tasks::Trigger::Now, employee));
            return Task::batch([page, draft]);
        }
        Event::Shared(event) => return shared::update(app, event),
        Event::Workflows(event) => return workflows::update(app, event),
        Event::Open => {
            if !app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == TAB_ID) {
                app.navigation_tabs.open_tabs.push(AppTab {
                    id: TAB_ID.to_owned(),
                    title: crate::app::i18n::I18n::new(app.general_settings.desktop_locale)
                        .__("数字员工")
                        .to_owned(),
                    screen: Screen::DigitalEmployees,
                    project_path: None,
                });
            }
            app.navigation_tabs.active_tab_id = Some(TAB_ID.to_owned());
            app.navigation_tabs.screen = Screen::DigitalEmployees;
            return refresh_visible(app);
        }
        Event::Refresh => return refresh(app),
        Event::Loaded(generation, result) => {
            let state = &mut app.digital_employees;
            if state.generation == generation && state.loading {
                state.loading = false;
                match result {
                    Ok(summary) => state.summary = Some(summary),
                    Err(error) => {
                        state.summary = None;
                        state.error = Some(error);
                    }
                }
            }
        }
        Event::ToggleValues(metric) => {
            let show = &mut app.digital_employees.trend_values[metric as usize];
            *show = !*show;
        }
    }
    Task::none()
}

pub(crate) fn account_changed(app: &mut App) -> Task<Message> {
    let generation = app.digital_employees.generation.wrapping_add(1);
    let page = app.digital_employees.page;
    let board = board::account_reset(&app.digital_employees);
    let assistants = app.digital_employees.assistants.account_reset();
    let shared = app.digital_employees.shared.reset_account();
    let workflows = app.digital_employees.workflows.account_reset();
    let tasks = app.digital_employees.tasks.reset_account();
    app.digital_employees =
        State { generation, page, board, tasks, assistants, shared, workflows, ..State::default() };
    if app.navigation_tabs.open_tabs.iter().any(|tab| tab.id == TAB_ID) {
        return refresh_visible(app);
    }
    Task::none()
}

fn select_page(app: &mut App, page: Page) -> Task<Message> {
    let state = &mut app.digital_employees;
    if state.page == page {
        return Task::none();
    }
    state.page = page;
    let window = if page != Page::Board && state.board.fullscreen {
        state.board.fullscreen = false;
        app.main_window_id
            .map_or_else(Task::none, |id| iced::window::set_mode(id, iced::window::Mode::Windowed))
    } else {
        Task::none()
    };
    let load = match page {
        Page::Workbench if state.summary.is_none() => refresh(app),
        Page::Board if state.board.snapshot.is_none() => board::refresh(app),
        Page::Tasks => tasks::enter(app),
        Page::Assistants => assistants::refresh(app),
        Page::Shared if state.shared.directory.is_none() => shared::refresh(app),
        Page::Workflows if state.workflows.items.is_none() => workflows::refresh(app),
        _ => Task::none(),
    };
    Task::batch([window, board::scroll_to_top(), load])
}

fn refresh_visible(app: &mut App) -> Task<Message> {
    match app.digital_employees.page {
        Page::Workbench => refresh(app),
        Page::Board => board::refresh(app),
        Page::Tasks => tasks::enter(app),
        Page::Assistants => assistants::refresh(app),
        Page::Shared => shared::refresh(app),
        Page::Workflows => workflows::refresh(app),
    }
}

fn refresh(app: &mut App) -> Task<Message> {
    let state = &mut app.digital_employees;
    if state.loading {
        return Task::none();
    }
    let Some(account) = app.account.active_account() else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    state.generation = state.generation.wrapping_add(1);
    state.loading = true;
    state.error = None;
    let generation = state.generation;
    Task::perform(
        async move { api::load(credential.expose()).await.map(Box::new) },
        move |result| Message::DigitalEmployees(Event::Loaded(generation, result)),
    )
}
