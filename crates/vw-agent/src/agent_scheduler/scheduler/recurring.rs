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

/// 周期任务超过计划时间后仍继续等待的宽限分钟数。
pub const RECURRING_OVERDUE_GRACE_MINUTES: i64 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 从无限任务字段解析出的有效周期计划。
pub struct RecurringSchedule {
    pub expression: String,
    pub timezone: Option<String>,
    pub next_scheduled_at: DateTime<Utc>,
    pub last_executed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 在给定时刻对周期任务作出的触发或等待决定。
pub enum RecurringDecision {
    Trigger { scheduled_at: DateTime<Utc>, overdue_at: DateTime<Utc> },
    Wait { reason: RecurringWaitReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 周期任务暂不触发的可区分原因。
pub enum RecurringWaitReason {
    AlreadyExecuted,
    NotPastGrace { overdue_at: DateTime<Utc> },
}

impl RecurringSchedule {
    /// 从无限任务构建计划；任务种类、表达式或时间字段无效时返回错误。
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

    /// 基于当前时间判断是否已越过宽限期，且避免同一计划时刻重复执行。
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

    /// 记录本次执行时间，并从实际执行时刻计算下一次计划时间。
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
    /// 仅当决定为触发时返回 `true`。
    pub fn should_trigger(&self) -> bool {
        matches!(self, Self::Trigger { .. })
    }
}

/// 原子更新任务的最近执行时间和下一计划时间；任务不存在时返回 `Ok(None)`。
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
