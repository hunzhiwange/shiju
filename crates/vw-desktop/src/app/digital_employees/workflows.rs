//! Account-scoped, read-only workflow list.

use std::{collections::HashSet, fmt, sync::Arc};

use iced::Task;
use serde::Deserialize;

use crate::app::{App, Message};

use super::{Event, api, board};

pub(super) const PAGE_SIZE: usize = 9;

#[derive(Default)]
pub(super) struct List {
    pub items: Option<Arc<Vec<Workflow>>>,
    pub loading: bool,
    pub error: Option<String>,
    pub search: String,
    pub status: Option<Status>,
    pub department: Department,
    pub sort: Sort,
    pub page: usize,
    generation: u64,
}

#[derive(Clone)]
pub(crate) enum ListEvent {
    Refresh,
    Loaded(u64, Result<Arc<Vec<Workflow>>, String>),
    Search(String),
    Status(Option<Status>),
    Department(Department),
    Sort(Sort),
    Page(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Published,
    Draft,
    Offline,
    Unknown,
}

impl Status {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Published => "已发布",
            Self::Draft => "草稿",
            Self::Offline => "已下线",
            Self::Unknown => "—",
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Department {
    #[default]
    All,
    Company,
    Path(String),
}

impl Department {
    pub(super) fn matches(&self, workflow: &Workflow) -> bool {
        match self {
            Self::All => true,
            Self::Company => workflow.department() == Self::Company,
            Self::Path(prefix) => match workflow.department() {
                Self::Path(path) => super::departments::contains(prefix, &path),
                _ => false,
            },
        }
    }

    pub(super) fn label<'a>(&self, directory: Option<&'a super::departments::Directory>) -> &'a str {
        match self {
            Self::All => "全部可查看部门",
            Self::Company => "全公司",
            Self::Path(path) => {
                super::departments::name(directory, path).unwrap_or("部门名称未提供")
            }
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Sort {
    #[default]
    Updated,
    Name,
    Runs,
}

impl fmt::Display for Sort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Updated => "最近更新",
            Self::Name => "名称",
            Self::Runs => "近 30 日运行次数",
        })
    }
}

#[derive(Deserialize)]
struct Response {
    #[serde(default, deserialize_with = "api::nullable_list")]
    workflows: Vec<Workflow>,
    total: Option<usize>,
}

#[derive(Deserialize)]
pub(crate) struct Workflow {
    workflow_id: String,
    pub name: String,
    pub description: Option<String>,
    department_path: Option<String>,
    pub scope: Option<String>,
    availability: Option<String>,
    version: Option<u64>,
    updated_at: Option<String>,
    draft: Option<Version>,
    published: Option<Version>,
    #[serde(default, deserialize_with = "api::nullable_list")]
    pub step_titles: Vec<String>,
    pub active_task_count: Option<u64>,
    pub runs_30d: Option<u64>,
    pub succeeded_30d: Option<u64>,
}

#[derive(Deserialize)]
struct Version {
    version_no: u64,
}

impl Workflow {
    pub(super) fn status(&self) -> Status {
        if self.availability.as_deref() == Some("OFFLINE") {
            Status::Offline
        } else if self.published.is_some() {
            Status::Published
        } else if self.draft.is_some() {
            Status::Draft
        } else {
            Status::Unknown
        }
    }

    pub(super) fn version(&self) -> Option<u64> {
        self.published
            .as_ref()
            .or(self.draft.as_ref())
            .map(|version| version.version_no)
            .or(self.version.filter(|version| *version > 0))
    }

    pub(super) fn department(&self) -> Department {
        let path = self.department_path.as_deref().unwrap_or_default().trim();
        if path.is_empty() {
            Department::Company
        } else {
            let path = path.trim_matches('/');
            Department::Path(format!("/{path}"))
        }
    }

    fn matches_search(
        &self,
        search: &str,
        directory: Option<&super::departments::Directory>,
    ) -> bool {
        search.is_empty()
            || self.name.to_lowercase().contains(search)
            || self.description.as_ref().is_some_and(|value| value.to_lowercase().contains(search))
            || self.department().label(directory).to_lowercase().contains(search)
            || self
                .department_path
                .as_ref()
                .is_some_and(|path| path.to_lowercase().contains(search))
            || self.step_titles.iter().any(|step| step.to_lowercase().contains(search))
    }
}

impl List {
    pub(super) fn account_reset(&self) -> Self {
        Self { generation: self.generation.wrapping_add(1), ..Self::default() }
    }

    pub(super) fn filtered<'a>(
        &self,
        scoped: &[&'a Workflow],
        directory: Option<&super::departments::Directory>,
    ) -> Vec<&'a Workflow> {
        let search = self.search.trim().to_lowercase();
        let mut rows: Vec<_> = scoped
            .iter()
            .copied()
            .filter(|item| {
                self.status.is_none_or(|status| item.status() == status)
                    && item.matches_search(&search, directory)
            })
            .collect();
        match self.sort {
            Sort::Updated => rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            Sort::Name => rows.sort_by(|a, b| a.name.cmp(&b.name)),
            Sort::Runs => rows.sort_by(|a, b| b.runs_30d.cmp(&a.runs_30d)),
        }
        rows
    }
}

pub(super) fn message(event: ListEvent) -> Message {
    Message::DigitalEmployees(Event::Workflows(event))
}

pub(super) fn update(app: &mut App, event: ListEvent) -> Task<Message> {
    let state = &mut app.digital_employees.workflows;
    match event {
        ListEvent::Refresh => {
            return Task::batch([refresh(app), super::tasks::load_departments(app)]);
        }
        ListEvent::Loaded(generation, result) => {
            if state.generation != generation || !state.loading {
                return Task::none();
            }
            state.loading = false;
            match result {
                Ok(items) => {
                    if !items.iter().any(|item| state.department.matches(item)) {
                        state.department = Department::All;
                    }
                    state.items = Some(items);
                }
                Err(error) => {
                    state.items = None;
                    state.error = Some(error);
                }
            }
        }
        ListEvent::Search(search) => {
            state.search = search;
            state.page = 0;
        }
        ListEvent::Status(status) => {
            state.status = status;
            state.page = 0;
        }
        ListEvent::Department(department) => {
            state.department = department;
            state.page = 0;
        }
        ListEvent::Sort(sort) => {
            state.sort = sort;
            state.page = 0;
        }
        ListEvent::Page(page) => {
            state.page = page;
            return board::scroll_to_top();
        }
    }
    Task::none()
}

pub(super) fn refresh(app: &mut App) -> Task<Message> {
    let state = &mut app.digital_employees.workflows;
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
    Task::perform(async move { load(credential.expose()).await.map(Arc::new) }, move |result| {
        message(ListEvent::Loaded(generation, result))
    })
}

async fn load(credential: &str) -> Result<Vec<Workflow>, String> {
    let client = api::Client::new(credential)?;
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut offset = 0;
    loop {
        let response: Response =
            client.get("workflows", &[("limit", "100"), ("offset", &offset.to_string())]).await?;
        let count = response.workflows.len();
        offset += count;
        items.extend(
            response.workflows.into_iter().filter(|item| seen.insert(item.workflow_id.clone())),
        );
        if count == 0 || response.total.is_none_or(|total| offset >= total) {
            return Ok(items);
        }
    }
}
