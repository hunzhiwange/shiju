use std::{collections::HashMap, sync::Arc};

use iced::{Subscription, Task};

use crate::app::{App, Message, Screen};

use super::api::Client;
use super::board_api::{self, Collaboration, Detail, Handoff, Snapshot};
use super::{Event, Page, State};

pub(super) const PAGE_SIZE: usize = 6;
pub(super) const SCROLL_ID: &str = "digital-employees-content";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Queue {
    #[default]
    All,
    Running,
    Waiting,
}

#[derive(Default)]
pub(super) struct Board {
    pub snapshot: Option<Arc<Snapshot>>,
    pub loading: bool,
    pub error: Option<desktop_i18n::workflow::Feedback>,
    pub queue: Queue,
    pub task_view: bool,
    pub page: usize,
    pub employee: Option<String>,
    pub focus: Option<String>,
    pub expanded: Option<String>,
    pub details: HashMap<String, Option<Result<Arc<TaskRecord>, desktop_i18n::workflow::Feedback>>>,
    pub tour: bool,
    pub fullscreen: bool,
    pub still: bool,
    pub pulse: bool,
    generation: u64,
}

pub(crate) struct TaskRecord {
    pub(super) detail: Detail,
    pub(super) collaboration: Option<Collaboration>,
    pub(super) handoff: Option<Handoff>,
}

#[derive(Clone)]
pub(crate) enum BoardEvent {
    Refresh,
    Loaded(u64, Result<Arc<Snapshot>, desktop_i18n::workflow::Feedback>),
    TaskView(bool),
    Queue(Queue),
    Page(usize),
    Select(String),
    Focus(Option<String>),
    Expand(String),
    Retry(String),
    DetailLoaded(u64, String, Result<Arc<TaskRecord>, desktop_i18n::workflow::Feedback>),
    ToggleTour,
    ToggleMotion,
    MotionTick,
    TourTick,
    ToggleFullscreen,
}

pub(super) fn account_reset(state: &State) -> Board {
    Board {
        fullscreen: state.board.fullscreen,
        generation: state.board.generation.wrapping_add(1),
        ..Board::default()
    }
}

pub(crate) fn subscription(app: &App) -> Subscription<Message> {
    let board = &app.digital_employees.board;
    if app.navigation_tabs.screen != Screen::DigitalEmployees
        || app.digital_employees.page != Page::Board
        || board.task_view
    {
        return Subscription::none();
    }
    Subscription::batch([
        if board.tour {
            iced::time::every(std::time::Duration::from_secs(8))
                .map(|_| message(BoardEvent::TourTick))
        } else {
            Subscription::none()
        },
        if !board.still
            && board
                .snapshot
                .as_ref()
                .is_some_and(|data| data.tasks.iter().any(|task| task.running() && !task.paused()))
        {
            iced::time::every(std::time::Duration::from_millis(650))
                .map(|_| message(BoardEvent::MotionTick))
        } else {
            Subscription::none()
        },
    ])
}

pub(super) fn message(event: BoardEvent) -> Message {
    Message::DigitalEmployees(Event::Board(event))
}

pub(super) fn update(app: &mut App, event: BoardEvent) -> Task<Message> {
    let board = &mut app.digital_employees.board;
    match event {
        BoardEvent::Refresh => return refresh(app),
        BoardEvent::Loaded(generation, result) => {
            if generation == board.generation && board.loading {
                board.loading = false;
                match result {
                    Ok(snapshot) => {
                        let rows = snapshot
                            .summary
                            .roster
                            .as_ref()
                            .map(|roster| roster.rows.as_slice())
                            .unwrap_or_default();
                        if !rows.iter().any(|person| {
                            Some(person.employee_id.as_str()) == board.employee.as_deref()
                        }) {
                            board.employee = rows
                                .iter()
                                .find(|person| person.state == "working")
                                .or_else(|| rows.first())
                                .map(|person| person.employee_id.clone());
                        }
                        if !snapshot
                            .tasks
                            .iter()
                            .any(|task| Some(task.task_id.as_str()) == board.focus.as_deref())
                        {
                            board.focus = None;
                        }
                        board.page = 0;
                        board.expanded = None;
                        board.details.clear();
                        board.snapshot = Some(snapshot);
                        return load_visible_details(app);
                    }
                    Err(error) => {
                        board.snapshot = None;
                        board.details.clear();
                        board.error = Some(error);
                    }
                }
            }
        }
        BoardEvent::TaskView(tasks) => {
            board.task_view = tasks;
            return load_visible_details(app);
        }
        BoardEvent::Queue(queue) => {
            board.queue = queue;
            board.page = 0;
            return load_visible_details(app);
        }
        BoardEvent::Page(page) => {
            board.page = page;
            return load_visible_details(app);
        }
        BoardEvent::Select(id) => {
            board.employee = Some(id);
            let entering_office = board.task_view;
            board.task_view = false;
            let details = load_visible_details(app);
            return if entering_office { Task::batch([details, scroll_to_top()]) } else { details };
        }
        BoardEvent::Focus(id) => {
            board.focus = id.clone();
            board.task_view = false;
            if let Some(id) = id {
                if let Some(snapshot) = &board.snapshot
                    && let Some(task) = snapshot.tasks.iter().find(|task| task.task_id == id)
                    && let Some(rows) = snapshot.summary.roster.as_ref()
                    && let Some(person) = rows.rows.iter().find(|person| {
                        let id = person.employee_id.as_str();
                        task.owner_ref.as_deref() == Some(id)
                            || task.input_text("employee_id") == Some(id)
                    })
                {
                    board.employee = Some(person.employee_id.clone());
                }
            }
            return Task::batch([load_visible_details(app), scroll_to_top()]);
        }
        BoardEvent::Expand(id) => {
            if board.expanded.as_deref() == Some(&id) {
                board.expanded = None;
            } else {
                board.expanded = Some(id);
            }
        }
        BoardEvent::Retry(id) => {
            if board.details.get(&id).is_some_and(Option::is_some) {
                board.details.remove(&id);
                return load_detail(app, id);
            }
        }
        BoardEvent::DetailLoaded(generation, id, result) => {
            if generation == board.generation
                && let Some(entry) = board.details.get_mut(&id)
            {
                *entry = Some(result);
            }
        }
        BoardEvent::ToggleTour => board.tour = !board.tour,
        BoardEvent::ToggleMotion => {
            board.still = !board.still;
            board.pulse = false;
        }
        BoardEvent::MotionTick => {
            if !board.still {
                board.pulse = !board.pulse;
            }
        }
        BoardEvent::TourTick => {
            if board.tour
                && !board.task_view
                && let Some(rows) = board
                    .snapshot
                    .as_ref()
                    .and_then(|data| data.summary.roster.as_ref())
                    .map(|roster| &roster.rows)
                && !rows.is_empty()
            {
                let next = rows
                    .iter()
                    .position(|person| {
                        Some(person.employee_id.as_str()) == board.employee.as_deref()
                    })
                    .map_or(0, |index| (index + 1) % rows.len());
                board.employee = Some(rows[next].employee_id.clone());
                return load_visible_details(app);
            }
        }
        BoardEvent::ToggleFullscreen => {
            if let Some(id) = app.main_window_id {
                let fullscreen = !board.fullscreen;
                board.fullscreen = fullscreen;
                return iced::window::set_mode(
                    id,
                    if fullscreen {
                        iced::window::Mode::Fullscreen
                    } else {
                        iced::window::Mode::Windowed
                    },
                );
            }
        }
    }
    Task::none()
}

pub(super) fn refresh(app: &mut App) -> Task<Message> {
    let board = &mut app.digital_employees.board;
    if board.loading {
        return Task::none();
    }
    let Some(account) = app.account.active_account() else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    board.loading = true;
    board.error = None;
    board.details.retain(|_, entry| entry.is_some());
    board.generation = board.generation.wrapping_add(1);
    let generation = board.generation;
    Task::perform(
        async move { board_api::load(credential.expose()).await.map(Arc::new) },
        move |result| message(BoardEvent::Loaded(generation, result)),
    )
}

fn load_detail(app: &mut App, id: String) -> Task<Message> {
    let board = &mut app.digital_employees.board;
    if board.loading || board.details.contains_key(&id) {
        return Task::none();
    }
    let Some(account) = app.account.active_account() else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    let generation = board.generation;
    board.details.insert(id.clone(), None);
    let request_id = id.clone();
    Task::perform(
        async move {
            let client = Client::new(credential.expose())?;
            let (detail, collaboration, handoff) = tokio::join!(
                client.task::<Detail>(&request_id, None),
                client.task::<Collaboration>(&request_id, Some("collaboration")),
                client.task::<Handoff>(&request_id, Some("handoff")),
            );
            Ok(Arc::new(TaskRecord {
                detail: detail?,
                collaboration: collaboration.ok(),
                handoff: handoff.ok(),
            }))
        },
        move |result| message(BoardEvent::DetailLoaded(generation, id.clone(), result)),
    )
}

impl Queue {
    pub(super) fn includes(self, task: &board_api::WorkItem) -> bool {
        match self {
            Self::All => true,
            Self::Running => task.running(),
            Self::Waiting => task.waiting(),
        }
    }
}

fn load_visible_details(app: &mut App) -> Task<Message> {
    let board = &app.digital_employees.board;
    let Some(data) = &board.snapshot else { return Task::none() };
    let mut ids = Vec::new();
    if board.task_view {
        ids.extend(
            data.tasks
                .iter()
                .filter(|task| board.queue.includes(task))
                .skip(board.page.saturating_mul(PAGE_SIZE))
                .take(PAGE_SIZE)
                .map(|task| task.task_id.clone()),
        );
    } else {
        ids.extend(board.focus.iter().cloned());
        ids.extend(
            data.tasks
                .iter()
                .filter(|task| {
                    board.employee.as_deref().is_some_and(|id| {
                        task.owner_ref.as_deref() == Some(id)
                            || task.input_text("employee_id") == Some(id)
                    })
                })
                .take(3)
                .map(|task| task.task_id.clone()),
        );
    }
    Task::batch(ids.into_iter().map(|id| load_detail(app, id)))
}

pub(super) fn scroll_to_top() -> Task<Message> {
    iced::widget::operation::scroll_to(
        iced::widget::Id::new(SCROLL_ID),
        iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
    )
}
