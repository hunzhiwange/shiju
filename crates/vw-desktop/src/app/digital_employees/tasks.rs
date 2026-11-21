use std::{
    collections::HashSet,
    fmt,
    hash::{Hash, Hasher},
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, NaiveDateTime, TimeDelta, Timelike, Utc};
use desktop_settings_cron::system_settings_cron::date_time::{DateTimePicker, PickerEvent};
use iced::futures::{SinkExt, Stream};
use iced::widget::text_editor;

use iced::{Subscription, Task};

use crate::app::{App, Message, Screen};

use super::task_api::{self, Directory, List, Record, Schedules, Work};
use super::task_schedule::{RepeatPattern, RepeatSchedule, TIMEZONE, future_once};
use super::{Event, Page, board};

pub(super) const DETAIL_SCROLL_ID: &str = "digital-employees-task-detail";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Phase {
    #[default]
    All,
    Queued,
    Running,
    Waiting,
    Ended,
}

impl Phase {
    pub(super) const ALL: [Self; 5] =
        [Self::All, Self::Queued, Self::Running, Self::Waiting, Self::Ended];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Queued => "排队中",
            Self::Running => "执行中",
            Self::Waiting => "等人处理",
            Self::Ended => "已结束",
        }
    }

    pub(super) fn statuses(self) -> &'static [&'static str] {
        match self {
            Self::All => &[],
            Self::Queued => &["CREATED", "SCHEDULED", "QUEUED"],
            Self::Running => &["RUNNING", "CANCEL_REQUESTED"],
            Self::Waiting => &["WAITING_APPROVAL", "WAITING_INPUT", "HUMAN_TAKEOVER"],
            Self::Ended => &["SUCCEEDED", "FAILED", "REJECTED", "CANCELLED"],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Trigger {
    #[default]
    Now,
    Once,
    Cron,
}

impl Trigger {
    pub(super) const ALL: [Self; 3] = [Self::Now, Self::Once, Self::Cron];

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Now => "immediate",
            Self::Once => "once_at",
            Self::Cron => "cron",
        }
    }
}

impl fmt::Display for Trigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Now => "立即开始",
            Self::Once => "指定时间",
            Self::Cron => "周期执行",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Sort {
    #[default]
    Updated,
    Name,
    Status,
    Progress,
}

impl Sort {
    pub(super) const ALL: [Self; 4] = [Self::Updated, Self::Name, Self::Status, Self::Progress];

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Updated => "updated",
            Self::Name => "name",
            Self::Status => "status",
            Self::Progress => "progress",
        }
    }
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Updated => "最近更新",
            Self::Name => "按名称",
            Self::Status => "按状态",
            Self::Progress => "按进度",
        })
    }
}

#[derive(Clone, Default)]
pub(super) struct Filters {
    pub keyword: String,
    pub department: String,
    pub phase: Phase,
    pub trigger: Option<Trigger>,
    pub employee: String,
    pub workflow: String,
    pub sort: Sort,
}

#[derive(Clone)]
pub(super) struct Draft {
    pub trigger: Trigger,
    pub title: String,
    pub prompt: text_editor::Content,
    pub employee: String,
    pub workflow: String,
    pub department: String,
    pub once_at: Option<NaiveDateTime>,
    pub once_picker: Option<DateTimePicker>,
    pub repeat: RepeatSchedule,
    pub retained_input: Option<serde_json::Map<String, serde_json::Value>>,
    pub key: String,
}

impl Draft {
    fn new(trigger: Trigger, work: Option<&Work>, employee: String) -> Self {
        Self {
            trigger,
            title: work.map(|work| work.title.clone()).unwrap_or_default(),
            prompt: text_editor::Content::with_text(
                work.and_then(|work| work.input_text("prompt")).unwrap_or_default(),
            ),
            employee: work
                .and_then(|work| work.input_text("employee_id"))
                .map(str::to_owned)
                .unwrap_or(employee),
            workflow: work
                .and_then(|work| work.input_text("workflow_id"))
                .unwrap_or_default()
                .to_owned(),
            department: work.and_then(|work| work.department_path.clone()).unwrap_or_default(),
            once_at: work
                .and_then(|work| work.once_at.as_deref())
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&TIMEZONE).naive_local()),
            once_picker: None,
            repeat: RepeatSchedule::from_cron(
                work.and_then(|work| work.cron.as_deref()).unwrap_or("0 12 * * *"),
            ),
            retained_input: work.and_then(|work| work.input.as_ref()?.as_object()).map(|input| {
                ["attachments", "completion_requirements"]
                    .into_iter()
                    .filter_map(|key| input.get(key).map(|value| (key.to_owned(), value.clone())))
                    .collect()
            }),
            key: new_key(),
        }
    }
}

fn new_key() -> String {
    format!("task-{:032x}", rand::random::<u128>())
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DetailTab {
    #[default]
    Overview,
    Execution,
    Results,
    Collaboration,
    Diagnosis,
}

#[derive(Default)]
pub(super) struct Drawer {
    pub id: String,
    pub tab: DetailTab,
    pub record: Option<Arc<Record>>,
    pub error: Option<String>,
    pub loading: bool,
    pub view_only: bool,
    pub live: bool,
    pub fallback: bool,
    stream_revision: u64,
    refresh_pending: bool,
}

#[derive(Default)]
pub(super) struct Center {
    pub schedules: bool,
    pub filters: Filters,
    pub keyword: String,
    pub expanded: bool,
    pub page: usize,
    pub schedule_page: usize,
    pub list: Option<Arc<List>>,
    pub plans: Option<Arc<Schedules>>,
    pub directory: Option<Arc<Directory>>,
    pub directory_error: Option<String>,
    pub drawer: Option<Drawer>,
    pub draft: Option<Draft>,
    pub loading: bool,
    pub busy: bool,
    pub error: Option<String>,
    pub command_error: Option<String>,
    pub notice: Option<String>,
    pub selected: HashSet<String>,
    list_revision: u64,
    detail_revision: u64,
    stream_revision: u64,
    account_revision: u64,
    directory_loading: bool,
}

impl Center {
    pub fn reset_account(&self) -> Self {
        Self {
            account_revision: self.account_revision.wrapping_add(1),
            list_revision: self.list_revision.wrapping_add(1),
            detail_revision: self.detail_revision.wrapping_add(1),
            stream_revision: self.stream_revision.wrapping_add(1),
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum DraftField {
    Title,
    Employee,
    Workflow,
    Department,
}

#[derive(Clone, Copy)]
pub(crate) enum ScheduleAction {
    Pause,
    Resume,
    RunNow,
}

impl ScheduleAction {
    fn key(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::RunNow => "run-now",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Pause => "暂停",
            Self::Resume => "恢复",
            Self::RunNow => "立即执行",
        }
    }
}

#[derive(Clone)]
pub(crate) enum CenterEvent {
    ViewSchedules(bool),
    Load,
    ListLoaded(u64, Result<Arc<List>, String>),
    SchedulesLoaded(u64, Result<Arc<Schedules>, String>),
    DirectoryLoaded(u64, Result<Arc<Directory>, String>),
    Keyword(String),
    Query,
    Phase(Phase),
    Department(String),
    Employee(String),
    Workflow(String),
    TriggerFilter(Option<Trigger>),
    Sort(Sort),
    Clear,
    Expand,
    Page(usize),
    Open(String),
    Close,
    DetailTab(DetailTab),
    DetailLoaded(u64, Result<Arc<Record>, String>),
    RefreshDetail,
    Reconnect,
    AutoRefresh,
    StreamSignal(u64, StreamSignal),
    ViewOnly,
    New(Trigger, String),
    Reuse,
    CloseDraft,
    Edit(DraftField, String),
    EditPrompt(text_editor::Action),
    DraftTrigger(Trigger),
    OpenDatePicker,
    DatePicker(PickerEvent),
    ClearOnce,
    DraftRepeat(RepeatPattern),
    Submit,
    Retry(String),
    Select(String, bool),
    SelectPage(bool),
    BatchRetry,
    BatchDone(u64, Result<Arc<task_api::BatchResult>, String>),
    Schedule(String, ScheduleAction),
    CommandDone(u64, bool, Result<(), String>),
}

pub(super) fn message(event: CenterEvent) -> Message {
    Message::DigitalEmployees(Event::Tasks(event))
}

#[derive(Clone, Copy)]
pub(crate) enum StreamSignal {
    Connected,
    Changed,
    Disconnected,
}

struct StreamKey {
    revision: u64,
    id: String,
    credential: desktop_account::SecretSkey,
}

impl Hash for StreamKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.revision.hash(state);
        self.id.hash(state);
    }
}

fn stream(key: &StreamKey) -> impl Stream<Item = Message> + use<> {
    let StreamKey { revision, id, credential } = key;
    let (revision, id, credential) = (*revision, id.clone(), credential.clone());
    iced::stream::channel(8, async move |mut sender| {
        let signal = |value| message(CenterEvent::StreamSignal(revision, value));
        let connect =
            async { super::api::Client::new(credential.expose())?.task_stream(&id).await };
        if let Ok(Ok(mut response)) = tokio::time::timeout(Duration::from_secs(20), connect).await {
            if sender.send(signal(StreamSignal::Connected)).await.is_err() {
                return;
            }
            let mut buffer = Vec::new();
            let mut data = Vec::new();
            while let Ok(Ok(Some(chunk))) =
                tokio::time::timeout(Duration::from_secs(20), response.chunk()).await
            {
                buffer.extend_from_slice(&chunk);
                if buffer.len().saturating_add(data.len()) > 1024 * 1024 {
                    break;
                }
                let mut changed = false;
                if let Some(last) = buffer.iter().rposition(|byte| *byte == b'\n') {
                    for line in buffer
                        .drain(..=last)
                        .collect::<Vec<_>>()
                        .split_inclusive(|byte| *byte == b'\n')
                    {
                        let line = line.strip_suffix(b"\n").unwrap_or(line);
                        let line = line.strip_suffix(b"\r").unwrap_or(line);
                        if let Some(value) = line.strip_prefix(b"data:") {
                            data.extend_from_slice(value);
                            data.push(b'\n');
                        } else if line.is_empty() && !data.is_empty() {
                            changed |= serde_json::from_slice::<serde_json::Value>(&data)
                                .is_ok_and(|event| {
                                    event.get("event_id").is_some()
                                        || event.get("event_type").is_some()
                                });
                            data.clear();
                        }
                    }
                }
                if changed && sender.send(signal(StreamSignal::Changed)).await.is_err() {
                    return;
                }
            }
        }
        let _ = sender.send(signal(StreamSignal::Disconnected)).await;
    })
}

pub(super) fn subscription(app: &App) -> Subscription<Message> {
    if app.navigation_tabs.screen != Screen::DigitalEmployees
        || app.digital_employees.page != Page::Tasks
    {
        return Subscription::none();
    }
    let center = &app.digital_employees.tasks;
    let Some(drawer) = center.drawer.as_ref() else { return Subscription::none() };
    let Some(account) = app.account.active_account() else { return Subscription::none() };
    if center.draft.is_some()
        || drawer.record.as_ref().is_none_or(|record| {
            Phase::Ended.statuses().contains(&record.detail.task.status.as_str())
        })
    {
        return Subscription::none();
    }
    if drawer.fallback {
        iced::time::every(Duration::from_secs(5)).map(|_| message(CenterEvent::AutoRefresh))
    } else {
        Subscription::run_with(
            StreamKey {
                revision: drawer.stream_revision,
                id: drawer.id.clone(),
                credential: account.skill_hub_credential(),
            },
            stream,
        )
    }
}

pub(super) fn enter(app: &mut App) -> Task<Message> {
    Task::batch([load(app), load_directory(app)])
}

fn load(app: &mut App) -> Task<Message> {
    let Some(account) = app.account.active_account() else { return Task::none() };
    let key = account.skill_hub_credential();
    let center = &mut app.digital_employees.tasks;
    center.list_revision = center.list_revision.wrapping_add(1);
    center.loading = true;
    center.selected.clear();
    center.error = None;
    let revision = center.list_revision;
    if center.schedules {
        let page = center.schedule_page;
        Task::perform(
            async move { task_api::schedules(key.expose(), page).await.map(Arc::new) },
            move |result| message(CenterEvent::SchedulesLoaded(revision, result)),
        )
    } else {
        let filters = center.filters.clone();
        let page = center.page;
        Task::perform(
            async move { task_api::list(key.expose(), filters, page).await.map(Arc::new) },
            move |result| message(CenterEvent::ListLoaded(revision, result)),
        )
    }
}

fn load_directory(app: &mut App) -> Task<Message> {
    let Some(account) = app.account.active_account() else { return Task::none() };
    let center = &mut app.digital_employees.tasks;
    if center.directory_loading || center.directory.is_some() {
        return Task::none();
    }
    center.directory_loading = true;
    center.directory_error = None;
    let revision = center.account_revision;
    let key = account.skill_hub_credential();
    Task::perform(
        async move { task_api::directory(key.expose()).await.map(Arc::new) },
        move |result| message(CenterEvent::DirectoryLoaded(revision, result)),
    )
}

fn detail(app: &mut App) -> Task<Message> {
    let Some(account) = app.account.active_account() else { return Task::none() };
    let center = &mut app.digital_employees.tasks;
    let Some(drawer) = center.drawer.as_mut() else { return Task::none() };
    if drawer.loading {
        return Task::none();
    }
    drawer.loading = true;
    drawer.error = None;
    center.detail_revision = center.detail_revision.wrapping_add(1);
    let revision = center.detail_revision;
    let id = drawer.id.clone();
    let previous = drawer.record.clone();
    let key = account.skill_hub_credential();
    Task::perform(
        async move { task_api::detail(key.expose(), &id, previous).await.map(Arc::new) },
        move |result| message(CenterEvent::DetailLoaded(revision, result)),
    )
}

pub(super) fn route(app: &mut App, phase: Phase, id: Option<String>) -> Task<Message> {
    let center = &mut app.digital_employees.tasks;
    center.schedules = false;
    center.page = 0;
    center.filters = Filters { phase, ..Filters::default() };
    center.keyword.clear();
    center.drawer = None;
    center.draft = None;
    let nav = if app.digital_employees.page == Page::Tasks {
        enter(app)
    } else {
        super::select_page(app, Page::Tasks)
    };
    let drawer = id.map_or_else(Task::none, |id| update(app, CenterEvent::Open(id)));
    Task::batch([nav, drawer])
}

pub(super) fn update(app: &mut App, event: CenterEvent) -> Task<Message> {
    let center = &mut app.digital_employees.tasks;
    match event {
        CenterEvent::ViewSchedules(value) => {
            center.schedules = value;
            return load(app);
        }
        CenterEvent::Load => return enter(app),
        CenterEvent::ListLoaded(revision, result) if revision == center.list_revision => {
            center.loading = false;
            match result {
                Ok(data) if data.page.tasks.is_empty() && center.page > 0 => {
                    center.page = 0;
                    return load(app);
                }
                Ok(data) => center.list = Some(data),
                Err(error) => {
                    center.list = None;
                    center.error = Some(error);
                }
            }
        }
        CenterEvent::SchedulesLoaded(revision, result) if revision == center.list_revision => {
            center.loading = false;
            match result {
                Ok(data) if data.schedules.is_empty() && center.schedule_page > 0 => {
                    center.schedule_page = 0;
                    return load(app);
                }
                Ok(data) => center.plans = Some(data),
                Err(error) => {
                    center.plans = None;
                    center.error = Some(error);
                }
            }
        }
        CenterEvent::DirectoryLoaded(revision, result) if revision == center.account_revision => {
            center.directory_loading = false;
            match result {
                Ok(data) => center.directory = Some(data),
                Err(error) => center.directory_error = Some(error),
            }
        }
        CenterEvent::Keyword(value) => center.keyword = value,
        CenterEvent::Query => {
            center.filters.keyword = center.keyword.clone();
            return reset_page(app);
        }
        CenterEvent::Phase(value) => {
            center.filters.phase = value;
            return reset_page(app);
        }
        CenterEvent::Department(value) => {
            center.filters.department = value;
            return reset_page(app);
        }
        CenterEvent::Employee(value) => {
            center.filters.employee = value;
            return reset_page(app);
        }
        CenterEvent::Workflow(value) => {
            center.filters.workflow = value;
            return reset_page(app);
        }
        CenterEvent::TriggerFilter(value) => {
            center.filters.trigger = value;
            return reset_page(app);
        }
        CenterEvent::Sort(value) => {
            center.filters.sort = value;
            return reset_page(app);
        }
        CenterEvent::Clear => {
            center.filters = Filters::default();
            center.keyword.clear();
            return reset_page(app);
        }
        CenterEvent::Expand => center.expanded = !center.expanded,
        CenterEvent::Page(page) => {
            if center.schedules {
                center.schedule_page = page;
            } else {
                center.page = page;
            }
            return Task::batch([load(app), board::scroll_to_top()]);
        }
        CenterEvent::Open(id) => {
            center.stream_revision = center.stream_revision.wrapping_add(1);
            center.drawer =
                Some(Drawer { id, stream_revision: center.stream_revision, ..Drawer::default() });
            return detail(app);
        }
        CenterEvent::Close => center.drawer = None,
        CenterEvent::DetailTab(tab) => {
            if let Some(drawer) = &mut center.drawer {
                drawer.tab = tab;
                return iced::widget::operation::scroll_to(
                    iced::widget::Id::new(DETAIL_SCROLL_ID),
                    iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: 0.0 },
                );
            }
        }
        CenterEvent::DetailLoaded(revision, result) if revision == center.detail_revision => {
            if let Some(drawer) = &mut center.drawer {
                drawer.loading = false;
                match result {
                    Ok(record) => {
                        let changed = drawer.record.as_ref().is_some_and(|previous| {
                            let old = &previous.detail.task;
                            let new = &record.detail.task;
                            old.status != new.status || old.updated_at != new.updated_at
                        });
                        drawer.record = Some(record);
                        let refresh = std::mem::take(&mut drawer.refresh_pending);
                        if changed || refresh {
                            let list =
                                if changed && !center.loading { load(app) } else { Task::none() };
                            let next = if refresh { detail(app) } else { Task::none() };
                            return Task::batch([list, next]);
                        }
                    }
                    Err(error) => {
                        drawer.error = Some(error);
                        drawer.record = None;
                        drawer.live = false;
                        drawer.refresh_pending = false;
                    }
                }
            }
        }
        CenterEvent::RefreshDetail => return detail(app),
        CenterEvent::AutoRefresh => {
            if center.drawer.as_ref().is_some_and(|drawer| drawer.fallback) {
                return detail(app);
            }
        }
        CenterEvent::Reconnect => {
            if let Some(drawer) = &mut center.drawer {
                center.stream_revision = center.stream_revision.wrapping_add(1);
                drawer.stream_revision = center.stream_revision;
                drawer.live = false;
                drawer.fallback = false;
                drawer.refresh_pending = drawer.loading;
            }
            return detail(app);
        }
        CenterEvent::StreamSignal(revision, signal) => {
            if let Some(drawer) = &mut center.drawer
                && drawer.stream_revision == revision
            {
                match signal {
                    StreamSignal::Connected | StreamSignal::Changed => {
                        if matches!(signal, StreamSignal::Connected) {
                            drawer.live = true;
                        }
                        if drawer.loading {
                            drawer.refresh_pending = true;
                        } else {
                            return detail(app);
                        }
                    }
                    StreamSignal::Disconnected => {
                        drawer.live = false;
                        drawer.fallback = true;
                        return detail(app);
                    }
                }
            }
        }
        CenterEvent::ViewOnly => {
            if let Some(drawer) = &mut center.drawer {
                drawer.view_only = !drawer.view_only;
            }
        }
        CenterEvent::New(trigger, employee) if !center.busy => {
            center.command_error = None;
            center.draft = Some(Draft::new(trigger, None, employee));
            return load_directory(app);
        }
        CenterEvent::Reuse if !center.busy => {
            if let Some(record) = center.drawer.as_ref().and_then(|drawer| drawer.record.as_ref()) {
                let work = &record.detail.task;
                let trigger = Trigger::ALL
                    .into_iter()
                    .find(|trigger| work.trigger_type.as_deref() == Some(trigger.key()))
                    .unwrap_or_default();
                center.command_error = None;
                center.draft = Some(Draft::new(trigger, Some(work), String::new()));
                return load_directory(app);
            }
        }
        CenterEvent::CloseDraft if !center.busy => {
            center.draft = None;
            center.command_error = None;
        }
        CenterEvent::Edit(field, value) if !center.busy => {
            if let Some(draft) = &mut center.draft {
                let target = match field {
                    DraftField::Title => &mut draft.title,
                    DraftField::Employee => &mut draft.employee,
                    DraftField::Workflow => &mut draft.workflow,
                    DraftField::Department => &mut draft.department,
                };
                if *target != value {
                    *target = value;
                    draft.key = new_key();
                }
            }
        }
        CenterEvent::EditPrompt(action) if !center.busy => {
            if let Some(draft) = &mut center.draft {
                let before = action.is_edit().then(|| draft.prompt.text());
                draft.prompt.perform(action);
                if before.is_some_and(|before| before != draft.prompt.text()) {
                    draft.key = new_key();
                }
            }
        }
        CenterEvent::DraftTrigger(value) if !center.busy => {
            if let Some(draft) = &mut center.draft
                && draft.trigger != value
            {
                draft.trigger = value;
                draft.once_picker = None;
                draft.key = new_key();
                center.command_error = None;
            }
        }
        CenterEvent::OpenDatePicker if !center.busy => {
            if let Some(draft) = &mut center.draft
                && draft.trigger == Trigger::Once
                && draft.once_picker.is_none()
            {
                let time = draft.once_at.unwrap_or_else(|| {
                    (Utc::now() + TimeDelta::minutes(1)).with_timezone(&TIMEZONE).naive_local()
                });
                let mut picker = DateTimePicker::new(false, time.date(), time.time());
                picker.set_time(time.hour(), time.minute());
                draft.once_picker = Some(picker);
            }
        }
        CenterEvent::DatePicker(event) if !center.busy => {
            if let Some(draft) = &mut center.draft
                && let Some(picker) = &mut draft.once_picker
            {
                match event {
                    PickerEvent::Navigate(months) => picker.navigate(months),
                    PickerEvent::Date(date) => picker.select(date),
                    PickerEvent::Time(time) => picker.set_time(time.hour(), time.minute()),
                    PickerEvent::Cancel => draft.once_picker = None,
                    PickerEvent::Confirm => {
                        let value = Some(picker.date.and_time(picker.time));
                        draft.once_picker = None;
                        if value != draft.once_at {
                            draft.once_at = value;
                            draft.key = new_key();
                            center.command_error = None;
                        }
                    }
                }
            }
        }
        CenterEvent::ClearOnce if !center.busy => {
            if let Some(draft) = &mut center.draft
                && draft.trigger == Trigger::Once
                && draft.once_at.is_some()
            {
                draft.once_at = None;
                draft.key = new_key();
                center.command_error = None;
            }
        }
        CenterEvent::DraftRepeat(pattern) if !center.busy => {
            let repeat = RepeatSchedule::Pattern(pattern);
            if let Some(draft) = &mut center.draft
                && draft.trigger == Trigger::Cron
                && draft.repeat != repeat
            {
                draft.repeat = repeat;
                draft.key = new_key();
                center.command_error = None;
            }
        }
        CenterEvent::Submit if !center.busy => match validate(center) {
            Ok((draft, version, time)) => {
                return command(app, true, move |key| async move {
                    task_api::create(key.expose(), draft, version, time).await
                });
            }
            Err(error) => center.command_error = Some(error.into()),
        },
        CenterEvent::Retry(id) if !center.busy => {
            let rows = center.list.iter().flat_map(|list| &list.page.tasks);
            let detail = center
                .drawer
                .iter()
                .filter_map(|drawer| drawer.record.as_ref())
                .map(|record| &record.detail.task);
            if detail
                .chain(rows)
                .find(|row| row.task_id == id)
                .is_some_and(|row| row.allows("retry"))
            {
                return command(app, false, move |key| async move {
                    task_api::retry(key.expose(), &id).await
                });
            }
        }
        CenterEvent::Select(id, checked) if !center.busy => {
            if checked {
                center.selected.insert(id);
            } else {
                center.selected.remove(&id);
            }
        }
        CenterEvent::SelectPage(checked) if !center.busy => {
            center.selected.clear();
            if checked && let Some(list) = &center.list {
                center.selected.extend(list.page.tasks.iter().map(|work| work.task_id.clone()));
            }
        }
        CenterEvent::BatchRetry if !center.busy => {
            let ids: Vec<_> = center
                .list
                .iter()
                .flat_map(|list| &list.page.tasks)
                .filter(|work| center.selected.contains(&work.task_id) && work.allows("retry"))
                .map(|work| work.task_id.clone())
                .collect();
            let Some(account) = app.account.active_account() else { return Task::none() };
            if !ids.is_empty() {
                let key = account.skill_hub_credential();
                let revision = center.account_revision;
                center.busy = true;
                center.command_error = None;
                center.notice = None;
                return Task::perform(
                    async move { task_api::batch(key.expose(), ids, "retry").await.map(Arc::new) },
                    move |result| message(CenterEvent::BatchDone(revision, result)),
                );
            }
        }
        CenterEvent::BatchDone(revision, result) if revision == center.account_revision => {
            center.busy = false;
            match result {
                Ok(result) => {
                    center.notice = Some(format!(
                        "批量重跑 · 成功 {}，已处理 {}，跳过 {}，失败 {}",
                        result.succeeded,
                        result.noops,
                        result.skipped.len(),
                        result.failed
                    ));
                    return load(app);
                }
                Err(error) => center.command_error = Some(error),
            }
        }
        CenterEvent::Schedule(id, action) if !center.busy => {
            if center
                .plans
                .as_ref()
                .is_some_and(|plans| plans.schedules.iter().any(|plan| plan.schedule_id == id))
            {
                return command(app, false, move |key| async move {
                    task_api::schedule_command(key.expose(), &id, action.key()).await
                });
            }
        }
        CenterEvent::CommandDone(revision, created, result)
            if revision == center.account_revision =>
        {
            center.busy = false;
            match result {
                Ok(()) => {
                    if created {
                        center.schedules = center
                            .draft
                            .as_ref()
                            .is_some_and(|draft| draft.trigger != Trigger::Now);
                        center.draft = None;
                        center.drawer = None;
                        center.page = 0;
                        center.schedule_page = 0;
                    }
                    center.notice =
                        Some(if created { "任务配置已提交" } else { "操作已提交" }.into());
                    return Task::batch([load(app), detail(app)]);
                }
                Err(error) => center.command_error = Some(error),
            }
        }
        _ => {}
    }
    Task::none()
}

fn reset_page(app: &mut App) -> Task<Message> {
    app.digital_employees.tasks.page = 0;
    load(app)
}

fn validate(center: &Center) -> Result<(Draft, String, Option<String>), &'static str> {
    let draft = center.draft.as_ref().ok_or("请先填写任务配置")?;
    let directory = center.directory.as_ref().ok_or("员工和流程尚未加载")?;
    if draft.title.trim().is_empty() {
        return Err("请填写任务名称");
    }
    if !directory
        .employees
        .iter()
        .any(|employee| employee.employee_id == draft.employee && employee.can_dispatch())
    {
        return Err("请选择可派工的共享员工");
    }
    let version = directory
        .workflows
        .iter()
        .find(|workflow| workflow.workflow_id == draft.workflow)
        .and_then(|workflow| workflow.version())
        .ok_or("请选择已发布的可用流程")?;
    let mut time = None;
    match draft.trigger {
        Trigger::Once => {
            let local = draft.once_at.ok_or("请选择执行日期和时间")?;
            time = Some(future_once(local)?.to_rfc3339());
        }
        Trigger::Cron if draft.repeat.cron().split_whitespace().count() != 5 => {
            return Err("原周期规则无效，请重新选择重复周期");
        }
        _ => {}
    }
    Ok((draft.clone(), version.to_owned(), time))
}

fn command<F, Fut>(app: &mut App, created: bool, run: F) -> Task<Message>
where
    F: FnOnce(desktop_account::SecretSkey) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    let Some(account) = app.account.active_account() else { return Task::none() };
    let key = account.skill_hub_credential();
    let center = &mut app.digital_employees.tasks;
    center.busy = true;
    center.command_error = None;
    center.notice = None;
    let revision = center.account_revision;
    Task::perform(async move { run(key).await }, move |result| {
        message(CenterEvent::CommandDone(revision, created, result))
    })
}
