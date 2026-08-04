//! 后端事件与 ACP `session/update` 的类型化映射。

use agent_client_protocol::{
    ContentBlock, ContentChunk, Error, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus,
    RequestPermissionRequest, RequestPermissionResponse, SessionNotification, SessionUpdate,
    TextContent, ToolCall, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields,
};
use async_trait::async_trait;

/// 后端工具状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendToolStatus {
    /// 等待开始。
    Pending,
    /// 正在执行。
    Running,
    /// 成功完成。
    Completed,
    /// 执行失败或被拒绝。
    Failed,
    /// 已取消。
    Cancelled,
}

/// 后端计划项状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPlanStatus {
    /// 尚未开始。
    Pending,
    /// 正在执行。
    InProgress,
    /// 已完成。
    Completed,
}

/// 后端计划项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendPlanEntry {
    /// 计划文本。
    pub content: String,
    /// 当前状态。
    pub status: BackendPlanStatus,
}

/// 后端终止原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendTermination {
    /// 正常结束。
    EndTurn,
    /// 达到令牌上限。
    MaxTokens,
    /// 达到轮次请求上限。
    MaxTurnRequests,
    /// 拒绝继续。
    Refusal,
    /// 客户端取消。
    Cancelled,
}

/// 协议中立的单轮后端事件。
#[derive(Debug, Clone, PartialEq)]
pub enum BackendEvent {
    /// 助手文本。
    Text(String),
    /// 工具开始。
    ToolStart {
        /// 稳定调用 ID。
        call_id: String,
        /// 展示标题。
        title: String,
        /// 已脱敏输入。
        raw_input: serde_json::Value,
    },
    /// 工具进度更新。
    ToolUpdate {
        /// 稳定调用 ID。
        call_id: String,
        /// 状态。
        status: BackendToolStatus,
        /// 可选文本内容。
        content: Option<String>,
        /// 可选已脱敏输出。
        raw_output: Option<serde_json::Value>,
    },
    /// 工具终态。
    ToolComplete {
        /// 稳定调用 ID。
        call_id: String,
        /// 终态。
        status: BackendToolStatus,
        /// 可选文本内容。
        content: Option<String>,
        /// 可选已脱敏输出。
        raw_output: Option<serde_json::Value>,
    },
    /// 完整计划快照。
    Plan(Vec<BackendPlanEntry>),
    /// 协议层以 thought chunk 展示的状态文本。
    Status(String),
    /// 单轮终态。
    Terminal(BackendTermination),
    /// 单轮错误。
    Error(String),
}

/// `session/update` 通知输出边界。
#[async_trait(?Send)]
pub trait SessionUpdateSink: 'static {
    /// 发送一条真实 ACP 通知。
    async fn send(&self, notification: SessionNotification)
    -> agent_client_protocol::Result<()>;

    /// 请求客户端对工具调用做出权限选择。
    async fn request_permission(
        &self,
        _request: RequestPermissionRequest,
    ) -> agent_client_protocol::Result<RequestPermissionResponse> {
        Ok(RequestPermissionResponse::new(
            agent_client_protocol::RequestPermissionOutcome::Cancelled,
        ))
    }
}

pub(crate) struct NoopUpdateSink;

#[async_trait(?Send)]
impl SessionUpdateSink for NoopUpdateSink {
    async fn send(
        &self,
        _notification: SessionNotification,
    ) -> agent_client_protocol::Result<()> {
        Ok(())
    }
}

fn tool_status(status: BackendToolStatus) -> ToolCallStatus {
    match status {
        BackendToolStatus::Pending => ToolCallStatus::Pending,
        BackendToolStatus::Running => ToolCallStatus::InProgress,
        BackendToolStatus::Completed => ToolCallStatus::Completed,
        BackendToolStatus::Failed | BackendToolStatus::Cancelled => ToolCallStatus::Failed,
    }
}

fn tool_content(content: Option<String>) -> Option<Vec<agent_client_protocol::ToolCallContent>> {
    content.map(|text| vec![text.into()])
}

fn tool_update(
    call_id: String,
    status: BackendToolStatus,
    content: Option<String>,
    raw_output: Option<serde_json::Value>,
) -> SessionUpdate {
    let mut fields = ToolCallUpdateFields::new().status(tool_status(status));
    if let Some(content) = tool_content(content) {
        fields = fields.content(content);
    }
    if let Some(raw_output) = raw_output {
        fields = fields.raw_output(raw_output);
    }
    SessionUpdate::ToolCallUpdate(ToolCallUpdate::new(call_id, fields))
}

pub(crate) fn session_update(event: BackendEvent) -> Result<Option<SessionUpdate>, Error> {
    match event {
        BackendEvent::Text(text) => Ok(Some(SessionUpdate::AgentMessageChunk(ContentChunk::new(
            ContentBlock::Text(TextContent::new(text)),
        )))),
        BackendEvent::ToolStart { call_id, title, raw_input } => {
            Ok(Some(SessionUpdate::ToolCall(
                ToolCall::new(call_id, title)
                    .status(ToolCallStatus::InProgress)
                    .raw_input(raw_input),
            )))
        }
        BackendEvent::ToolUpdate { call_id, status, content, raw_output }
        | BackendEvent::ToolComplete { call_id, status, content, raw_output } => {
            Ok(Some(tool_update(call_id, status, content, raw_output)))
        }
        BackendEvent::Plan(entries) => Ok(Some(SessionUpdate::Plan(Plan::new(
            entries
                .into_iter()
                .map(|entry| {
                    let status = match entry.status {
                        BackendPlanStatus::Pending => PlanEntryStatus::Pending,
                        BackendPlanStatus::InProgress => PlanEntryStatus::InProgress,
                        BackendPlanStatus::Completed => PlanEntryStatus::Completed,
                    };
                    PlanEntry::new(entry.content, PlanEntryPriority::Medium, status)
                })
                .collect(),
        )))),
        BackendEvent::Status(status) => Ok(Some(SessionUpdate::AgentThoughtChunk(
            ContentChunk::new(ContentBlock::Text(TextContent::new(status))),
        ))),
        BackendEvent::Terminal(_) => Ok(None),
        BackendEvent::Error(error) => {
            Err(Error::internal_error().data(serde_json::json!({ "reason": error })))
        }
    }
}
