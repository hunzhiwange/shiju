use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 单步或整轮会话的 token 使用统计。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    #[serde(default)]
    pub reasoning_tokens: i64,
}

/// UI 层消息的角色类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChatRole {
    User,
    Assistant,
    System,
    /// Trusted runtime instructions, distinct from user content.
    Developer,
    Tool,
}

/// 思考过程的时间片段信息。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ThinkTiming {
    pub start_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<u64>,
    pub last_update_ms: u64,
}

/// 一条助手消息对应的模型处理周期。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessingTiming {
    pub started_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_ms: Option<u64>,
    /// 整轮处理的单调时钟耗时；包含步骤间停顿，但不包含系统休眠。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_elapsed_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_elapsed_ms: Option<u64>,
}

/// UI 会话中展示的一条消息。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub think_timing: Vec<ThinkTiming>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_timing: Option<ProcessingTiming>,
}

/// 一条带稳定存储序号和可选消息身份的会话消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMessageRecord {
    pub seq_no: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    pub message: ChatMessage,
}

/// 按 `seq_no` 升序返回的固定大小消息页。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMessagePage {
    pub records: Vec<SessionMessageRecord>,
    pub has_more: bool,
}

/// 对单个会话消息序列的原子增量修改。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionMessageDelta {
    pub session_id: String,
    pub expected_updated_ms: u64,
    pub updated_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub upserts: Vec<SessionMessageRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncate_after: Option<u64>,
}

/// 会话列表展示所需的轻量元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionMeta {
    pub id: String,
    pub title: String,
    pub updated_ms: u64,
    pub message_count: usize,
    pub call_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_content: Option<String>,
}

/// 会话内待办事项的 UI 结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTodoItem {
    pub content: String,
    pub status: String,
    pub priority: String,
    pub id: String,
}

/// 一次工具调用在会话步骤内持久化的生命周期时间。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolTiming {
    pub call_id: String,
    pub name: String,
    #[serde(default)]
    pub started_ms: Option<u64>,
    #[serde(default)]
    pub finished_ms: Option<u64>,
}

/// 会话中单个执行步骤的统计信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionStep {
    pub index: u32,
    /// 所属助手轮次的稳定身份；旧会话未迁移时为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub started_ms: u64,
    #[serde(default)]
    pub finished_ms: Option<u64>,
    #[serde(default)]
    pub start_snapshot_path: Option<String>,
    #[serde(default)]
    pub finish_snapshot_path: Option<String>,
    #[serde(default)]
    pub usage: TokenUsage,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_timings: Vec<ToolTiming>,
}

/// UI 层完整会话对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub message_ids: Vec<Option<String>>,
    #[serde(default)]
    pub calls: Vec<Value>,
    #[serde(default)]
    pub steps: Vec<ChatSessionStep>,
    #[serde(default)]
    pub created_ms: u64,
    #[serde(default)]
    pub updated_ms: u64,
}

#[cfg(test)]
#[path = "ui_types_tests.rs"]
mod ui_types_tests;
