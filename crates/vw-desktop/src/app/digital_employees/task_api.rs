use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use serde::Deserialize;
use serde_json::{Value, json};

use super::api::{Client, nullable_list};
use super::assistants::{IamPage, load_users};
use super::board_api::{Collaboration, Handoff, Step};
use super::task_schedule::TIMEZONE;
use super::tasks::{Draft, Filters, Trigger};

pub(super) const PAGE_SIZE: usize = 20;

#[derive(Clone, Deserialize)]
pub(super) struct Work {
    pub(super) task_id: String,
    pub(super) title: String,
    pub(super) status: String,
    pub(super) status_label: Option<String>,
    pub(super) current_step: Option<String>,
    pub(super) owner_ref: Option<String>,
    pub(super) progress_pct: Option<f32>,
    #[serde(default)]
    pub(super) progress_estimated: bool,
    pub(super) department_path: Option<String>,
    pub(super) created_by: Option<String>,
    pub(super) created_at: Option<String>,
    pub(super) updated_at: Option<String>,
    pub(super) trigger_type: Option<String>,
    pub(super) trigger_label: Option<String>,
    pub(super) once_at: Option<String>,
    pub(super) cron: Option<String>,
    pub(super) next_fire_at: Option<String>,
    pub(super) schedule_id: Option<String>,
    pub(super) blocker: Option<String>,
    pub(super) assigned_at: Option<String>,
    pub(super) acknowledged_at: Option<String>,
    pub(super) started_at: Option<String>,
    pub(super) submitted_at: Option<String>,
    pub(super) acknowledgement_latency_ms: Option<u64>,
    pub(super) input: Option<Value>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) allowed_actions: Vec<String>,
}

impl Work {
    pub(super) fn input_text(&self, key: &str) -> Option<&str> {
        self.input.as_ref()?.get(key)?.as_str().filter(|value| !value.is_empty())
    }

    pub(super) fn allows(&self, action: &str) -> bool {
        self.allowed_actions.iter().any(|value| value == action)
    }
}

#[derive(Deserialize)]
pub(crate) struct WorkPage {
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) tasks: Vec<Work>,
    pub(super) total: Option<usize>,
}

#[derive(Deserialize)]
pub(crate) struct Counts {
    pub(super) total: usize,
    counts: HashMap<String, usize>,
}

impl Counts {
    pub(super) fn sum(&self, statuses: &[&str]) -> usize {
        statuses.iter().filter_map(|status| self.counts.get(*status)).sum()
    }
}

#[derive(Clone, Deserialize)]
pub(super) struct Employee {
    pub(super) employee_id: String,
    pub(super) name: String,
    pub(super) agent_kind: Option<String>,
    visibility: Option<String>,
    status: Option<String>,
    publish_status: Option<String>,
    active_version_id: Option<String>,
    runtime_status: Option<String>,
}

impl Employee {
    pub(super) fn can_dispatch(&self) -> bool {
        self.agent_kind.as_deref().is_none_or(|kind| kind == "manager")
            && self.visibility.as_deref() != Some("private")
            && self.status.as_deref() == Some("ACTIVE")
            && self.publish_status.as_deref() == Some("PUBLISHED")
            && self.runtime_status.as_deref() == Some("on_duty")
            && self.active_version_id.as_deref().is_some_and(|id| !id.is_empty())
    }
}

#[derive(Clone, Deserialize)]
pub(super) struct Workflow {
    pub(super) workflow_id: String,
    pub(super) name: String,
    availability: Option<String>,
    published: Option<Published>,
}

#[derive(Clone, Deserialize)]
struct Published {
    version_id: String,
}

impl Workflow {
    pub(super) fn version(&self) -> Option<&str> {
        if self.availability.as_deref() == Some("OFFLINE") {
            return None;
        }
        self.published.as_ref().map(|version| version.version_id.as_str())
    }
}

#[derive(Default)]
pub(crate) struct Directory {
    pub(super) employees: Vec<Employee>,
    pub(super) workflows: Vec<Workflow>,
}

#[derive(Deserialize)]
struct Employees {
    #[serde(default, deserialize_with = "nullable_list")]
    employees: Vec<Employee>,
    total: Option<usize>,
}

#[derive(Deserialize)]
struct Workflows {
    #[serde(default, deserialize_with = "nullable_list")]
    workflows: Vec<Workflow>,
    total: Option<usize>,
}

pub(crate) struct List {
    pub(super) page: WorkPage,
    pub(super) counts: Result<Counts, desktop_i18n::workflow::Feedback>,
    pub(super) paused_schedules: HashSet<String>,
}

pub(super) async fn list(key: &str, filters: Filters, page: usize) -> Result<List, desktop_i18n::workflow::Feedback> {
    let client = Client::new(key)?;
    let mut query = vec![
        ("q", filters.keyword.trim()),
        ("department_path_prefix", filters.department.as_str()),
        ("trigger_type", filters.trigger.map_or("", Trigger::key)),
        ("employee_id", filters.employee.as_str()),
        ("workflow_id", filters.workflow.as_str()),
    ];
    query.retain(|(_, value)| !value.is_empty());
    let mut page_query = query.clone();
    page_query.push(("sort", filters.sort.key()));
    let offset = page.saturating_mul(PAGE_SIZE).to_string();
    page_query.extend([("limit", "20"), ("offset", offset.as_str())]);
    let statuses = filters.phase.statuses().join(",");
    if !statuses.is_empty() {
        page_query.push(("status", &statuses));
    }
    let (page, counts) =
        tokio::join!(client.get("tasks", &page_query), client.get("tasks/count", &query),);
    let page: WorkPage = page?;
    let schedule_ids: HashSet<_> = page
        .tasks
        .iter()
        .filter(|work| work.allows("cancel_schedule") && !work.allows("cancel"))
        .filter_map(|work| work.schedule_id.as_deref())
        .collect();
    let paused_schedules = iced::futures::future::join_all(schedule_ids.into_iter().map(|id| {
        let client = &client;
        async move {
            let response = tokio::time::timeout(
                Duration::from_secs(3),
                client.get_path::<ScheduleResponse>(&["schedules", id], &[]),
            )
            .await
            .ok()?
            .ok()?;
            (response.schedule.control_state.as_deref() == Some("PAUSE")).then(|| id.to_owned())
        }
    }))
    .await
    .into_iter()
    .flatten()
    .collect();
    Ok(List { page, counts, paused_schedules })
}

pub(super) async fn directory(key: &str) -> Result<Directory, desktop_i18n::workflow::Feedback> {
    let client = Client::new(key)?;
    let (employees, workflows) = tokio::join!(
        async {
            let mut all = Vec::new();
            let mut seen = HashSet::new();
            loop {
                let offset = all.len().to_string();
                let page: Employees =
                    client.get("employees", &[("limit", "100"), ("offset", &offset)]).await?;
                let empty = page.employees.is_empty();
                if page.employees.iter().any(|employee| !seen.insert(employee.employee_id.clone()))
                {
                    return Err("员工目录返回了重复记录，请刷新重试".into());
                }
                all.extend(page.employees);
                if empty || all.len() >= page.total.unwrap_or(all.len()) {
                    return Ok::<_, desktop_i18n::workflow::Feedback>(all);
                }
            }
        },
        async {
            let mut all = Vec::new();
            let mut seen = HashSet::new();
            loop {
                let offset = all.len().to_string();
                let page: Workflows =
                    client.get("workflows", &[("limit", "100"), ("offset", &offset)]).await?;
                let empty = page.workflows.is_empty();
                if page.workflows.iter().any(|workflow| !seen.insert(workflow.workflow_id.clone()))
                {
                    return Err("流程目录返回了重复记录，请刷新重试".into());
                }
                all.extend(page.workflows);
                if empty || all.len() >= page.total.unwrap_or(all.len()) {
                    return Ok::<_, desktop_i18n::workflow::Feedback>(all);
                }
            }
        },
    );
    Ok(Directory { employees: employees?, workflows: workflows? })
}

#[derive(Deserialize)]
struct DepartmentRow {
    department_path: Option<String>,
    department_name: Option<String>,
}

pub(super) async fn departments(key: &str) -> Result<Vec<(String, String)>, desktop_i18n::workflow::Feedback> {
    let page: IamPage<DepartmentRow> = Client::new(key)?.iam("departments", &[]).await?;
    Ok(department_choices(page))
}

fn department_choices(page: IamPage<DepartmentRow>) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    page.list
        .into_iter()
        .filter_map(|row| {
            let path = row.department_path?;
            let name = row.department_name?;
            if path.trim().is_empty() || name.trim().is_empty() || !seen.insert(path.clone()) {
                return None;
            }
            Some((path, name))
        })
        .collect()
}

#[derive(Clone, Deserialize)]
pub(crate) struct Schedule {
    pub(super) schedule_id: String,
    pub(super) title: String,
    pub(super) cron: Option<String>,
    pub(super) next_fire_at: Option<String>,
    pub(super) timezone: Option<String>,
    pub(super) enabled: bool,
    pub(super) control_state: Option<String>,
    pub(super) version: Option<i64>,
    pub(super) input: Option<Value>,
}

impl Schedule {
    pub(super) fn input_text(&self, key: &str) -> Option<&str> {
        self.input.as_ref()?.get(key)?.as_str().filter(|value| !value.trim().is_empty())
    }
}

#[derive(Deserialize)]
struct ScheduleResponse {
    schedule: Schedule,
}

#[derive(Deserialize)]
pub(crate) struct Schedules {
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) schedules: Vec<Schedule>,
    pub(super) total: Option<usize>,
}

pub(super) async fn schedules(key: &str, page: usize) -> Result<Schedules, desktop_i18n::workflow::Feedback> {
    let offset = page.saturating_mul(PAGE_SIZE).to_string();
    Client::new(key)?.get("schedules", &[("limit", "20"), ("offset", &offset)]).await
}

pub(super) async fn schedule(key: &str, id: &str) -> Result<Schedule, desktop_i18n::workflow::Feedback> {
    let response: ScheduleResponse = Client::new(key)?.get_path(&["schedules", id], &[]).await?;
    Ok(response.schedule)
}

pub(super) async fn update_schedule(
    key: &str,
    id: &str,
    version: i64,
    cron: &str,
    timezone: &str,
) -> Result<Schedule, desktop_i18n::workflow::Feedback> {
    let response: ScheduleResponse = Client::new(key)?.patch_schedule(id, &json!({
        "expected_version": version,
        "cron": cron,
        "timezone": timezone,
    })).await?;
    Ok(response.schedule)
}

#[derive(Deserialize)]
pub(super) struct Detail {
    pub(super) task: Work,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) steps: Vec<Step>,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) artifacts: Vec<Artifact>,
    pub(super) runtime: Option<Runtime>,
    pub(super) deliverables: Option<Deliverables>,
}

#[derive(Deserialize)]
pub(super) struct Deliverables {
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) items: Vec<Artifact>,
    pub(super) empty_state: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Artifact {
    pub(super) name: String,
    pub(super) content_type: Option<String>,
    // The service can return both names on the same artifact.
    byte_size: Option<u64>,
    size_bytes: Option<u64>,
}

impl Artifact {
    pub(super) fn byte_size(&self) -> Option<u64> {
        self.byte_size.or(self.size_bytes)
    }
}

#[cfg(test)]
#[path = "task_api_tests.rs"]
mod tests;

#[derive(Deserialize)]
pub(super) struct Runtime {
    pub(super) backend: Option<String>,
    pub(super) ensure_status: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct LogPage {
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) events: Vec<Log>,
}

#[derive(Deserialize)]
pub(super) struct Log {
    pub(super) event_type: String,
    pub(super) occurred_at: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct TracePage {
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) entries: Vec<Trace>,
}

#[derive(Deserialize)]
pub(super) struct Trace {
    pub(super) title: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) kind: Option<String>,
    pub(super) started_at: Option<String>,
    pub(super) duration_ms: Option<u64>,
}

pub(crate) struct Record {
    pub(super) detail: Detail,
    pub(super) creator: Option<String>,
    pub(super) collaboration: Result<Collaboration, desktop_i18n::workflow::Feedback>,
    pub(super) handoff: Result<Handoff, desktop_i18n::workflow::Feedback>,
    pub(super) events: Result<LogPage, desktop_i18n::workflow::Feedback>,
    pub(super) trace: Result<TracePage, desktop_i18n::workflow::Feedback>,
}

pub(super) async fn detail(
    key: &str,
    id: &str,
    previous: Option<Arc<Record>>,
) -> Result<Record, desktop_i18n::workflow::Feedback> {
    let client = Client::new(key)?;
    let detail: Detail = client.task(id, None).await?;
    let events_path = ["tasks", id, "events"];
    let trace_path = ["tasks", id, "trace"];
    let (collaboration, handoff, events, trace, creator) = tokio::join!(
        client.task(id, Some("collaboration")),
        client.task(id, Some("handoff")),
        client.get_path(&events_path, &[("limit", "50")]),
        client.get_path(&trace_path, &[("limit", "50")]),
        async {
            let user = detail.task.created_by.as_deref()?;
            if let Some(record) =
                previous.filter(|record| record.detail.task.created_by.as_deref() == Some(user))
            {
                return record.creator.clone();
            }
            let members = tokio::time::timeout(
                Duration::from_secs(3),
                load_users(&client, HashSet::from([user])),
            )
            .await
            .ok()?;
            members.into_iter().next().map(|member| member.name)
        },
    );
    Ok(Record { detail, creator, collaboration, handoff, events, trace })
}

#[derive(Deserialize)]
pub(crate) struct BatchResult {
    pub(super) succeeded: usize,
    pub(super) noops: usize,
    pub(super) failed: usize,
    #[serde(default, deserialize_with = "nullable_list")]
    pub(super) skipped: Vec<Value>,
}

pub(super) async fn batch(
    key: &str,
    ids: Vec<String>,
    action: &str,
) -> Result<BatchResult, desktop_i18n::workflow::Feedback> {
    Client::new(key)?
        .post(&["tasks", "batch"], &json!({ "task_ids": ids, "action": action }), None)
        .await
}

pub(super) async fn retry(key: &str, id: &str) -> Result<(), desktop_i18n::workflow::Feedback> {
    Client::new(key)?.post::<Value>(&["tasks", id, "actions", "retry"], &json!({}), None).await?;
    Ok(())
}

pub(super) async fn cancel(key: &str, id: &str, reason: &str) -> Result<(), desktop_i18n::workflow::Feedback> {
    let body =
        if reason.trim().is_empty() { json!({}) } else { json!({ "reason": reason.trim() }) };
    Client::new(key)?.post::<Value>(&["tasks", &format!("{id}:cancel")], &body, None).await?;
    Ok(())
}

pub(super) async fn run_now(key: &str, id: &str) -> Result<(), desktop_i18n::workflow::Feedback> {
    Client::new(key)?.post::<Value>(&["tasks", &format!("{id}:run-now")], &json!({}), None).await?;
    Ok(())
}

pub(super) async fn schedule_command(key: &str, id: &str, action: &str) -> Result<(), desktop_i18n::workflow::Feedback> {
    Client::new(key)?
        .post::<Value>(&["schedules", id, "actions", action], &json!({}), None)
        .await?;
    Ok(())
}

#[derive(Deserialize)]
struct Preview {
    #[serde(default, deserialize_with = "nullable_list")]
    steps: Vec<Value>,
}

pub(super) async fn create(
    key: &str,
    draft: Draft,
    version: String,
    time: Option<String>,
) -> Result<(), desktop_i18n::workflow::Feedback> {
    let cron = (draft.trigger == Trigger::Cron).then(|| draft.repeat.cron());
    create_with_timing(key, draft, version, time, cron, TIMEZONE.name()).await
}

pub(super) async fn create_with_timing(
    key: &str,
    draft: Draft,
    version: String,
    time: Option<String>,
    cron: Option<String>,
    timezone: &str,
) -> Result<(), desktop_i18n::workflow::Feedback> {
    let client = Client::new(key)?;
    let preview: Preview = client
        .get_path(&["workflows", &draft.workflow, "step-preview"], &[("version", "published")])
        .await?;
    if preview.steps.is_empty() {
        return Err("流程未返回可执行步骤，请检查已发布版本".into());
    }
    let steps: Vec<_> = preview
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let name = step.as_str().or_else(|| step.get("name").and_then(Value::as_str));
            let mut output = json!({
                "seq": step.get("seq").and_then(Value::as_u64).unwrap_or(index as u64 + 1),
                "name": name.map(str::to_owned).unwrap_or_else(|| format!("步骤 {}", index + 1)),
                "status": "PENDING",
            });
            for field in ["type", "skill_ref", "summary"] {
                if let Some(value) = step.get(field).and_then(Value::as_str) {
                    output[field] = json!(value);
                }
            }
            if let Some(actor) =
                step.get("actor").or_else(|| step.get("host")).and_then(Value::as_str)
            {
                output["actor"] = json!(actor);
            }
            output
        })
        .collect();
    let mut input = draft.retained_input.unwrap_or_default();
    input.extend([
        ("employee_id".into(), json!(draft.employee)),
        ("owner_type".into(), json!("employee")),
        ("owner_ref".into(), json!(draft.employee)),
        ("workflow_id".into(), json!(draft.workflow)),
        ("workflow_version_id".into(), json!(version)),
        ("prompt".into(), json!(draft.prompt.text().trim())),
    ]);
    let mut body = json!({
        "title": draft.title.trim(),
        "trigger_type": draft.trigger.key(),
        "input": input,
        "steps": steps,
    });
    if !draft.department.trim().is_empty() {
        body["department_path"] = json!(draft.department.trim());
    }
    if let Some(time) = time {
        body["once_at"] = json!(time);
        body["timezone"] = json!(timezone);
    }
    if let Some(cron) = cron {
        body["cron"] = json!(cron);
        body["timezone"] = json!(timezone);
    }
    client.post::<Value>(&["tasks"], &body, Some(&draft.key)).await?;
    Ok(())
}
