use vw_api_transport::common::TimestampMs;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent 能力类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapabilityKindDto {
    Skill,
    RemoteSkill,
    McpServer,
    Model,
    Runtime,
    Tool,
}

/// Agent 能力声明。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCapabilityDto {
    pub kind: AgentCapabilityKindDto,
    pub name: String,
    #[serde(default)]
    pub metadata: Value,
}

/// 资源 lease 状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentResourceLeaseStatusDto {
    Active,
    Released,
    Expired,
    Revoked,
}

/// Agent Scheduler 资源 lease。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentResourceLeaseDto {
    pub id: String,
    pub resource_kind: String,
    pub resource_id: String,
    pub holder_id: String,
    pub status: AgentResourceLeaseStatusDto,
    pub acquired_at_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<TimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at_ms: Option<TimestampMs>,
}
