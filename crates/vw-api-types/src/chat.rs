//! 聊天消息、流式事件与请求体类型。
//!
//! 本模块定义聊天域的核心协议类型，覆盖：
//! - 会话中的消息角色、状态与消息分片
//! - 发起聊天补全或流式生成所需的请求体
//! - 从网关或代理运行时回传到 UI 的事件流
//! - 工具调用、待办更新、问题提问等聊天期内附属事件
//!
//! # 主要类型
//!
//! - [`MessageDto`][]: 单条消息的标准表示
//! - [`MessagePartDto`][]: 消息的结构化内容片段
//! - [`ChatStreamRequest`][]: 发起聊天流式生成时的请求体
//! - [`ChatEvent`][]: UI 或客户端消费的统一事件枚举
//! - [`GatewayChatStreamRequest`][]: 面向网关兼容层的请求格式

use crate::common::{JsonMap, TimestampMs};
use crate::id::{MessageId, ProjectId, RequestId, SessionId, WorktreeId};
use crate::question::QuestionDto;
use crate::session::{SessionDto, SessionUsageDto};
use crate::todo::TodoDto;
use serde::{Deserialize, Serialize};

/// 消息角色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    /// 系统提示消息。
    System,
    /// 用户输入消息。
    User,
    /// 助手输出消息。
    Assistant,
    /// 工具调用相关消息。
    Tool,
}

/// 消息生成状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    /// 正在流式生成。
    Streaming,
    /// 已完成。
    Completed,
    /// 生成失败。
    Error,
}

/// 单次调用覆盖的推理强度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffortDto {
    /// 显式关闭推理。
    None,
    /// 最低推理强度。
    Minimal,
    /// 低推理强度。
    Low,
    /// 中等推理强度。
    Medium,
    /// 高推理强度。
    High,
    /// 超高推理强度。
    Xhigh,
    /// 最大推理强度。
    Max,
}

/// 消息内容片段。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessagePartDto {
    /// 普通文本片段。
    Text { text: String },
    /// 图片引用片段。
    Image { url: String },
    /// 工具调用片段。
    ToolCall { tool_call_id: String, tool_name: String, arguments_json: String },
    /// 工具结果片段。
    ToolResult { tool_call_id: String, content: String },
}

/// 聊天消息实体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageDto {
    /// 消息 ID。
    pub id: MessageId,
    /// 所属会话 ID。
    pub session_id: SessionId,
    /// 消息角色。
    pub role: ChatRole,
    /// 创建时间。
    pub created_at_ms: TimestampMs,
    /// 消息内容分片。
    pub parts: Vec<MessagePartDto>,
    /// 消息状态。
    pub status: MessageStatus,
    /// 可选的用量统计。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<SessionUsageDto>,
}

/// 发送到聊天接口的输入消息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMessageDto {
    /// 输入消息分片。
    pub parts: Vec<MessagePartDto>,
}

/// 单次聊天调用的可选配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChatOptionsDto {
    /// 指定模型。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 指定提供商 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    /// 采样温度。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// 运行工作目录。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// 指定工作树 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<WorktreeId>,
    /// 覆盖默认推理强度；缺失时沿用 agent 默认值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffortDto>,
}

/// 聊天调用附带的上下文。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChatContextDto {
    /// 当前选中的文件路径列表。
    #[serde(default)]
    pub selected_file_paths: Vec<String>,
    /// 额外扩展字段。
    #[serde(flatten)]
    pub extra: JsonMap,
}

/// 发起流式聊天的请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatStreamRequest {
    /// 目标会话 ID。
    pub session_id: SessionId,
    /// 可选项目 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// 本次用户输入。
    pub input: InputMessageDto,
    /// 本次调用选项。
    #[serde(default)]
    pub options: ChatOptionsDto,
    /// 本次调用上下文。
    #[serde(default)]
    pub context: ChatContextDto,
}

/// 与网关兼容的聊天流请求体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GatewayChatStreamRequest {
    /// 可选会话 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    /// 网关格式消息列表。
    pub messages: Vec<serde_json::Value>,
    /// 系统提示集合。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<String>>,
    /// 指定模型。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// 覆盖默认推理强度；缺失时沿用 agent 默认值。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffortDto>,
    /// 显式指定委托 agent。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// 显式指定委托 agent 允许工具列表。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
    /// 显式指定 ACP 代理。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_agent: Option<String>,
    /// 显式指定 ACP 允许工具列表。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acp_allowed_tools: Option<Vec<String>>,
    /// 透传网关选项。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<serde_json::Value>,
}

/// 解析网关聊天流后得到的内部事件。
#[derive(Debug, Clone, PartialEq)]
pub enum GatewayChatStreamEvent {
    /// 文本增量。
    Delta(String),
    /// 流式结束。
    Done {
        finish_reason: Option<String>,
        usage: Option<serde_json::Value>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    /// 错误事件。
    Error(String),
    /// 其他未标准化事件。
    Other(serde_json::Value),
}

/// 会话启动事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionStartedEvent {
    pub session_id: SessionId,
    pub request_id: RequestId,
    pub started_at_ms: TimestampMs,
}

/// 助手文本增量事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantDeltaEvent {
    pub session_id: SessionId,
    pub message_id: MessageId,
    pub delta: String,
}

/// 助手消息完成事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantMessageCompletedEvent {
    pub session_id: SessionId,
    pub message: MessageDto,
}

/// 工具开始执行事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolStartedEvent {
    pub session_id: SessionId,
    pub message_id: MessageId,
    pub tool_call_id: String,
    pub tool_name: String,
    pub display_title: String,
}

/// 工具输出更新事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolUpdatedEvent {
    pub session_id: SessionId,
    pub message_id: MessageId,
    pub tool_call_id: String,
    pub delta: String,
}

/// 工具执行完成事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCompletedEvent {
    pub session_id: SessionId,
    pub message_id: MessageId,
    pub tool_call_id: String,
    pub status: String,
}

/// 待办更新事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoUpdatedEvent {
    pub session_id: SessionId,
    pub todo: TodoDto,
}

/// 提出问题事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionRaisedEvent {
    pub session_id: SessionId,
    pub question: QuestionDto,
}

/// 问题已解决事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionResolvedEvent {
    pub session_id: SessionId,
    pub question: QuestionDto,
}

/// 用量更新事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageUpdatedEvent {
    pub session_id: SessionId,
    pub usage: SessionUsageDto,
}

/// 标题更新事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TitleUpdatedEvent {
    pub session_id: SessionId,
    pub title: String,
}

/// 会话详情更新事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionUpdatedEvent {
    pub session_id: SessionId,
    pub session: SessionDto,
}

/// 聊天流错误事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatErrorEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    pub code: String,
    pub message: String,
}

/// 聊天流完成事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoneEvent {
    pub session_id: SessionId,
    pub message_id: MessageId,
    pub status: String,
}

/// 统一聊天事件枚举。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum ChatEvent {
    SessionStarted(SessionStartedEvent),
    AssistantDelta(AssistantDeltaEvent),
    AssistantMessageCompleted(AssistantMessageCompletedEvent),
    ToolStarted(ToolStartedEvent),
    ToolUpdated(ToolUpdatedEvent),
    ToolCompleted(ToolCompletedEvent),
    TodoUpdated(TodoUpdatedEvent),
    QuestionRaised(QuestionRaisedEvent),
    QuestionResolved(QuestionResolvedEvent),
    UsageUpdated(UsageUpdatedEvent),
    TitleUpdated(TitleUpdatedEvent),
    SessionUpdated(SessionUpdatedEvent),
    Error(ChatErrorEvent),
    Done(DoneEvent),
}
