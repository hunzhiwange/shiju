//! 定义调度领域的强类型标识、状态和值对象，并在构造边界校验外部输入。

use anyhow::{Result, bail};
use serde_json::Value;
use std::path::{Path, PathBuf};
use vw_api_types::agent_scheduler::{
    AgentCapabilityDto, AgentCapabilityKindDto, AgentResourceLeaseDto, AgentResourceLeaseStatusDto,
    AgentRunDto, AgentRunStatusDto, AgentSchedulerScope, AgentTaskDto, AgentTaskKindDto,
    AgentTaskStatusDto, AgentTeamDto, AgentTeamMemberDto, AgentTeamStatusDto, AgentWorkerDto,
    AgentWorkerStatusDto, TeamTaskDagNodeDto,
};
use vw_api_types::common::TimestampMs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSchedulerContext {
    scope: AgentSchedulerScope,
    project_path: PathBuf,
}

impl AgentSchedulerContext {
    pub fn new(scope: AgentSchedulerScope, project_path: impl Into<PathBuf>) -> Result<Self> {
        let project_path = project_path.into();
        if project_path.as_os_str().is_empty() {
            bail!("project path must not be empty");
        }
        Ok(Self { scope, project_path })
    }

    pub fn scope(&self) -> &AgentSchedulerScope {
        &self.scope
    }

    pub fn project_path(&self) -> &Path {
        &self.project_path
    }
}

impl std::ops::Deref for AgentSchedulerContext {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.project_path()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId(String);

impl TaskId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Ok(Self(non_empty(value.into(), "task id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunId(String);

impl RunId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Ok(Self(non_empty(value.into(), "run id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkerName(String);

impl WorkerName {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Ok(Self(non_empty(value.into(), "worker name")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TeamId(String);

impl TeamId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Ok(Self(non_empty(value.into(), "team id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceLeaseId(String);

impl ResourceLeaseId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Ok(Self(non_empty(value.into(), "resource lease id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceKind(String);

impl ResourceKind {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Ok(Self(non_empty(value.into(), "resource kind")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Ok(Self(non_empty(value.into(), "resource id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HolderId(String);

impl HolderId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Ok(Self(non_empty(value.into(), "holder id")?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTaskKind {
    Finite,
    Infinite,
    TeamDag,
}

impl AgentTaskKind {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "finite" => Ok(Self::Finite),
            "infinite" => Ok(Self::Infinite),
            "team_dag" => Ok(Self::TeamDag),
            other => bail!("unsupported agent task kind: {other}"),
        }
    }
}

impl From<AgentTaskKindDto> for AgentTaskKind {
    fn from(value: AgentTaskKindDto) -> Self {
        match value {
            AgentTaskKindDto::Finite => Self::Finite,
            AgentTaskKindDto::Infinite => Self::Infinite,
            AgentTaskKindDto::TeamDag => Self::TeamDag,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl From<AgentTaskStatusDto> for AgentTaskStatus {
    fn from(value: AgentTaskStatusDto) -> Self {
        match value {
            AgentTaskStatusDto::Draft => Self::Draft,
            AgentTaskStatusDto::Queued => Self::Queued,
            AgentTaskStatusDto::Assigned => Self::Assigned,
            AgentTaskStatusDto::Dispatched => Self::Dispatched,
            AgentTaskStatusDto::Acknowledged => Self::Acknowledged,
            AgentTaskStatusDto::Active => Self::Active,
            AgentTaskStatusDto::Running => Self::Running,
            AgentTaskStatusDto::Submitted => Self::Submitted,
            AgentTaskStatusDto::Revision => Self::Revision,
            AgentTaskStatusDto::Blocked => Self::Blocked,
            AgentTaskStatusDto::Paused => Self::Paused,
            AgentTaskStatusDto::Completed => Self::Completed,
            AgentTaskStatusDto::Failed => Self::Failed,
            AgentTaskStatusDto::Cancelled => Self::Cancelled,
            AgentTaskStatusDto::Archived => Self::Archived,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentTask {
    pub id: TaskId,
    pub title: String,
    pub kind: AgentTaskKind,
    pub status: AgentTaskStatus,
    pub assigned_to: Option<WorkerName>,
    pub team_id: Option<TeamId>,
    pub room_id: Option<String>,
    pub project_room_id: Option<String>,
    pub delegated_to_team: Option<TeamId>,
    pub schedule: Option<String>,
    pub timezone: Option<String>,
    pub next_scheduled_at: Option<String>,
    pub last_executed_at: Option<String>,
    pub dag: Vec<TeamTaskDagNode>,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub metadata: Value,
}

impl TryFrom<AgentTaskDto> for AgentTask {
    type Error = anyhow::Error;

    fn try_from(value: AgentTaskDto) -> Result<Self> {
        let AgentTaskDto {
            id,
            title,
            kind,
            status,
            assigned_to,
            team_id,
            room_id,
            project_room_id,
            delegated_to_team,
            schedule,
            timezone,
            next_scheduled_at,
            last_executed_at,
            dag,
            created_at_ms,
            updated_at_ms,
            metadata,
        } = value;

        Ok(Self {
            id: TaskId::new(id)?,
            title,
            kind: kind.into(),
            status: status.into(),
            assigned_to: optional_worker_name(assigned_to)?,
            team_id: optional_team_id(team_id)?,
            room_id,
            project_room_id,
            delegated_to_team: optional_team_id(delegated_to_team)?,
            schedule,
            timezone,
            next_scheduled_at,
            last_executed_at,
            dag: dag.into_iter().map(TeamTaskDagNode::try_from).collect::<Result<Vec<_>>>()?,
            created_at_ms,
            updated_at_ms,
            metadata,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamTaskDagNode {
    pub task_id: TaskId,
    pub title: String,
    pub depends_on: Vec<TaskId>,
    pub assigned_to: Option<WorkerName>,
    pub status: AgentTaskStatus,
}

impl TryFrom<TeamTaskDagNodeDto> for TeamTaskDagNode {
    type Error = anyhow::Error;

    fn try_from(value: TeamTaskDagNodeDto) -> Result<Self> {
        let TeamTaskDagNodeDto { task_id, title, depends_on, assigned_to, status } = value;

        Ok(Self {
            task_id: TaskId::new(task_id)?,
            title,
            depends_on: depends_on.into_iter().map(TaskId::new).collect::<Result<Vec<_>>>()?,
            assigned_to: optional_worker_name(assigned_to)?,
            status: status.into(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunStatus {
    Pending,
    Leased,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Stale,
}

impl From<AgentRunStatusDto> for AgentRunStatus {
    fn from(value: AgentRunStatusDto) -> Self {
        match value {
            AgentRunStatusDto::Pending => Self::Pending,
            AgentRunStatusDto::Leased => Self::Leased,
            AgentRunStatusDto::Running => Self::Running,
            AgentRunStatusDto::Succeeded => Self::Succeeded,
            AgentRunStatusDto::Failed => Self::Failed,
            AgentRunStatusDto::Cancelled => Self::Cancelled,
            AgentRunStatusDto::TimedOut => Self::TimedOut,
            AgentRunStatusDto::Stale => Self::Stale,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRun {
    pub id: RunId,
    pub task_id: TaskId,
    pub worker: Option<WorkerName>,
    pub status: AgentRunStatus,
    pub lease_expires_at_ms: Option<TimestampMs>,
    pub attempt: u32,
    pub started_at_ms: Option<TimestampMs>,
    pub finished_at_ms: Option<TimestampMs>,
    pub last_heartbeat_at_ms: Option<TimestampMs>,
    pub error_message: Option<String>,
}

impl TryFrom<AgentRunDto> for AgentRun {
    type Error = anyhow::Error;

    fn try_from(value: AgentRunDto) -> Result<Self> {
        let AgentRunDto {
            id,
            task_id,
            worker_id,
            status,
            lease_expires_at_ms,
            attempt,
            started_at_ms,
            finished_at_ms,
            last_heartbeat_at_ms,
            error_message,
        } = value;
        let status = AgentRunStatus::from(status);
        let worker = optional_worker_name(worker_id)?;

        match status {
            AgentRunStatus::Leased if worker.is_none() => {
                bail!("leased run requires worker name")
            }
            AgentRunStatus::Running if worker.is_none() => {
                bail!("running run requires worker name")
            }
            _ => {}
        }

        Ok(Self {
            id: RunId::new(id)?,
            task_id: TaskId::new(task_id)?,
            worker,
            status,
            lease_expires_at_ms,
            attempt,
            started_at_ms,
            finished_at_ms,
            last_heartbeat_at_ms,
            error_message,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentCapabilityKind {
    Skill,
    RemoteSkill,
    McpServer,
    Model,
    Runtime,
    Tool,
}

impl AgentCapabilityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::RemoteSkill => "remote_skill",
            Self::McpServer => "mcp_server",
            Self::Model => "model",
            Self::Runtime => "runtime",
            Self::Tool => "tool",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "skill" => Ok(Self::Skill),
            "remote_skill" => Ok(Self::RemoteSkill),
            "mcp_server" => Ok(Self::McpServer),
            "model" => Ok(Self::Model),
            "runtime" => Ok(Self::Runtime),
            "tool" => Ok(Self::Tool),
            other => bail!("unsupported agent capability kind: {other}"),
        }
    }
}

impl From<AgentCapabilityKindDto> for AgentCapabilityKind {
    fn from(value: AgentCapabilityKindDto) -> Self {
        match value {
            AgentCapabilityKindDto::Skill => Self::Skill,
            AgentCapabilityKindDto::RemoteSkill => Self::RemoteSkill,
            AgentCapabilityKindDto::McpServer => Self::McpServer,
            AgentCapabilityKindDto::Model => Self::Model,
            AgentCapabilityKindDto::Runtime => Self::Runtime,
            AgentCapabilityKindDto::Tool => Self::Tool,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentCapability {
    pub kind: AgentCapabilityKind,
    pub name: String,
    pub metadata: Value,
}

impl From<AgentCapabilityDto> for AgentCapability {
    fn from(value: AgentCapabilityDto) -> Self {
        Self { kind: value.kind.into(), name: value.name, metadata: value.metadata }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentWorkerStatus {
    Pending,
    Idle,
    Busy,
    Running,
    Sleeping,
    Stopped,
    Failed,
    Unavailable,
}

impl AgentWorkerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Idle => "idle",
            Self::Busy => "busy",
            Self::Running => "running",
            Self::Sleeping => "sleeping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "idle" => Ok(Self::Idle),
            "busy" => Ok(Self::Busy),
            "running" => Ok(Self::Running),
            "sleeping" => Ok(Self::Sleeping),
            "stopped" => Ok(Self::Stopped),
            "failed" => Ok(Self::Failed),
            "unavailable" => Ok(Self::Unavailable),
            other => bail!("unsupported agent worker status: {other}"),
        }
    }
}

impl From<AgentWorkerStatusDto> for AgentWorkerStatus {
    fn from(value: AgentWorkerStatusDto) -> Self {
        match value {
            AgentWorkerStatusDto::Pending => Self::Pending,
            AgentWorkerStatusDto::Idle => Self::Idle,
            AgentWorkerStatusDto::Busy => Self::Busy,
            AgentWorkerStatusDto::Running => Self::Running,
            AgentWorkerStatusDto::Sleeping => Self::Sleeping,
            AgentWorkerStatusDto::Stopped => Self::Stopped,
            AgentWorkerStatusDto::Failed => Self::Failed,
            AgentWorkerStatusDto::Unavailable => Self::Unavailable,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentWorker {
    pub id: WorkerName,
    pub runtime: Option<String>,
    pub model: String,
    pub model_provider: Option<String>,
    pub status: AgentWorkerStatus,
    pub desired_state: Option<String>,
    pub team_id: Option<TeamId>,
    pub worker_role: Option<String>,
    pub matrix_user_id: Option<String>,
    pub room_id: Option<String>,
    pub container_state: Option<String>,
    pub idle_since_ms: Option<TimestampMs>,
    pub last_heartbeat_at: Option<String>,
    pub capabilities: Vec<AgentCapability>,
}

impl TryFrom<AgentWorkerDto> for AgentWorker {
    type Error = anyhow::Error;

    fn try_from(value: AgentWorkerDto) -> Result<Self> {
        let AgentWorkerDto {
            id,
            runtime,
            deployment: _,
            model,
            model_provider,
            status,
            desired_state,
            team_id,
            worker_role,
            matrix_user_id,
            room_id,
            container_state,
            idle_since_ms,
            last_seen_at_ms: _,
            last_heartbeat_at,
            capabilities,
            updated_at_ms: _,
        } = value;

        Ok(Self {
            id: WorkerName::new(id)?,
            runtime,
            model,
            model_provider,
            status: status.into(),
            desired_state,
            team_id: optional_team_id(team_id)?,
            worker_role,
            matrix_user_id,
            room_id,
            container_state,
            idle_since_ms,
            last_heartbeat_at,
            capabilities: capabilities.into_iter().map(AgentCapability::from).collect(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTeamStatus {
    Pending,
    Active,
    Degraded,
    Failed,
}

impl AgentTeamStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Degraded => "degraded",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "degraded" => Ok(Self::Degraded),
            "failed" => Ok(Self::Failed),
            other => bail!("unsupported agent team status: {other}"),
        }
    }
}

impl From<AgentTeamStatusDto> for AgentTeamStatus {
    fn from(value: AgentTeamStatusDto) -> Self {
        match value {
            AgentTeamStatusDto::Pending => Self::Pending,
            AgentTeamStatusDto::Active => Self::Active,
            AgentTeamStatusDto::Degraded => Self::Degraded,
            AgentTeamStatusDto::Failed => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTeamMember {
    pub worker_id: WorkerName,
    pub runtime_name: Option<WorkerName>,
    pub role: String,
    pub room_id: Option<String>,
    pub matrix_user_id: Option<String>,
    pub ready: bool,
}

impl TryFrom<AgentTeamMemberDto> for AgentTeamMember {
    type Error = anyhow::Error;

    fn try_from(value: AgentTeamMemberDto) -> Result<Self> {
        let AgentTeamMemberDto { worker_id, runtime_name, role, room_id, matrix_user_id, ready } =
            value;

        Ok(Self {
            worker_id: WorkerName::new(worker_id)?,
            runtime_name: optional_worker_name(runtime_name)?,
            role,
            room_id,
            matrix_user_id,
            ready,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTeam {
    pub id: TeamId,
    pub name: String,
    pub status: AgentTeamStatus,
    pub leader_id: WorkerName,
    pub team_room_id: Option<String>,
    pub leader_dm_room_id: Option<String>,
    pub leader_ready: bool,
    pub ready_workers: u32,
    pub total_workers: u32,
    pub members: Vec<AgentTeamMember>,
}

impl TryFrom<AgentTeamDto> for AgentTeam {
    type Error = anyhow::Error;

    fn try_from(value: AgentTeamDto) -> Result<Self> {
        let AgentTeamDto {
            id,
            name,
            status,
            leader_id,
            team_room_id,
            leader_dm_room_id,
            leader_ready,
            ready_workers,
            total_workers,
            members,
            updated_at_ms: _,
        } = value;

        Ok(Self {
            id: TeamId::new(id)?,
            name,
            status: status.into(),
            leader_id: WorkerName::new(leader_id)?,
            team_room_id,
            leader_dm_room_id,
            leader_ready,
            ready_workers,
            total_workers,
            members: members
                .into_iter()
                .map(AgentTeamMember::try_from)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentResourceLeaseStatus {
    Active,
    Released,
    Expired,
    Revoked,
}

impl From<AgentResourceLeaseStatusDto> for AgentResourceLeaseStatus {
    fn from(value: AgentResourceLeaseStatusDto) -> Self {
        match value {
            AgentResourceLeaseStatusDto::Active => Self::Active,
            AgentResourceLeaseStatusDto::Released => Self::Released,
            AgentResourceLeaseStatusDto::Expired => Self::Expired,
            AgentResourceLeaseStatusDto::Revoked => Self::Revoked,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResourceLease {
    pub id: ResourceLeaseId,
    pub resource_kind: ResourceKind,
    pub resource_id: ResourceId,
    pub holder_id: HolderId,
    pub status: AgentResourceLeaseStatus,
    pub acquired_at_ms: TimestampMs,
    pub expires_at_ms: Option<TimestampMs>,
    pub released_at_ms: Option<TimestampMs>,
}

impl TryFrom<AgentResourceLeaseDto> for AgentResourceLease {
    type Error = anyhow::Error;

    fn try_from(value: AgentResourceLeaseDto) -> Result<Self> {
        let AgentResourceLeaseDto {
            id,
            resource_kind,
            resource_id,
            holder_id,
            status,
            acquired_at_ms,
            expires_at_ms,
            released_at_ms,
        } = value;

        Ok(Self {
            id: ResourceLeaseId::new(id)?,
            resource_kind: ResourceKind::new(resource_kind)?,
            resource_id: ResourceId::new(resource_id)?,
            holder_id: HolderId::new(holder_id)?,
            status: status.into(),
            acquired_at_ms,
            expires_at_ms,
            released_at_ms,
        })
    }
}

fn optional_worker_name(value: Option<String>) -> Result<Option<WorkerName>> {
    value.map(WorkerName::new).transpose()
}

fn optional_team_id(value: Option<String>) -> Result<Option<TeamId>> {
    value.map(TeamId::new).transpose()
}

fn non_empty(value: String, field_name: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("{field_name} must not be empty");
    }
    Ok(value)
}
