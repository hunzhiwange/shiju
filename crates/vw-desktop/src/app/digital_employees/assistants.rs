//! Read-only private employee list scoped by Agent Cloud.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
    time::Duration,
};

use iced::Task;
use serde::Deserialize;

use crate::app::{App, Message};

use super::{Event, api::Client, board};

pub(super) const PAGE_SIZE: usize = 9;

#[derive(Default)]
pub(super) struct List {
    pub data: Option<Arc<Snapshot>>,
    pub loading: bool,
    pub error: Option<String>,
    pub organization: bool,
    pub search: String,
    pub filter: RunFilter,
    pub page: usize,
    generation: u64,
}

#[derive(Clone)]
pub(crate) enum ListEvent {
    Refresh,
    Loaded(u64, Result<Arc<Snapshot>, String>),
    Scope(bool),
    Search(String),
    Run(RunFilter),
    Page(usize),
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum RunFilter {
    #[default]
    All,
    OnDuty(usize),
    Abnormal(usize),
}

impl RunFilter {
    pub(super) fn matches(self, employee: &Assistant) -> bool {
        match self {
            Self::All => true,
            Self::OnDuty(_) => employee.runtime() == "on_duty",
            Self::Abnormal(_) => employee.runtime() == "abnormal",
        }
    }
}

impl fmt::Display for RunFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("全部"),
            Self::OnDuty(count) => write!(f, "在岗 {count}"),
            Self::Abnormal(count) => write!(f, "异常 {count}"),
        }
    }
}

pub(crate) struct Snapshot {
    pub(super) mine: Vec<Assistant>,
    pub(super) governance: Result<Option<Vec<Assistant>>, String>,
    pub(super) members: HashMap<String, Member>,
}

#[derive(Deserialize)]
struct Response {
    assistants: Vec<Assistant>,
}

#[derive(Deserialize)]
pub(super) struct Assistant {
    pub name: String,
    pub department_path: String,
    pub owner_user_id: String,
    pub created_by: String,
    pub publish_status: String,
    pub runtime_status: Option<String>,
    pub capability_summary: Capabilities,
    pub usage_count: u64,
    pub owned_by_caller: bool,
}

#[derive(Deserialize)]
pub(super) struct Capabilities {
    pub skill_count: u64,
    pub mcp_count: u64,
}

#[derive(Deserialize)]
struct IamResource {
    #[serde(default, deserialize_with = "super::api::nullable_list")]
    elements: Vec<String>,
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(super) struct IamPage<T> {
    #[serde(deserialize_with = "super::api::nullable_list")]
    pub(super) list: Vec<T>,
    #[serde(default, deserialize_with = "deserialize_iam_total")]
    total: Option<usize>,
}

fn deserialize_iam_total<'de, D>(deserializer: D) -> Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Total {
        Number(usize),
        String(String),
    }

    match Option::<Total>::deserialize(deserializer)? {
        Some(Total::Number(total)) => Ok(Some(total)),
        Some(Total::String(total)) => total.parse().map(Some).map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

#[derive(Deserialize)]
pub(super) struct Member {
    user_id: String,
    pub name: String,
    #[serde(default, deserialize_with = "super::api::nullable_list")]
    departments: Vec<Membership>,
    #[serde(skip)]
    pub department: Option<String>,
}

#[derive(Deserialize)]
struct Membership {
    department_id: String,
}

#[derive(Deserialize)]
pub(super) struct Department {
    department_id: String,
    pub(super) department_name: String,
    pub(super) department_path: Option<String>,
}

impl Assistant {
    pub(super) fn runtime(&self) -> &str {
        self.runtime_status.as_deref().unwrap_or_default().trim()
    }

    pub(super) fn owner_name<'a>(
        &self,
        members: &'a HashMap<String, Member>,
        actor: Option<&'a str>,
    ) -> Option<&'a str> {
        members
            .get(&self.owner_user_id)
            .map(|member| member.name.as_str())
            .or_else(|| actor.filter(|_| self.owned_by_caller))
    }

    pub(super) fn creator_name<'a>(
        &self,
        members: &'a HashMap<String, Member>,
        actor: Option<&'a str>,
    ) -> Option<&'a str> {
        members.get(&self.created_by).map(|member| member.name.as_str()).or_else(|| {
            actor.filter(|_| self.owned_by_caller && self.created_by == self.owner_user_id)
        })
    }
}

impl List {
    pub(super) fn account_reset(&self) -> Self {
        Self { generation: self.generation.wrapping_add(1), ..Self::default() }
    }
}

pub(super) fn message(event: ListEvent) -> Message {
    Message::DigitalEmployees(Event::Assistants(event))
}

pub(super) fn update(app: &mut App, event: ListEvent) -> Task<Message> {
    let state = &mut app.digital_employees.assistants;
    match event {
        ListEvent::Refresh => return refresh(app),
        ListEvent::Loaded(generation, result) => {
            if generation == state.generation && state.loading {
                state.loading = false;
                match result {
                    Ok(data) => {
                        if matches!(&data.governance, Ok(None)) {
                            state.organization = false;
                        }
                        state.data = Some(data);
                    }
                    Err(error) => {
                        state.data = None;
                        state.error = Some(error);
                    }
                }
            }
        }
        ListEvent::Scope(organization) => {
            state.organization = organization;
            state.page = 0;
        }
        ListEvent::Search(search) => {
            state.search = search;
            state.page = 0;
        }
        ListEvent::Run(filter) => {
            state.filter = filter;
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
    let state = &mut app.digital_employees.assistants;
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

async fn load(credential: &str) -> Result<Snapshot, String> {
    let client = Client::new(credential)?;
    let (mine, governance) = tokio::join!(
        client.get::<Response>("assistants", &[("view", "mine")]),
        load_governance(&client),
    );
    let mine = mine?.assistants;
    let governance = governance.map(|response| response.map(|value| value.assistants));
    let organization = governance.as_ref().ok().and_then(Option::as_deref).unwrap_or_default();
    let members = tokio::time::timeout(
        Duration::from_secs(5),
        load_members(&client, mine.iter().chain(organization)),
    )
    .await
    .unwrap_or_default();
    Ok(Snapshot { mine, governance, members })
}

async fn load_governance(client: &Client) -> Result<Option<Response>, String> {
    let resource =
        client.iam::<IamResource>("me/resource", &[("service_code", "agent-worker")]).await?;
    if !resource
        .elements
        .iter()
        .any(|code| code == "agent-worker:element:assistants:govern_assistant")
    {
        return Ok(None);
    }
    client.assistant_governance().await
}

async fn load_members<'a>(
    client: &Client,
    assistants: impl Iterator<Item = &'a Assistant>,
) -> HashMap<String, Member> {
    let mut wanted = HashSet::new();
    let mut owner_departments = HashMap::new();
    for assistant in assistants {
        wanted.extend([assistant.owner_user_id.as_str(), assistant.created_by.as_str()]);
        owner_departments
            .insert(assistant.owner_user_id.as_str(), assistant.department_path.as_str());
    }
    if wanted.is_empty() {
        return HashMap::new();
    }
    let (departments, users) = tokio::join!(
        client.iam::<IamPage<Department>>("departments", &[]),
        load_users(client, wanted),
    );
    let departments: HashMap<_, _> = departments
        .ok()
        .into_iter()
        .flat_map(|result| result.list)
        .map(|department| (department.department_id.clone(), department))
        .collect();
    users
        .into_iter()
        .map(|mut member| {
            let preferred = owner_departments.get(member.user_id.as_str()).copied();
            let mut choices =
                member.departments.iter().filter_map(|entry| departments.get(&entry.department_id));
            member.department = choices
                .clone()
                .find(|department| department.department_path.as_deref() == preferred)
                .or_else(|| choices.next())
                .map(|department| department.department_name.clone());
            (member.user_id.clone(), member)
        })
        .collect()
}

pub(super) async fn load_users(client: &Client, mut wanted: HashSet<&str>) -> Vec<Member> {
    let mut page = 1;
    let mut scanned = 0;
    let mut users = Vec::new();
    while let Ok(result) = client
        .iam::<IamPage<Member>>("users", &[("page", &page.to_string()), ("page_size", "100")])
        .await
    {
        if result.list.is_empty() {
            break;
        }
        scanned += result.list.len();
        users.extend(
            result.list.into_iter().filter(|member| wanted.remove(member.user_id.as_str())),
        );
        if wanted.is_empty() || result.total.is_none_or(|total| scanned >= total) {
            break;
        }
        page += 1;
    }
    users
}
