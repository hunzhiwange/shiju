use crate::{AgentTaskStatusDto, AgentEventKindDto, AgentOutboxMessageDto};
use vw_api_chat_identity::ReasoningEffortDto;
use vw_api_transport::{common::TimestampMs, id::{ProjectId, TaskId}};
use vw_api_task::{TaskPoolWorkflowSnapshotDto, TaskPoolWorkflowStatusRoleDto};
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;
use std::collections::BTreeMap;

fn deserialize_optional_reasoning_effort<'de, D>(
    deserializer: D,
) -> Result<Option<ReasoningEffortDto>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .map(|value| match value.as_str() {
            "none" => Ok(ReasoningEffortDto::None),
            "minimal" => Ok(ReasoningEffortDto::Minimal),
            "low" => Ok(ReasoningEffortDto::Low),
            "medium" => Ok(ReasoningEffortDto::Medium),
            "high" => Ok(ReasoningEffortDto::High),
            "xhigh" => Ok(ReasoningEffortDto::Xhigh),
            "max" => Ok(ReasoningEffortDto::Max),
            _ => Err(de::Error::custom(format!("invalid reasoning_effort value: {value}"))),
        })
        .transpose()
}

/// Agent Scheduler 桌面任务板命令类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerCommandKindDto {
    Start,
    Retry,
    Pause,
    Resume,
    Cancel,
    Archive,
    Promote,
    Merge,
    Move,
    UpdateMetadata,
    SoftDelete,
    Restore,
    ImportLegacyTask,
}

/// Agent Scheduler 桌面任务板命令请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerCommandRequest {
    pub task_id: TaskId,
    pub command: AgentSchedulerCommandKindDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "AgentSchedulerCommandPayloadDto::is_empty")]
    pub payload: AgentSchedulerCommandPayloadDto,
}

/// Agent Scheduler 桌面任务板命令的可选载荷。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerCommandPayloadDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_task: Option<AgentSchedulerMoveTaskPayloadDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_update: Option<AgentSchedulerTaskMetadataPatchDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_import: Option<AgentSchedulerLegacyTaskImportDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

impl AgentSchedulerCommandPayloadDto {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// 移动任务到目标任务板状态的 command 载荷。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerMoveTaskPayloadDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_status_key: Option<String>,
    pub to_status_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_status: Option<AgentTaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_no: Option<i64>,
}

/// 任务元数据与任务板字段 patch。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskMetadataPatchDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentTaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order_no: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_reasoning_effort"
    )]
    pub reasoning_effort: Option<ReasoningEffortDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_source_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_target_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_slot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_role: Option<TaskPoolWorkflowStatusRoleDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_merge: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata_remove_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_ids: Option<Vec<String>>,
    /// A null value removes only the property at that ID.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub property_changes: AgentSchedulerTaskPropertyChanges,
}

/// 旧桌面任务导入项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerLegacyTaskImportDto {
    pub task_id: TaskId,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentTaskStatusDto>,
    #[serde(default)]
    pub priority: u32,
    #[serde(default)]
    pub order_no: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_reasoning_effort"
    )]
    pub reasoning_effort: Option<ReasoningEffortDto>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    pub created_at_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<TimestampMs>,
}

/// Structured inputs for a stage action command or projection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerStageActionInputsDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, Value>,
}

impl AgentSchedulerStageActionInputsDto {
    pub fn is_empty(&self) -> bool {
        self.workflow_uuid.is_none() && self.parameters.is_empty()
    }
}

/// Stage action result kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerStageActionOutcomeDto {
    Success,
    Failure,
    Retry,
    Blocked,
}

/// Agent Scheduler 任务工作区上下文。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerWorkspaceContextDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_id: Option<String>,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// Agent Scheduler 桌面任务板任务快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardTaskDto {
    pub project_id: ProjectId,
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AgentTaskStatusDto>,
    pub priority: u32,
    pub order_no: i64,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffortDto>,
    pub prompt: String,
    pub retry_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<AgentSchedulerWorkspaceContextDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pause_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_source_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_target_branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_slot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_worktree_branch: Option<String>,
    #[serde(default, skip_serializing_if = "AgentSchedulerTaskBoardTaskReadModelDto::is_empty")]
    pub read_model: AgentSchedulerTaskBoardTaskReadModelDto,
    pub updated_at_ms: TimestampMs,
}

/// 单个任务面向薄客户端的展示投影。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardTaskReadModelDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_blocker: Option<AgentSchedulerDispatchBlockerDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_role: Option<TaskPoolWorkflowStatusRoleDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub label_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: AgentSchedulerTaskPropertyValues,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentSchedulerDispatchBlockerDto {
    ExplicitDependency { task_ids: Vec<String> },
}

impl AgentSchedulerTaskBoardTaskReadModelDto {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Agent Scheduler 桌面任务板状态响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardStatusResponse {
    pub project_id: ProjectId,
    #[serde(
        default,
        skip_serializing_if = "AgentSchedulerTaskBoardSnapshotReadModelDto::is_empty"
    )]
    pub read_model: AgentSchedulerTaskBoardSnapshotReadModelDto,
    pub tasks: Vec<AgentSchedulerTaskBoardTaskDto>,
}

/// 任务板快照级展示投影。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardSnapshotReadModelDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<TaskPoolWorkflowSnapshotDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<AgentSchedulerTaskLabelDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub properties: Vec<AgentSchedulerTaskPropertyDto>,
}

impl AgentSchedulerTaskBoardSnapshotReadModelDto {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

pub const TASK_PROPERTY_VALUES_METADATA_KEY: &str = "task_board.property_values";

pub type AgentSchedulerTaskPropertyValues = BTreeMap<String, AgentSchedulerTaskPropertyValue>;
pub type AgentSchedulerTaskPropertyChanges = BTreeMap<String, Option<AgentSchedulerTaskPropertyValue>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerTaskPropertyKind {
    SingleSelect,
    Date,
    Text,
    Number,
    MultiSelect,
    Checkbox,
    Link,
    Member,
    MultiMember,
    TextList,
    LinkList,
}

impl AgentSchedulerTaskPropertyKind {
    pub fn has_options(self) -> bool {
        matches!(self, Self::SingleSelect | Self::MultiSelect)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum AgentSchedulerTaskPropertyValue {
    SingleSelect(String),
    Date(String),
    Text(String),
    Number(String),
    MultiSelect(Vec<String>),
    Checkbox(bool),
    Link(String),
    Member(String),
    MultiMember(Vec<String>),
    TextList(Vec<String>),
    LinkList(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskPropertyOption {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub archived: bool,
}

/// Older clients supply an option name as a string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentSchedulerTaskPropertyOptionInput {
    Name(String),
    Colored { name: String, color: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskPropertyOptionChange {
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub archive: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerTaskPropertyIcon {
    #[default]
    Sliders,
    Star,
    Person,
    Calendar,
    Check,
    Chart,
    Link,
    Clock,
    List,
    Globe,
    Shield,
    Box,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskPropertyDto {
    pub id: String,
    pub name: String,
    pub kind: AgentSchedulerTaskPropertyKind,
    #[serde(default)]
    pub icon: AgentSchedulerTaskPropertyIcon,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<AgentSchedulerTaskPropertyOption>,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentSchedulerTaskPropertyMutation {
    Create {
        name: String,
        kind: AgentSchedulerTaskPropertyKind,
        #[serde(default)]
        icon: AgentSchedulerTaskPropertyIcon,
        #[serde(default)]
        description: String,
        #[serde(default)]
        options: Vec<AgentSchedulerTaskPropertyOptionInput>,
    },
    Update {
        id: String,
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<AgentSchedulerTaskPropertyIcon>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(default)]
        new_options: Vec<AgentSchedulerTaskPropertyOptionInput>,
        #[serde(default)]
        option_changes: BTreeMap<String, AgentSchedulerTaskPropertyOptionChange>,
    },
    Archive { id: String },
}

pub const TASK_LABEL_IDS_METADATA_KEY: &str = "task_board.label_ids";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskLabelDto {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AgentSchedulerTaskLabelMutation {
    Create {
        name: String,
        color: String,
        #[serde(default)]
        description: String,
    },
    Update {
        id: String,
        name: Option<String>,
        color: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    Archive {
        id: String,
    },
}

/// 选用租户生命周期预设，要求当前已保存的快照仍与编辑时一致。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerSelectWorkflowRequest {
    pub workflow_id: String,
    pub expected_workflow: Option<TaskPoolWorkflowSnapshotDto>,
}

/// Agent Scheduler 项目 workflow 快照响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardWorkflowResponse {
    pub project_id: ProjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<TaskPoolWorkflowSnapshotDto>,
}

/// Agent Scheduler 桌面任务板事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardEventDto {
    pub cursor: i64,
    pub project_id: ProjectId,
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_stage: Option<String>,
    pub kind: AgentEventKindDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_status: Option<AgentTaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_status: Option<AgentTaskStatusDto>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<AgentSchedulerWorkspaceContextDto>,
    #[serde(default)]
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

/// Agent Scheduler 桌面任务板事件响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskBoardEventsResponse {
    pub cursor: i64,
    pub events: Vec<AgentSchedulerTaskBoardEventDto>,
}

/// Agent Scheduler 桌面任务板日志条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskLogDto {
    pub task_id: TaskId,
    pub seq_no: i64,
    pub timestamp_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_from: Option<AgentTaskStatusDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_status_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_to: Option<AgentTaskStatusDto>,
    pub message: String,
}

/// Agent Scheduler 桌面任务板日志响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskLogsResponse {
    pub task_id: TaskId,
    pub logs: Vec<AgentSchedulerTaskLogDto>,
}

/// Agent Scheduler 桌面任务板产物。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskArtifactDto {
    pub task_id: TaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub kind: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default)]
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

/// Agent Scheduler 桌面任务板产物响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskArtifactsResponse {
    pub task_id: TaskId,
    pub artifacts: Vec<AgentSchedulerTaskArtifactDto>,
}

/// Agent Scheduler 桌面任务板命令响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerCommandResponse {
    pub accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<AgentSchedulerTaskBoardTaskDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<AgentSchedulerTaskBoardEventDto>,
}

/// Agent Scheduler 任务板批量命令请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerBulkCommandRequest {
    pub commands: Vec<AgentSchedulerCommandRequest>,
    #[serde(default)]
    pub stop_on_error: bool,
}

/// Agent Scheduler 任务板批量命令响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerBulkCommandResponse {
    pub accepted: u32,
    pub rejected: u32,
    pub results: Vec<AgentSchedulerCommandResponse>,
}

/// Agent Scheduler 任务元数据更新请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskMetadataUpdateRequest {
    pub patch: AgentSchedulerTaskMetadataPatchDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Agent Scheduler 任务元数据更新响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskMetadataUpdateResponse {
    pub task: AgentSchedulerTaskBoardTaskDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<AgentSchedulerTaskBoardEventDto>,
}

/// Agent Scheduler 旧任务导入请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerLegacyImportRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<TaskPoolWorkflowSnapshotDto>,
    pub tasks: Vec<AgentSchedulerLegacyTaskImportDto>,
    #[serde(default)]
    pub dry_run: bool,
}

/// Agent Scheduler 旧任务导入响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerLegacyImportResponse {
    pub imported: u32,
    pub skipped: u32,
    #[serde(default)]
    pub tasks: Vec<AgentSchedulerTaskBoardTaskDto>,
    #[serde(default)]
    pub events: Vec<AgentSchedulerTaskBoardEventDto>,
}

/// Agent Scheduler 唤醒请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerWakeRequest {
    pub now_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_promote_pool_tasks: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Agent Scheduler 唤醒响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerWakeResponse {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dispatched_runs: Vec<AgentSchedulerDispatchedRunDto>,
    pub dispatched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbox_message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerDispatchedRunDto {
    pub task_id: String,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbox_message_id: Option<String>,
}

/// Agent Scheduler outbox drain 请求。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerOutboxDrainRequest {
    pub now_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Agent Scheduler outbox drain 响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSchedulerOutboxDrainResponse {
    pub claimed: usize,
    pub messages: Vec<AgentOutboxMessageDto>,
}
