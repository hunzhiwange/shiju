//! 工作流流式请求、事件与脱敏调试输出。
use std::{collections::BTreeMap, fmt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vw_api_workflow::WorkflowPauseDto;

/// Workflow streaming 响应模式。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayWorkflowResponseMode {
    Streaming,
}

/// Workflow streaming 请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWorkflowStreamRequest {
    pub query: String,
    #[serde(default)]
    pub inputs: BTreeMap<String, Value>,
    pub response_mode: GatewayWorkflowResponseMode,
    pub user: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// Workflow 默认入口的 streaming 请求体，可指定保存应用或内联 YAML。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWorkflowDefaultStreamRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application_workflow: Option<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, Value>,
    pub response_mode: GatewayWorkflowResponseMode,
    pub user: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

/// Workflow 应用一次运行后的精简结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayWorkflowRunOutcome {
    pub summary: String,
    pub workflow_run_id: Option<String>,
    pub conversation_id: Option<String>,
}

/// Workflow streaming 错误事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWorkflowStreamError {
    pub task_id: String,
    pub status: u32,
    pub code: String,
    pub message: String,
}

/// Workflow streaming SSE 事件。
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum GatewayWorkflowStreamEvent {
    WorkflowStarted {
        task_id: String,
        workflow_run_id: String,
        data: Value,
    },
    NodeStarted {
        task_id: String,
        workflow_run_id: String,
        data: Value,
    },
    NodeFinished {
        task_id: String,
        workflow_run_id: String,
        data: Value,
    },
    TextChunk {
        task_id: String,
        workflow_run_id: String,
        text: String,
        data: Value,
    },
    Message {
        id: String,
        task_id: String,
        message_id: String,
        conversation_id: String,
        answer: String,
        created_at: i64,
    },
    WorkflowFinished {
        task_id: String,
        workflow_run_id: String,
        data: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pause: Option<WorkflowPauseDto>,
    },
    MessageEnd {
        id: String,
        task_id: String,
        message_id: String,
        conversation_id: String,
        metadata: Value,
    },
    Error(GatewayWorkflowStreamError),
    Unknown {
        event_type: Option<String>,
        payload: Value,
    },
}

impl fmt::Debug for GatewayWorkflowStreamEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkflowStarted { task_id, workflow_run_id, data } => formatter
                .debug_struct("WorkflowStarted")
                .field("task_id", task_id)
                .field("workflow_run_id", workflow_run_id)
                .field("data", data)
                .finish(),
            Self::NodeStarted { task_id, workflow_run_id, data } => formatter
                .debug_struct("NodeStarted")
                .field("task_id", task_id)
                .field("workflow_run_id", workflow_run_id)
                .field("data", data)
                .finish(),
            Self::NodeFinished { task_id, workflow_run_id, data } => formatter
                .debug_struct("NodeFinished")
                .field("task_id", task_id)
                .field("workflow_run_id", workflow_run_id)
                .field("data", data)
                .finish(),
            Self::TextChunk { task_id, workflow_run_id, text, data } => formatter
                .debug_struct("TextChunk")
                .field("task_id", task_id)
                .field("workflow_run_id", workflow_run_id)
                .field("text", text)
                .field("data", data)
                .finish(),
            Self::Message { id, task_id, message_id, conversation_id, answer, created_at } => {
                formatter
                    .debug_struct("Message")
                    .field("id", id)
                    .field("task_id", task_id)
                    .field("message_id", message_id)
                    .field("conversation_id", conversation_id)
                    .field("answer", answer)
                    .field("created_at", created_at)
                    .finish()
            }
            Self::WorkflowFinished { task_id, workflow_run_id, data, pause } => {
                let mut debug = formatter.debug_struct("WorkflowFinished");
                debug.field("task_id", task_id).field("workflow_run_id", workflow_run_id);
                if data.get("pause").is_some() {
                    debug.field("data", &"[REDACTED]");
                } else {
                    debug.field("data", data);
                }
                debug.field("pause", &pause.as_ref().map(RedactedWorkflowPause)).finish()
            }
            Self::MessageEnd { id, task_id, message_id, conversation_id, metadata } => formatter
                .debug_struct("MessageEnd")
                .field("id", id)
                .field("task_id", task_id)
                .field("message_id", message_id)
                .field("conversation_id", conversation_id)
                .field("metadata", metadata)
                .finish(),
            Self::Error(error) => formatter.debug_tuple("Error").field(error).finish(),
            Self::Unknown { event_type, payload } => formatter
                .debug_struct("Unknown")
                .field("event_type", event_type)
                .field("payload", payload)
                .finish(),
        }
    }
}

struct RedactedWorkflowPause<'a>(&'a WorkflowPauseDto);

impl fmt::Debug for RedactedWorkflowPause<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkflowPauseDto")
            .field("node_id", &self.0.node_id)
            .field("title", &self.0.title)
            .field("form_token", &"[REDACTED]")
            .field("form", &self.0.form)
            .field("actions", &self.0.actions)
            .finish()
    }
}
