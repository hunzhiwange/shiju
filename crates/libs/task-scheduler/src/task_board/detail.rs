use super::{
    AgentSchedulerStorageContext, AgentTaskBoardService, AgentTaskBoardUpdate, StoredAgentTask,
    TaskId, TimestampMs, object_metadata,
};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use vw_api_agent_scheduler::{
    AgentSchedulerTaskCollaborationDto, AgentSchedulerTaskCommentDto,
    AgentSchedulerTaskDetailChange, AgentSchedulerTaskDetailDto,
};

pub const COLLABORATION_KEY: &str = "task_board.collaboration";
pub const CREATOR_KEY: &str = "task_board.created_by";

pub(super) fn validate_metadata_key(key: &str) -> Result<()> {
    ensure!(key != COLLABORATION_KEY && key != CREATOR_KEY, "reserved task detail metadata key");
    Ok(())
}

impl AgentTaskBoardService<'_> {
    pub fn task_detail(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        id: &TaskId,
    ) -> Result<Option<AgentSchedulerTaskDetailDto>> {
        let task = self.store.load_task(context, id)?.filter(|task| !deleted(task));
        task.as_ref().map(project_detail).transpose()
    }

    pub fn change_task_detail(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        id: &TaskId,
        author: &str,
        change: AgentSchedulerTaskDetailChange,
        now: TimestampMs,
    ) -> Result<Option<AgentSchedulerTaskDetailDto>> {
        let changed =
            self.store.edit_task_board_task(context, id, Default::default(), |task| {
                ensure!(!deleted(task), "task has been deleted");
                let mut metadata = object_metadata(task.metadata.clone())?;
                apply_change(&mut metadata, author, change, now)?;
                Ok(AgentTaskBoardUpdate {
                    title: None,
                    status: None,
                    metadata: Value::Object(metadata),
                    updated_at_ms: TimestampMs(now.0.max(task.updated_at_ms.0)),
                })
            })?;
        changed.as_ref().map(|(_, task)| project_detail(task)).transpose()
    }
}

fn deleted(task: &StoredAgentTask) -> bool {
    task.metadata.get("task_board.deleted").and_then(Value::as_bool).unwrap_or(false)
}

fn project_detail(task: &StoredAgentTask) -> Result<AgentSchedulerTaskDetailDto> {
    Ok(AgentSchedulerTaskDetailDto {
        created_at_ms: task.created_at_ms,
        updated_at_ms: task.updated_at_ms,
        created_by: task.metadata.get(CREATOR_KEY).and_then(Value::as_str).map(str::to_owned),
        assignee: task
            .metadata
            .get("assignee")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| task.assigned_to.as_ref().map(|assignee| assignee.as_str().to_owned())),
        priority: task
            .metadata
            .get("priority")
            .and_then(Value::as_u64)
            .and_then(|priority| u32::try_from(priority).ok()),
        collaboration: read_collaboration(task.metadata.get(COLLABORATION_KEY))?,
    })
}

fn read_collaboration(value: Option<&Value>) -> Result<AgentSchedulerTaskCollaborationDto> {
    Ok(value
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .context("invalid task collaboration data")?
        .unwrap_or_default())
}

fn apply_change(
    metadata: &mut serde_json::Map<String, Value>,
    author: &str,
    change: AgentSchedulerTaskDetailChange,
    now: TimestampMs,
) -> Result<()> {
    let mut detail = read_collaboration(metadata.get(COLLABORATION_KEY))?;
    match change {
        AgentSchedulerTaskDetailChange::SetAssignee { assignee } => {
            let assignee = assignee.trim();
            ensure!(
                assignee.chars().count() <= 120 && !assignee.chars().any(char::is_control),
                "invalid task assignee"
            );
            metadata.insert("assignee".to_owned(), json!(assignee));
            return Ok(());
        }
        AgentSchedulerTaskDetailChange::SetPriority { priority } => {
            metadata.insert("priority".to_owned(), json!(priority));
            return Ok(());
        }
        AgentSchedulerTaskDetailChange::Comment { id, reply_to, body } => {
            ensure!(
                !id.is_empty()
                    && id.len() <= 80
                    && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-'),
                "invalid comment ID"
            );
            let body = body.trim();
            ensure!(
                !body.is_empty() && body.chars().count() <= 4000,
                "comment must contain 1 to 4000 characters"
            );
            if let Some(existing) = detail.comments.iter().find(|comment| comment.id == id) {
                ensure!(
                    existing.author == author
                        && existing.body == body
                        && existing.reply_to == reply_to,
                    "comment ID already exists"
                );
                return Ok(());
            }
            if let Some(parent) = &reply_to {
                ensure!(
                    detail
                        .comments
                        .iter()
                        .any(|comment| &comment.id == parent && comment.reply_to.is_none()),
                    "reply must refer to a comment on this task"
                );
            }
            ensure!(detail.comments.len() < 1000, "task comment limit reached");
            detail.comments.push(AgentSchedulerTaskCommentDto {
                id,
                reply_to,
                author: author.to_owned(),
                body: body.to_owned(),
                created_at_ms: now,
            });
        }
        AgentSchedulerTaskDetailChange::SetStartDate { date } => {
            detail.start_date = validate_date(date)?
        }
        AgentSchedulerTaskDetailChange::SetDueDate { date } => {
            detail.due_date = validate_date(date)?
        }
        AgentSchedulerTaskDetailChange::LinkPullRequest { url } => {
            let url = parse_pull_request(&url)?;
            if !detail.pull_requests.contains(&url) {
                ensure!(detail.pull_requests.len() < 100, "task pull request limit reached");
                detail.pull_requests.push(url);
            }
        }
        AgentSchedulerTaskDetailChange::UnlinkPullRequest { url } => {
            detail.pull_requests.retain(|link| link != &url);
        }
    }
    if let (Some(start), Some(due)) = (&detail.start_date, &detail.due_date)
        && start > due
    {
        bail!("task start date must be on or before its due date");
    }
    metadata.insert(COLLABORATION_KEY.to_owned(), serde_json::to_value(detail)?);
    Ok(())
}

fn validate_date(date: Option<String>) -> Result<Option<String>> {
    if let Some(date) = &date {
        let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .context("expected a date in YYYY-MM-DD format")?;
        ensure!(
            date.len() == 10 && !date.starts_with("0000") && parsed.to_string() == *date,
            "expected a date in YYYY-MM-DD format"
        );
    }
    Ok(date)
}

fn parse_pull_request(value: &str) -> Result<String> {
    let value = value.trim();
    ensure!(value.len() <= 2048, "pull request URL is too long");
    let mut url = url::Url::parse(value).context("expected an HTTPS pull request URL")?;
    ensure!(
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none(),
        "expected an HTTPS pull request URL without credentials"
    );
    let mut parts = url.path().trim_end_matches('/').rsplit('/');
    let number = parts.next().and_then(|part| part.parse::<u64>().ok());
    let kind = parts.next();
    ensure!(
        number.is_some_and(|number| number > 0)
            && matches!(
                kind,
                Some("pull" | "pulls" | "pull-requests" | "merge_requests" | "pullrequest")
            )
            && parts.filter(|part| !part.is_empty()).count() >= 2,
        "expected a pull request or merge request URL"
    );
    let path = url.path().trim_end_matches('/').to_owned();
    url.set_path(&path);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}
