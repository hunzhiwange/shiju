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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentSchedulerTaskDetailChange {
    SetAssignee { assignee: String },
    SetPriority { priority: u32 },
    Comment { id: String, reply_to: Option<String>, body: String },
    SetStartDate { date: Option<String> },
    SetDueDate { date: Option<String> },
    LinkPullRequest { url: String },
    UnlinkPullRequest { url: String },
}
