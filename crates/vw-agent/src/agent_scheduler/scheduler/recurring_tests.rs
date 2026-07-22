use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::json;
use vw_api_types::common::TimestampMs;

use super::recurring::{
    RecurringDecision, RecurringSchedule, RecurringWaitReason, record_executed,
};
use crate::agent_scheduler::domain::{TaskId, WorkerName};
use crate::agent_scheduler::state_machine::{AgentTaskKind, AgentTaskStatus};
use crate::agent_scheduler::store::AgentSchedulerStore;
use crate::agent_scheduler::store::tasks::{NewAgentTask, StoredAgentTask};

#[test]
fn agent_recurring_scheduler_matches_infinite_task_heartbeat_rules() -> Result<()> {
    let now = timestamp("2026-07-02T10:46:00Z")?;
    let next_scheduled_at = timestamp("2026-07-02T10:15:00Z")?;

    let never_executed = schedule(None, next_scheduled_at);
    assert_eq!(
        never_executed.decision(now),
        RecurringDecision::Trigger {
            scheduled_at: next_scheduled_at,
            overdue_at: timestamp("2026-07-02T10:45:00Z")?,
        }
    );

    let already_executed = schedule(Some(next_scheduled_at), next_scheduled_at);
    assert_eq!(
        already_executed.decision(now),
        RecurringDecision::Wait { reason: RecurringWaitReason::AlreadyExecuted }
    );

    let overdue = schedule(Some(timestamp("2026-07-02T10:00:00Z")?), next_scheduled_at);
    assert!(overdue.decision(now).should_trigger());

    let within_grace = schedule(None, timestamp("2026-07-02T10:30:00Z")?);
    assert_eq!(
        within_grace.decision(now),
        RecurringDecision::Wait {
            reason: RecurringWaitReason::NotPastGrace {
                overdue_at: timestamp("2026-07-02T11:00:00Z")?,
            },
        }
    );

    let temp_dir = tempfile::tempdir()?;
    let project_path = temp_dir.path();
    let store = AgentSchedulerStore::new();
    store.initialize_project(project_path)?;
    store.create_task(project_path, infinite_task()?)?;

    let updated =
        record_executed(&store, project_path, &TaskId::new("task-infinite-recurring")?, now)?
            .expect("recurring task exists");

    assert_eq!(updated.last_executed_at.as_deref(), Some("2026-07-02T10:46:00Z"));
    assert_eq!(updated.next_scheduled_at.as_deref(), Some("2026-07-02T11:00:00Z"));
    assert!(!RecurringSchedule::from_task(&updated)?.decision(now).should_trigger());

    Ok(())
}

#[test]
fn recurring_schedule_rejects_invalid_task_shape_and_missing_record_is_ignored() -> Result<()> {
    let mut task = stored_recurring_task()?;
    task.kind = AgentTaskKind::Finite;
    let kind_error = RecurringSchedule::from_task(&task).unwrap_err().to_string();
    assert!(kind_error.contains("requires infinite task"));

    task.kind = AgentTaskKind::Infinite;
    task.schedule = None;
    let schedule_error = RecurringSchedule::from_task(&task).unwrap_err().to_string();
    assert!(schedule_error.contains("missing schedule"));

    task.schedule = Some("*/15 * * * *".to_string());
    task.next_scheduled_at = None;
    let next_error = RecurringSchedule::from_task(&task).unwrap_err().to_string();
    assert!(next_error.contains("missing next_scheduled_at"));

    task.next_scheduled_at = Some("not-a-timestamp".to_string());
    let timestamp_error = RecurringSchedule::from_task(&task).unwrap_err().to_string();
    assert!(timestamp_error.contains("parse next_scheduled_at: not-a-timestamp"));

    let temp_dir = tempfile::tempdir()?;
    let store = AgentSchedulerStore::new();
    store.initialize_project(temp_dir.path())?;
    assert!(
        record_executed(
            &store,
            temp_dir.path(),
            &TaskId::new("missing-recurring-task")?,
            timestamp("2026-07-02T10:46:00Z")?,
        )?
        .is_none()
    );

    Ok(())
}

fn schedule(
    last_executed_at: Option<DateTime<Utc>>,
    next_scheduled_at: DateTime<Utc>,
) -> RecurringSchedule {
    RecurringSchedule {
        expression: "*/15 * * * *".to_string(),
        timezone: Some("UTC".to_string()),
        next_scheduled_at,
        last_executed_at,
    }
}

fn infinite_task() -> Result<NewAgentTask> {
    Ok(NewAgentTask {
        id: TaskId::new("task-infinite-recurring")?,
        title: "Monitor recurring task".to_string(),
        kind: AgentTaskKind::Infinite,
        status: AgentTaskStatus::Active,
        spec_uri: "shared/tasks/task-infinite-recurring/spec.md".to_string(),
        meta_uri: "shared/tasks/task-infinite-recurring/meta.json".to_string(),
        assigned_to: Some(WorkerName::new("worker-monitor")?),
        team_id: None,
        room_id: Some("!monitor-room:example.test".to_string()),
        project_room_id: None,
        delegated_to_team: None,
        schedule: Some("*/15 * * * *".to_string()),
        timezone: Some("UTC".to_string()),
        next_scheduled_at: Some("2026-07-02T10:15:00Z".to_string()),
        last_executed_at: None,
        dag: Vec::new(),
        metadata: json!({"source": "test"}),
        created_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn stored_recurring_task() -> Result<StoredAgentTask> {
    Ok(StoredAgentTask {
        id: TaskId::new("stored-recurring-task")?,
        title: "Stored recurring task".to_string(),
        kind: AgentTaskKind::Infinite,
        status: AgentTaskStatus::Active,
        spec_uri: "shared/tasks/stored-recurring-task/spec.md".to_string(),
        meta_uri: "shared/tasks/stored-recurring-task/meta.json".to_string(),
        assigned_to: None,
        team_id: None,
        room_id: None,
        project_room_id: None,
        delegated_to_team: None,
        schedule: Some("*/15 * * * *".to_string()),
        timezone: Some("UTC".to_string()),
        next_scheduled_at: Some("2026-07-02T10:15:00Z".to_string()),
        last_executed_at: None,
        dag: Vec::new(),
        metadata: serde_json::Value::Null,
        created_at_ms: TimestampMs(1_783_000_100_000),
        updated_at_ms: TimestampMs(1_783_000_100_000),
    })
}

fn timestamp(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
