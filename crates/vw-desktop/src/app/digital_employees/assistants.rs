//! Private employee list and ownership transfer scoped by Agent Cloud.

use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
    time::Duration,
};

use iced::Task;
use serde::Deserialize;

use crate::app::{App, Message};

use super::{Event, Page, api::Client, board};

pub(super) const PAGE_SIZE: usize = 9;

#[derive(Default)]
pub(super) struct List {
    pub data: Option<Arc<Snapshot>>,
    pub loading: bool,
    pub error: Option<desktop_i18n::workflow::Feedback>,
    pub organization: bool,
    pub search: String,
    pub filter: RunFilter,
    pub page: usize,
    pub transfer: Option<Transfer>,
    generation: u64,
}

#[derive(Clone)]
pub(crate) enum ListEvent {
    Refresh,
    Loaded(u64, Result<Arc<Snapshot>, desktop_i18n::workflow::Feedback>),
    Scope(bool),
    Search(String),
    Run(RunFilter),
    Page(usize),
    Chat(String),
    OpenTransfer(String),
    CloseTransfer,
    TransferMembers(u64, Result<Vec<Recipient>, desktop_i18n::workflow::Feedback>),
    TransferSearch(String),
    TransferSelect(Recipient),
    SubmitTransfer,
    TransferFinished(u64, Result<(), desktop_i18n::workflow::Feedback>),
}

pub(super) struct Transfer {
    employee_id: String,
    pub recipients: Option<Vec<Recipient>>,
    pub selected: Option<Recipient>,
    pub search: String,
    pub submitting: bool,
    pub error: Option<desktop_i18n::workflow::Feedback>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Recipient {
    id: String,
    label: String,
}

impl fmt::Display for Recipient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

impl Recipient {
    pub(super) fn matches(&self, query: &str) -> bool {
        self.label.to_lowercase().contains(query)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum RunFilter {
    #[default]
    All,
    OnDuty,
    Abnormal,
}

impl RunFilter {
    pub(super) fn matches(self, employee: &Assistant) -> bool {
        match self {
            Self::All => true,
            Self::OnDuty => employee.runtime() == "on_duty",
            Self::Abnormal => employee.runtime() == "abnormal",
        }
    }
}

impl fmt::Display for RunFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("全部状态"),
            Self::OnDuty => f.write_str("在岗"),
            Self::Abnormal => f.write_str("异常"),
        }
    }
}

pub(crate) struct Snapshot {
    pub(super) mine: Vec<Assistant>,
    pub(super) governance: Result<Option<Vec<Assistant>>, desktop_i18n::workflow::Feedback>,
    pub(super) members: HashMap<String, Member>,
}

#[derive(Deserialize)]
struct Response {
    assistants: Vec<Assistant>,
}

#[derive(Deserialize)]
pub(super) struct Assistant {
    employee_id: Option<String>,
    version: Option<u64>,
    #[serde(default, deserialize_with = "super::api::nullable_list")]
    allowed_actions: Vec<String>,
    pub name: String,
    pub department_path: String,
    pub owner_user_id: String,
    pub created_by: String,
    pub publish_status: String,
    pub status: Option<String>,
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
    account_name: Option<String>,
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
    pub(super) fn chat_id(&self) -> Option<&str> {
        self.employee_id.as_deref().filter(|id| {
            !id.trim().is_empty()
                && self.owned_by_caller
                && self.publish_status == "PUBLISHED"
                && self.status.as_deref().map_or(self.runtime() == "on_duty", |status| status == "ACTIVE")
        })
    }

    pub(super) fn transfer_id(&self) -> Option<&str> {
        self.employee_id.as_deref().filter(|id| {
            !id.is_empty()
                && self.version.is_some()
                && self.allowed_actions.iter().any(|action| action == "transfer_owner")
        })
    }

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
    pub(super) fn transfer_employee(&self) -> Option<&Assistant> {
        self.transfer.as_ref().and_then(|transfer| self.transfer_target(&transfer.employee_id))
    }

    fn transfer_target(&self, id: &str) -> Option<&Assistant> {
        let data = self.data.as_deref()?;
        let organization = data.governance.as_ref().ok().and_then(Option::as_deref);
        let source = if self.organization { organization? } else { data.mine.as_slice() };
        source.iter().find(|employee| employee.transfer_id() == Some(id))
    }

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
        ListEvent::Chat(id) => {
            let selection = state.data.as_ref().and_then(|data| data.mine.iter()
                .find(|employee| employee.chat_id() == Some(id.as_str())))
                .map(|employee| super::chat::Selection::private(id, employee.name.clone()));
            return super::chat::select(app, super::Page::Assistants, selection);
        }
        ListEvent::OpenTransfer(id) => return open_transfer(app, id),
        ListEvent::CloseTransfer => {
            if state.transfer.as_ref().is_some_and(|transfer| !transfer.submitting) {
                state.transfer = None;
            }
        }
        ListEvent::TransferMembers(generation, result) if generation == state.generation => {
            if let Some(transfer) = &mut state.transfer {
                match result {
                    Ok(recipients) => transfer.recipients = Some(recipients),
                    Err(error) => transfer.error = Some(error),
                }
            }
        }
        ListEvent::TransferSearch(search) => {
            if let Some(transfer) = state.transfer.as_mut().filter(|transfer| !transfer.submitting)
            {
                transfer.search = search;
            }
        }
        ListEvent::TransferSelect(recipient) => {
            if let Some(transfer) = state.transfer.as_mut().filter(|transfer| !transfer.submitting)
            {
                transfer.selected = Some(recipient);
            }
        }
        ListEvent::SubmitTransfer => return submit_transfer(app),
        ListEvent::TransferFinished(generation, result) if generation == state.generation => {
            if let Some(transfer) = state.transfer.as_mut().filter(|transfer| transfer.submitting) {
                transfer.submitting = false;
                match result {
                    Ok(()) => {
                        state.transfer = None;
                        state.data = None;
                        state.page = 0;
                        return Task::batch([
                            app.show_localized_toast(crate::app::state::ToastKind::Success, "已转交使用人", vec![]),
                            refresh(app),
                        ]);
                    }
                    Err(error) => {
                        transfer.error = Some(error);
                    }
                }
            }
        }
        ListEvent::TransferMembers(..) | ListEvent::TransferFinished(..) => {}
        ListEvent::Scope(organization) if state.transfer.is_none() => {
            state.organization = organization;
            state.page = 0;
        }
        ListEvent::Search(search) if state.transfer.is_none() => {
            state.search = search;
            state.page = 0;
        }
        ListEvent::Run(filter) if state.transfer.is_none() => {
            state.filter = filter;
            state.page = 0;
        }
        ListEvent::Page(page) if state.transfer.is_none() => {
            state.page = page;
            return board::scroll_to_top();
        }
        ListEvent::Scope(_) | ListEvent::Search(_) | ListEvent::Run(_) | ListEvent::Page(_) => {}
    }
    Task::none()
}

fn open_transfer(app: &mut App, id: String) -> Task<Message> {
    let state = &mut app.digital_employees.assistants;
    if app.digital_employees.page != Page::Assistants || state.loading || state.transfer.is_some() {
        return Task::none();
    }
    let Some(employee) = state.transfer_target(&id) else {
        return Task::none();
    };
    let current_owner = employee.owner_user_id.clone();
    let Some(account) = app.account.active_account() else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    state.transfer = Some(Transfer {
        employee_id: id,
        recipients: None,
        selected: None,
        search: String::new(),
        submitting: false,
        error: None,
    });
    Task::perform(
        async move {
            let client = Client::new(credential.expose())?;
            load_recipients(&client, &current_owner).await
        },
        move |result| message(ListEvent::TransferMembers(generation, result)),
    )
}

fn submit_transfer(app: &mut App) -> Task<Message> {
    if app.digital_employees.page != Page::Assistants {
        return Task::none();
    }
    let state = &mut app.digital_employees.assistants;
    let Some(employee) = state.transfer_employee() else {
        return Task::none();
    };
    let Some(version) = employee.version else {
        return Task::none();
    };
    let Some(transfer) = state.transfer.as_mut().filter(|transfer| !transfer.submitting) else {
        return Task::none();
    };
    let Some(recipient) = transfer.selected.as_ref().filter(|recipient| {
        transfer.recipients.as_ref().is_some_and(|recipients| recipients.contains(recipient))
    }) else {
        return Task::none();
    };
    let Some(account) = app.account.active_account() else {
        return Task::none();
    };
    let credential = account.skill_hub_credential();
    let employee_id = transfer.employee_id.clone();
    let recipient_id = recipient.id.clone();
    let generation = state.generation;
    transfer.submitting = true;
    transfer.error = None;
    Task::perform(
        async move {
            Client::new(credential.expose())?
                .post::<serde_json::Value>(
                    &["employees", &employee_id, "owner", "transfer"],
                    &serde_json::json!({ "owner_user_id": recipient_id, "expected_version": version }),
                    None,
                )
                .await
                .map(|_| ())
        },
        move |result| message(ListEvent::TransferFinished(generation, result)),
    )
}

async fn load_recipients(client: &Client, owner: &str) -> Result<Vec<Recipient>, desktop_i18n::workflow::Feedback> {
    let mut page = 1;
    let mut scanned = 0;
    let mut recipients = Vec::new();
    let mut seen = HashSet::new();
    loop {
        let result = load_user_page(client, page).await?;
        if result.list.is_empty() {
            break;
        }
        scanned += result.list.len();
        recipients.extend(result.list.into_iter().filter_map(|member| {
            let name = member.name.trim();
            if name.is_empty()
                || member.user_id.is_empty()
                || member.user_id == owner
                || !seen.insert(member.user_id.clone())
            {
                return None;
            }
            let label = match member.account_name.as_deref().map(str::trim) {
                Some(account) if !account.is_empty() && account != name => {
                    format!("{name}（{account}）")
                }
                _ => name.to_owned(),
            };
            Some(Recipient { id: member.user_id, label })
        }));
        if result.total.is_none_or(|total| scanned >= total) {
            break;
        }
        page += 1;
    }
    recipients.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(recipients)
}

pub(super) fn refresh(app: &mut App) -> Task<Message> {
    let state = &mut app.digital_employees.assistants;
    if state.loading || state.transfer.is_some() {
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

async fn load(credential: &str) -> Result<Snapshot, desktop_i18n::workflow::Feedback> {
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

async fn load_governance(client: &Client) -> Result<Option<Response>, desktop_i18n::workflow::Feedback> {
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

async fn load_user_page(client: &Client, page: usize) -> Result<IamPage<Member>, desktop_i18n::workflow::Feedback> {
    client.iam("users", &[("page", &page.to_string()), ("page_size", "100")]).await
}

pub(super) async fn load_users(client: &Client, mut wanted: HashSet<&str>) -> Vec<Member> {
    let mut page = 1;
    let mut scanned = 0;
    let mut users = Vec::new();
    while let Ok(result) = load_user_page(client, page).await {
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
