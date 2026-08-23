//! 定义不同任务种类允许的状态转换，并对非法转换返回明确错误。

use anyhow::{Result, bail};

pub use super::domain::AgentTaskKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentTaskStatus {
    Draft,
    Queued,
    Assigned,
    Dispatched,
    Acknowledged,
    Active,
    Running,
    Submitted,
    Revision,
    Blocked,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Archived,
}

impl AgentTaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Queued => "queued",
            Self::Assigned => "assigned",
            Self::Dispatched => "dispatched",
            Self::Acknowledged => "acknowledged",
            Self::Active => "active",
            Self::Running => "running",
            Self::Submitted => "submitted",
            Self::Revision => "revision",
            Self::Blocked => "blocked",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Archived => "archived",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "queued" => Ok(Self::Queued),
            "assigned" => Ok(Self::Assigned),
            "dispatched" => Ok(Self::Dispatched),
            "acknowledged" => Ok(Self::Acknowledged),
            "active" => Ok(Self::Active),
            "running" => Ok(Self::Running),
            "submitted" => Ok(Self::Submitted),
            "revision" => Ok(Self::Revision),
            "blocked" => Ok(Self::Blocked),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "archived" => Ok(Self::Archived),
            other => bail!("unsupported agent task status: {other}"),
        }
    }
}

pub fn agent_task_can_transition(
    kind: AgentTaskKind,
    from: AgentTaskStatus,
    to: AgentTaskStatus,
) -> bool {
    if from == to {
        return true;
    }

    match kind {
        AgentTaskKind::Finite => finite_can_transition(from, to),
        AgentTaskKind::Infinite => infinite_can_transition(from, to),
        AgentTaskKind::TeamDag => team_dag_can_transition(from, to),
    }
}

pub fn ensure_transition(
    kind: AgentTaskKind,
    from: AgentTaskStatus,
    to: AgentTaskStatus,
) -> Result<()> {
    if agent_task_can_transition(kind, from, to) {
        return Ok(());
    }

    bail!(
        "unsupported agent task transition for {}: {} -> {}",
        task_kind_name(kind),
        from.as_str(),
        to.as_str()
    )
}

pub fn ensure_transition_key(kind: AgentTaskKind, from: &str, to: &str) -> Result<()> {
    ensure_transition(kind, AgentTaskStatus::parse(from)?, AgentTaskStatus::parse(to)?)
}

fn finite_can_transition(from: AgentTaskStatus, to: AgentTaskStatus) -> bool {
    use AgentTaskStatus::{
        Acknowledged, Archived, Assigned, Blocked, Cancelled, Completed, Dispatched, Draft, Failed,
        Paused, Queued, Running, Submitted,
    };

    match (from, to) {
        (Draft, Queued)
        | (Draft, Assigned)
        | (Queued, Assigned)
        | (Assigned, Dispatched)
        | (Dispatched, Acknowledged)
        | (Acknowledged, Running)
        | (Running, Submitted)
        | (Submitted, Completed)
        | (Submitted, Queued)
        | (Acknowledged, Blocked)
        | (Running, Blocked)
        | (Submitted, Blocked)
        | (Blocked, Running)
        | (Assigned, Paused)
        | (Dispatched, Paused)
        | (Acknowledged, Paused)
        | (Running, Paused)
        | (Paused, Assigned)
        | (Assigned, Failed)
        | (Dispatched, Failed)
        | (Acknowledged, Failed)
        | (Running, Failed)
        | (Submitted, Failed)
        | (Failed, Queued)
        | (Draft, Cancelled)
        | (Queued, Cancelled)
        | (Assigned, Cancelled)
        | (Dispatched, Cancelled)
        | (Acknowledged, Cancelled)
        | (Running, Cancelled)
        | (Submitted, Cancelled)
        | (Paused, Cancelled)
        | (Blocked, Cancelled)
        | (Completed, Archived)
        | (Failed, Archived)
        | (Cancelled, Archived) => true,
        _ => false,
    }
}

fn infinite_can_transition(from: AgentTaskStatus, to: AgentTaskStatus) -> bool {
    use AgentTaskStatus::{
        Acknowledged, Active, Archived, Blocked, Cancelled, Dispatched, Draft, Failed, Paused,
        Queued, Running,
    };

    match (from, to) {
        (Draft, Active)
        | (Draft, Queued)
        | (Queued, Active)
        | (Active, Dispatched)
        | (Dispatched, Acknowledged)
        | (Acknowledged, Running)
        | (Running, Active)
        | (Acknowledged, Blocked)
        | (Running, Blocked)
        | (Blocked, Active)
        | (Active, Paused)
        | (Paused, Active)
        | (Dispatched, Failed)
        | (Acknowledged, Failed)
        | (Running, Failed)
        | (Failed, Active)
        | (Draft, Cancelled)
        | (Queued, Cancelled)
        | (Active, Cancelled)
        | (Dispatched, Cancelled)
        | (Acknowledged, Cancelled)
        | (Running, Cancelled)
        | (Paused, Cancelled)
        | (Blocked, Cancelled)
        | (Cancelled, Archived) => true,
        _ => false,
    }
}

fn team_dag_can_transition(from: AgentTaskStatus, to: AgentTaskStatus) -> bool {
    use AgentTaskStatus::{
        Acknowledged, Archived, Assigned, Blocked, Cancelled, Completed, Dispatched, Draft, Failed,
        Paused, Queued, Revision, Running, Submitted,
    };

    match (from, to) {
        (Draft, Queued)
        | (Queued, Assigned)
        | (Assigned, Dispatched)
        | (Dispatched, Acknowledged)
        | (Acknowledged, Running)
        | (Running, Submitted)
        | (Submitted, Completed)
        | (Submitted, Revision)
        | (Revision, Queued)
        | (Acknowledged, Blocked)
        | (Running, Blocked)
        | (Submitted, Blocked)
        | (Blocked, Queued)
        | (Queued, Paused)
        | (Assigned, Paused)
        | (Dispatched, Paused)
        | (Acknowledged, Paused)
        | (Running, Paused)
        | (Paused, Queued)
        | (Assigned, Failed)
        | (Dispatched, Failed)
        | (Acknowledged, Failed)
        | (Running, Failed)
        | (Submitted, Failed)
        | (Failed, Queued)
        | (Draft, Cancelled)
        | (Queued, Cancelled)
        | (Assigned, Cancelled)
        | (Dispatched, Cancelled)
        | (Acknowledged, Cancelled)
        | (Running, Cancelled)
        | (Submitted, Cancelled)
        | (Paused, Cancelled)
        | (Blocked, Cancelled)
        | (Completed, Archived)
        | (Failed, Archived)
        | (Cancelled, Archived) => true,
        _ => false,
    }
}

fn task_kind_name(kind: AgentTaskKind) -> &'static str {
    match kind {
        AgentTaskKind::Finite => "finite",
        AgentTaskKind::Infinite => "infinite",
        AgentTaskKind::TeamDag => "team_dag",
    }
}
