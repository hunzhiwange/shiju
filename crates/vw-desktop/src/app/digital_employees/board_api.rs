use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use super::api::{Client, Summary, nullable_list};

pub(crate) struct Snapshot {
    pub(super) summary: Summary,
    pub(super) tasks: Vec<WorkItem>,
    pub(super) profiles: HashMap<String, Profile>,
    pub(super) workflows: HashMap<String, String>,
}

#[derive(Deserialize)]
pub(super) struct Profile {
    employee_id: String,
    pub role_title: Option<String>,
    pub manager_name: Option<String>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
struct Directory {
    #[serde(default, deserialize_with = "nullable_list")]
    employees: Vec<Profile>,
}

#[derive(Deserialize)]
struct Workflows {
    #[serde(default, deserialize_with = "nullable_list")]
    workflows: Vec<Workflow>,
}

#[derive(Deserialize)]
struct Workflow {
    workflow_id: String,
    name: String,
}

#[derive(Deserialize)]
struct TaskPage {
    #[serde(default, deserialize_with = "nullable_list")]
    tasks: Vec<WorkItem>,
    total: Option<usize>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct WorkItem {
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub status_label: Option<String>,
    pub phase: Option<String>,
    pub control_state: Option<String>,
    pub owner_ref: Option<String>,
    pub current_step: Option<String>,
    pub blocker: Option<String>,
    pub progress_pct: Option<f32>,
    #[serde(default)]
    pub progress_estimated: bool,
    pub updated_at: Option<String>,
    pub input: Option<serde_json::Value>,
}

impl WorkItem {
    pub fn waiting(&self) -> bool {
        matches!(self.status.as_str(), "WAITING_APPROVAL" | "WAITING_INPUT" | "HUMAN_TAKEOVER")
    }

    pub fn running(&self) -> bool {
        self.status == "RUNNING"
    }

    pub fn paused(&self) -> bool {
        self.running() && self.control_state.as_deref() == Some("PAUSE")
    }

    pub fn progress(&self) -> Option<f32> {
        self.progress_pct.filter(|pct| (0.0..=100.0).contains(pct))
    }

    pub fn input_text(&self, key: &str) -> Option<&str> {
        self.input.as_ref()?.get(key)?.as_str().filter(|value| !value.trim().is_empty())
    }

    pub fn step_employee<'a>(&'a self, step: &'a Step) -> Option<&'a str> {
        let actor = step.actor.as_deref().filter(|actor| !actor.is_empty())?;
        self.input
            .as_ref()
            .and_then(|input| input.get("specialists")?.get(actor)?.as_str())
            .filter(|id| !id.is_empty())
            .or(Some(actor))
    }
}

#[derive(Deserialize)]
pub(super) struct Detail {
    #[serde(default, deserialize_with = "nullable_list")]
    pub steps: Vec<Step>,
}

#[derive(Deserialize)]
pub(super) struct Step {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub status: String,
    pub actor: Option<String>,
    pub actor_name: Option<String>,
    pub(super) summary: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Collaboration {
    #[serde(default, deserialize_with = "nullable_list")]
    pub entries: Vec<CollaborationEntry>,
    pub record_complete: bool,
}

#[derive(Deserialize)]
pub(super) struct CollaborationEntry {
    pub kind: String,
    pub occurred_at: Option<String>,
    pub(super) summary: Option<String>,
    pub actor: Actor,
    #[serde(default, deserialize_with = "nullable_list")]
    pub mentions: Vec<Mention>,
}

#[derive(Deserialize)]
pub(super) struct Actor {
    pub label: String,
    #[serde(rename = "ref")]
    pub employee_ref: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Mention {
    pub employee_ref: String,
}

#[derive(Deserialize)]
pub(super) struct Handoff {
    #[serde(default, deserialize_with = "nullable_list")]
    pub inputs: Vec<HandoffNote>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub outputs: Vec<Output>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub reports: Vec<HandoffNote>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub reviews: Vec<Review>,
    pub record_complete: bool,
}

#[derive(Deserialize)]
pub(super) struct HandoffNote {
    pub summary: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Output {
    pub name: Option<String>,
    pub producer: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Review {
    pub outcome: String,
    pub summary: Option<String>,
}

pub(super) async fn load(credential: &str) -> Result<Snapshot, desktop_i18n::workflow::Feedback> {
    let client = Client::new(credential)?;
    let (summary, tasks, directory, workflows) = tokio::join!(
        client.get::<Summary>("workbench/summary", &[]),
        load_tasks(&client),
        client.get::<Directory>("employees", &[("limit", "100")]),
        client.get::<Workflows>("workflows", &[("limit", "100")]),
    );
    Ok(Snapshot {
        summary: summary?,
        tasks: tasks?,
        profiles: directory
            .ok()
            .into_iter()
            .flat_map(|page| page.employees)
            .map(|profile| (profile.employee_id.clone(), profile))
            .collect(),
        workflows: workflows
            .ok()
            .into_iter()
            .flat_map(|page| page.workflows)
            .map(|workflow| (workflow.workflow_id, workflow.name))
            .collect(),
    })
}

pub(super) async fn load_tasks(client: &Client) -> Result<Vec<WorkItem>, desktop_i18n::workflow::Feedback> {
    let mut tasks = Vec::new();
    let mut ids = HashSet::new();
    let mut cursors = HashSet::new();
    let mut cursor: Option<String> = None;
    let mut offset = 0_usize;
    loop {
        let offset_param = offset.to_string();
        let pagination =
            cursor.as_deref().map_or(("offset", offset_param.as_str()), |id| ("after_id", id));
        let page: TaskPage = client.get("tasks", &[("limit", "100"), pagination]).await?;
        let length = page.tasks.len();
        let previous_count = tasks.len();
        tasks.extend(page.tasks.into_iter().filter(|task| ids.insert(task.task_id.clone())));

        if let Some(next) = page.next_cursor.filter(|id| !id.is_empty()) {
            if !cursors.insert(next.clone()) {
                return Err("任务分页游标重复，请刷新重试".into());
            }
            cursor = Some(next);
            continue;
        }
        offset += length;
        if cursor.is_some() || length == 0 || offset >= page.total.unwrap_or(offset) {
            return Ok(tasks);
        }
        if tasks.len() == previous_count {
            return Err("任务分页没有返回新记录，请刷新重试".into());
        }
    }
}
