use vw_api_transport::common::TimestampMs;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent Scheduler 事件类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventKindDto {
    TaskCreated,
    ImportedLegacyTask,
    TaskAssigned,
    WorkerSelected,
    WorkerAcknowledged,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    TaskBlocked,
    TaskPaused,
    TaskResumed,
    TaskCancelled,
    TaskArchived,
    RunLeased,
    RunHeartbeat,
    RunStarted,
    RunSubmitted,
    RunCompleted,
    RunFailed,
    RunRecovered,
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

/// Agent Scheduler 事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEventDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    pub kind: AgentEventKindDto,
    pub message: String,
    #[serde(default)]
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}

/// Outbox 消息状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentOutboxMessageStatusDto {
    Pending,
    Sending,
    Delivered,
    Failed,
    Cancelled,
}

/// Agent Scheduler outbox 消息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentOutboxMessageDto {
    pub id: String,
    pub target_kind: String,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient_id: Option<String>,
    pub body: String,
    pub status: AgentOutboxMessageStatusDto,
    pub idempotency_key: String,
    pub attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_attempt_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivered_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default)]
    pub payload: Value,
    pub created_at_ms: TimestampMs,
}
