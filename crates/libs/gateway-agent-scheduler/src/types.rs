//! Agent Scheduler HTTP 请求与响应契约。

use serde::{Deserialize, Serialize};
use vw_api_agent_scheduler::{
    AgentTaskDto,
    AgentEventDto,
    AgentOutboxMessageDto,
    AgentSchedulerDispatchModeDto,
};
use vw_api_transport::common::TimestampMs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSchedulerTasksQuery {
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSchedulerTasksResponse {
    pub tasks: Vec<AgentTaskDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSchedulerWakeRequest {
    pub project_path: String,
    pub now_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "is_default_dispatch_mode")]
    pub dispatch_mode: AgentSchedulerDispatchModeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub auto_promote_pool_tasks: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSchedulerWakeResponse {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSchedulerRecoveryRequest {
    pub project_path: String,
    pub now_ms: TimestampMs,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSchedulerRecoveryResponse {
    pub graced_run_ids: Vec<String>,
    pub recovered_run_ids: Vec<String>,
    pub released_resource_lease_ids: Vec<String>,
    pub tainted_resource_lease_ids: Vec<String>,
    pub drifted_worker_ids: Vec<String>,
    pub slept_worker_ids: Vec<String>,
    pub outbox_message_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSchedulerEventsQuery {
    pub project_path: String,
    #[serde(default)]
    pub after_cursor: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSchedulerEventsResponse {
    pub cursor: i64,
    pub events: Vec<AgentEventDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSchedulerOutboxDrainRequest {
    pub project_path: String,
    pub now_ms: TimestampMs,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSchedulerOutboxDrainResponse {
    pub claimed: usize,
    pub messages: Vec<AgentOutboxMessageDto>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentSchedulerProjectEventsQuery {
    #[serde(default)]
    pub after_cursor: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
}

fn is_default_dispatch_mode(dispatch_mode: &AgentSchedulerDispatchModeDto) -> bool {
    *dispatch_mode == AgentSchedulerDispatchModeDto::default()
}

fn is_false(value: &bool) -> bool {
    !*value
}
