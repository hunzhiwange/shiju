//! 解析周期任务时间并判断执行、等待或跳过，同时维护最近和下次执行时间。

use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration as ChronoDuration, SecondsFormat, Utc};
use vw_api_types::common::TimestampMs;

use crate::cron::{Schedule, next_run_for_schedule};

use super::super::domain::TaskId;
use super::super::state_machine::AgentTaskKind;
use super::super::store::AgentSchedulerStore;
use super::super::store::tasks::StoredAgentTask;

pub const RECURRING_OVERDUE_GRACE_MINUTES: i64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringSchedule {
    pub expression: String,
    pub timezone: Option<String>,
    pub next_scheduled_at: DateTime<Utc>,
    pub last_executed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurringDecision {
    Trigger { scheduled_at: DateTime<Utc>, overdue_at: DateTime<Utc> },
    Wait { reason: RecurringWaitReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurringWaitReason {
    AlreadyExecuted,
    NotPastGrace { overdue_at: DateTime<Utc> },
}

impl RecurringSchedule {
    pub fn from_task(task: &StoredAgentTask) -> Result<Self> {
        if task.kind != AgentTaskKind::Infinite {
            bail!("recurring schedule requires infinite task: {}", task.id.as_str());
        }

        let expression = task
            .schedule
            .clone()
            .with_context(|| format!("infinite task {} is missing schedule", task.id.as_str()))?;
        let next_scheduled_at = parse_task_timestamp(
            "next_scheduled_at",
            task.next_scheduled_at.as_deref().with_context(|| {
                format!("infinite task {} is missing next_scheduled_at", task.id.as_str())
            })?,
        )?;
        let last_executed_at = task
            .last_executed_at
            .as_deref()
            .map(|value| parse_task_timestamp("last_executed_at", value))
            .transpose()?;

        Ok(Self {
            expression,
            timezone: task.timezone.clone(),
            next_scheduled_at,
            last_executed_at,
        })
    }

    pub fn decision(&self, now: DateTime<Utc>) -> RecurringDecision {
        if self.last_executed_at.as_ref().is_some_and(|last| last >= &self.next_scheduled_at) {
            return RecurringDecision::Wait { reason: RecurringWaitReason::AlreadyExecuted };
        }

        let overdue_at =
            self.next_scheduled_at + ChronoDuration::minutes(RECURRING_OVERDUE_GRACE_MINUTES);
        if now > overdue_at {
            return RecurringDecision::Trigger { scheduled_at: self.next_scheduled_at, overdue_at };
        }

        RecurringDecision::Wait { reason: RecurringWaitReason::NotPastGrace { overdue_at } }
    }

    pub fn record_executed(&self, executed_at: DateTime<Utc>) -> Result<Self> {
        let next_scheduled_at = next_run_for_schedule(
            &Schedule::Cron { expr: self.expression.clone(), tz: self.timezone.clone() },
            executed_at,
        )
        .with_context(|| format!("compute next recurring schedule for {}", self.expression))?;

        Ok(Self {
            expression: self.expression.clone(),
            timezone: self.timezone.clone(),
            next_scheduled_at,
            last_executed_at: Some(executed_at),
        })
    }
}

impl RecurringDecision {
    pub fn should_trigger(&self) -> bool {
        matches!(self, Self::Trigger { .. })
    }
}

pub fn record_executed(
    store: &AgentSchedulerStore,
    project_path: &Path,
    task_id: &TaskId,
    executed_at: DateTime<Utc>,
) -> Result<Option<StoredAgentTask>> {
    let Some(task) = store.load_task(project_path, task_id)? else {
        return Ok(None);
    };
    let updated_schedule = RecurringSchedule::from_task(&task)?.record_executed(executed_at)?;
    let last_executed_at = format_task_timestamp(executed_at);
    let next_scheduled_at = format_task_timestamp(updated_schedule.next_scheduled_at);

    store.update_recurring_task_execution(
        project_path,
        task_id,
        &last_executed_at,
        &next_scheduled_at,
        TimestampMs(executed_at.timestamp_millis()),
    )
}

fn parse_task_timestamp(field_name: &str, value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("parse {field_name}: {value}"))?
        .with_timezone(&Utc))
}

fn format_task_timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}
