//! 定义调度审计事件的封闭目录，以及持久化字符串与强类型事件之间的转换。

use anyhow::{Result, bail};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// 可持久化的调度审计事件种类。
pub enum AgentEventKind {
    TaskCreated,
    ImportedLegacyTask,
    TaskAssigned,
    WorkerSelected,
    WorkerAcknowledged,
    RunStarted,
    RunSubmitted,
    RunCompleted,
    RunFailed,
    RunRecovered,
    TaskBlocked,
    TaskPaused,
    TaskResumed,
    TaskCancelled,
    TaskArchived,
    InfiniteScheduled,
    InfiniteExecuted,
    TeamNodeQueued,
    TeamNodeDispatched,
    TeamNodeCompleted,
    ResourceLeased,
    ResourceReleased,
    OutboxQueued,
    OutboxDelivered,
    OutboxFailed,
    HumanIntervened,
}

impl AgentEventKind {
    /// 返回事件存储和 API 载荷使用的稳定键。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TaskCreated => "task_created",
            Self::ImportedLegacyTask => "imported_legacy_task",
            Self::TaskAssigned => "task_assigned",
            Self::WorkerSelected => "worker_selected",
            Self::WorkerAcknowledged => "worker_acknowledged",
            Self::RunStarted => "run_started",
            Self::RunSubmitted => "run_submitted",
            Self::RunCompleted => "run_completed",
            Self::RunFailed => "run_failed",
            Self::RunRecovered => "run_recovered",
            Self::TaskBlocked => "task_blocked",
            Self::TaskPaused => "task_paused",
            Self::TaskResumed => "task_resumed",
            Self::TaskCancelled => "task_cancelled",
            Self::TaskArchived => "task_archived",
            Self::InfiniteScheduled => "infinite_scheduled",
            Self::InfiniteExecuted => "infinite_executed",
            Self::TeamNodeQueued => "team_node_queued",
            Self::TeamNodeDispatched => "team_node_dispatched",
            Self::TeamNodeCompleted => "team_node_completed",
            Self::ResourceLeased => "resource_leased",
            Self::ResourceReleased => "resource_released",
            Self::OutboxQueued => "outbox_queued",
            Self::OutboxDelivered => "outbox_delivered",
            Self::OutboxFailed => "outbox_failed",
            Self::HumanIntervened => "human_intervened",
        }
    }

    /// 解析稳定事件键；未知事件不会被静默接纳。
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "task_created" => Ok(Self::TaskCreated),
            "imported_legacy_task" => Ok(Self::ImportedLegacyTask),
            "task_assigned" => Ok(Self::TaskAssigned),
            "worker_selected" => Ok(Self::WorkerSelected),
            "worker_acknowledged" => Ok(Self::WorkerAcknowledged),
            "run_started" => Ok(Self::RunStarted),
            "run_submitted" => Ok(Self::RunSubmitted),
            "run_completed" => Ok(Self::RunCompleted),
            "run_failed" => Ok(Self::RunFailed),
            "run_recovered" => Ok(Self::RunRecovered),
            "task_blocked" => Ok(Self::TaskBlocked),
            "task_paused" => Ok(Self::TaskPaused),
            "task_resumed" => Ok(Self::TaskResumed),
            "task_cancelled" => Ok(Self::TaskCancelled),
            "task_archived" => Ok(Self::TaskArchived),
            "infinite_scheduled" => Ok(Self::InfiniteScheduled),
            "infinite_executed" => Ok(Self::InfiniteExecuted),
            "team_node_queued" => Ok(Self::TeamNodeQueued),
            "team_node_dispatched" => Ok(Self::TeamNodeDispatched),
            "team_node_completed" => Ok(Self::TeamNodeCompleted),
            "resource_leased" => Ok(Self::ResourceLeased),
            "resource_released" => Ok(Self::ResourceReleased),
            "outbox_queued" => Ok(Self::OutboxQueued),
            "outbox_delivered" => Ok(Self::OutboxDelivered),
            "outbox_failed" => Ok(Self::OutboxFailed),
            "human_intervened" => Ok(Self::HumanIntervened),
            other => bail!("unsupported agent event kind: {other}"),
        }
    }

    /// 返回包含全部受支持事件种类的静态目录。
    pub fn catalog() -> &'static [Self] {
        &AGENT_EVENT_KIND_CATALOG
    }
}

pub const AGENT_EVENT_KIND_CATALOG: [AgentEventKind; 26] = [
    AgentEventKind::TaskCreated,
    AgentEventKind::ImportedLegacyTask,
    AgentEventKind::TaskAssigned,
    AgentEventKind::WorkerSelected,
    AgentEventKind::WorkerAcknowledged,
    AgentEventKind::RunStarted,
    AgentEventKind::RunSubmitted,
    AgentEventKind::RunCompleted,
    AgentEventKind::RunFailed,
    AgentEventKind::RunRecovered,
    AgentEventKind::TaskBlocked,
    AgentEventKind::TaskPaused,
    AgentEventKind::TaskResumed,
    AgentEventKind::TaskCancelled,
    AgentEventKind::TaskArchived,
    AgentEventKind::InfiniteScheduled,
    AgentEventKind::InfiniteExecuted,
    AgentEventKind::TeamNodeQueued,
    AgentEventKind::TeamNodeDispatched,
    AgentEventKind::TeamNodeCompleted,
    AgentEventKind::ResourceLeased,
    AgentEventKind::ResourceReleased,
    AgentEventKind::OutboxQueued,
    AgentEventKind::OutboxDelivered,
    AgentEventKind::OutboxFailed,
    AgentEventKind::HumanIntervened,
];

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
