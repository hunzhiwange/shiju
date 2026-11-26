use serde::{Deserialize, Serialize};
use vw_api_transport::common::TimestampMs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskDetailDto {
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
    pub created_by: Option<String>,
    pub assignee: Option<String>,
    pub priority: Option<u32>,
    pub collaboration: AgentSchedulerTaskCollaborationDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_agent: Option<String>,
    #[serde(default)]
    pub reply_session: AgentSchedulerTaskReplySession,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerTaskReplySession {
    #[default]
    Missing,
    Idle,
    Busy,
    Unavailable,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentSchedulerTaskCollaborationDto {
    pub start_date: Option<String>,
    pub due_date: Option<String>,
    pub comments: Vec<AgentSchedulerTaskCommentDto>,
    pub pull_requests: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskCommentDto {
    pub id: String,
    pub reply_to: Option<String>,
    pub author: String,
    pub body: String,
    pub created_at_ms: TimestampMs,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_turn: Option<AgentSchedulerTaskAgentTurnDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerTaskReplyMode {
    Start,
    Queue,
    Restart,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskReplyDelivery {
    pub agent: String,
    pub mode: AgentSchedulerTaskReplyMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_turn_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSchedulerTaskAgentTurnStatus {
    Queued,
    Running,
    Completed,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSchedulerTaskAgentTurnDto {
    pub agent: String,
    pub session_id: String,
    pub status: AgentSchedulerTaskAgentTurnStatus,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub response: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentSchedulerTaskDetailChange {
    SetAssignee {
        assignee: String,
    },
    SetPriority {
        priority: u32,
    },
    Comment {
        id: String,
        reply_to: Option<String>,
        body: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        attachments: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        delivery: Option<AgentSchedulerTaskReplyDelivery>,
    },
    StopReply {
        id: String,
    },
    SetStartDate {
        date: Option<String>,
    },
    SetDueDate {
        date: Option<String>,
    },
    LinkPullRequest {
        url: String,
    },
    UnlinkPullRequest {
        url: String,
    },
}
