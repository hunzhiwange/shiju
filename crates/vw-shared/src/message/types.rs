//! 消息与消息片段的共享类型定义。

use crate::snapshot;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// 用户消息摘要中可附带的文件差异信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiffSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default)]
    pub diffs: Vec<snapshot::FileDiff>,
}

/// 指向某个 provider 模型的引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRef {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

/// 会话执行时的工作目录与项目根目录。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInfo {
    pub cwd: String,
    pub root: String,
}

/// token 缓存读写统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCacheInfo {
    pub read: i64,
    pub write: i64,
}

/// 一次模型调用的 token 统计。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
    pub input: i64,
    pub output: i64,
    pub reasoning: i64,
    pub cache: TokenCacheInfo,
}

/// 助手侧消息可能携带的错误信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum AssistantError {
    ProviderAuthError {
        provider_id: String,
        message: String,
    },
    MessageOutputLengthError,
    MessageAbortedError {
        message: String,
    },
    ContextOverflowError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_body: Option<String>,
    },
    APIError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status_code: Option<i64>,
        is_retryable: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_headers: Option<std::collections::HashMap<String, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_body: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        metadata: Option<std::collections::HashMap<String, String>>,
    },
    Unknown {
        message: String,
    },
}

/// 用户消息时间信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTime {
    pub created: u64,
}

/// 助手消息时间信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantTime {
    pub created: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
    /// 整轮处理的单调时钟耗时；包含步骤间停顿，但不包含系统休眠。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing_elapsed_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_elapsed_ms: Option<u64>,
}

/// 助手消息元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantInfo {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub time: AssistantTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AssistantError>,
    #[serde(rename = "parentID")]
    pub parent_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    pub mode: String,
    pub agent: String,
    pub path: PathInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<bool>,
    pub cost: f64,
    pub tokens: TokenInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish: Option<String>,
}

/// 用户消息元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub time: UserTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<FileDiffSummary>,
    pub agent: String,
    pub model: ModelRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<std::collections::HashMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

/// 会话消息统一信息结构，区分用户与助手两侧。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Info {
    User(Box<UserInfo>),
    Assistant(Box<AssistantInfo>),
}

impl Info {
    /// 返回消息 ID。
    pub fn id(&self) -> &str {
        match self {
            Info::User(u) => &u.id,
            Info::Assistant(a) => &a.id,
        }
    }

    /// 设置消息 ID。
    pub fn set_id(&mut self, id: &str) {
        match self {
            Info::User(u) => u.id = id.to_string(),
            Info::Assistant(a) => a.id = id.to_string(),
        }
    }

    /// 设置会话 ID。
    pub fn set_session_id(&mut self, session_id: &str) {
        match self {
            Info::User(u) => u.session_id = session_id.to_string(),
            Info::Assistant(a) => a.session_id = session_id.to_string(),
        }
    }

    /// 仅对助手消息设置父消息 ID。
    pub fn set_parent_id(&mut self, parent_id: &str) {
        if let Info::Assistant(a) = self {
            a.parent_id = parent_id.to_string();
        }
    }
}

/// 所有消息片段共享的基础标识字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartBase {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(rename = "messageID")]
    pub message_id: String,
}

/// 片段时间区间。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartTime {
    pub start: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<u64>,
}

/// 普通文本片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<PartTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
}

/// 推理文本片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
    pub time: PartTime,
}

/// 快照引用片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub snapshot: String,
}

/// 补丁引用片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub hash: String,
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub additions: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletions: Option<i64>,
}

/// 文件来源中的文本片段范围。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSourceText {
    pub value: String,
    pub start: i64,
    pub end: i64,
}

/// 文件来源的公共基类。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSourceBase {
    pub text: FileSourceText,
}

/// 来自文件内容的片段来源。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSource {
    #[serde(flatten)]
    pub base: FileSourceBase,
    pub path: String,
}

/// LSP 位置坐标。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspPosition {
    pub line: i64,
    pub character: i64,
}

/// LSP 范围坐标。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspRange {
    pub start: LspPosition,
    pub end: LspPosition,
}

/// 来自符号检索结果的片段来源。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolSource {
    #[serde(flatten)]
    pub base: FileSourceBase,
    pub path: String,
    pub range: LspRange,
    pub name: String,
    pub kind: i64,
}

/// 来自外部资源的片段来源。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSource {
    #[serde(flatten)]
    pub base: FileSourceBase,
    #[serde(rename = "clientName")]
    pub client_name: String,
    pub uri: String,
}

/// 文件片段可引用的来源类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum FilePartSource {
    File(FileSource),
    Symbol(SymbolSource),
    Resource(ResourceSource),
}

/// 文件附件片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePart {
    #[serde(flatten)]
    pub base: PartBase,
    pub mime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FilePartSource>,
}

/// 代理切换或选择片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<FileSourceText>,
}

/// 压缩上下文事件片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub auto: bool,
}

/// 子任务使用的模型引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskModel {
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(rename = "modelID")]
    pub model_id: String,
}

/// 子任务片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub prompt: String,
    pub description: String,
    pub agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<SubtaskModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// 重试片段的时间信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryTime {
    pub created: u64,
}

/// 一次重试记录片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub attempt: i64,
    pub error: AssistantError,
    pub time: RetryTime,
}

/// 步骤开始片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepStartPart {
    #[serde(flatten)]
    pub base: PartBase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

/// 步骤结束片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepFinishPart {
    #[serde(flatten)]
    pub base: PartBase,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    pub cost: f64,
    pub tokens: TokenInfo,
}

/// 工具调用待执行状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatePending {
    pub input: Map<String, Value>,
    pub raw: String,
}

/// 工具调用执行中状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStateRunning {
    pub input: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
    pub time: PartTime,
}

/// 工具调用输出附带的附件信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAttachment {
    pub mime: String,
    pub url: String,
}

/// 工具调用完成状态的时间信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStateCompletedTime {
    pub start: u64,
    pub end: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacted: Option<u64>,
}

/// 工具调用完成状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStateCompleted {
    pub input: Map<String, Value>,
    pub output: String,
    pub title: String,
    pub metadata: Map<String, Value>,
    pub time: ToolStateCompletedTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<ToolAttachment>>,
}

/// 工具调用失败状态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStateError {
    pub input: Map<String, Value>,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
    pub time: PartTime,
}

/// 工具调用状态机。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ToolState {
    Pending(ToolStatePending),
    Running(ToolStateRunning),
    Completed(ToolStateCompleted),
    Error(ToolStateError),
}

/// 工具调用片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPart {
    #[serde(flatten)]
    pub base: PartBase,
    #[serde(rename = "callID")]
    pub call_id: String,
    pub tool: String,
    pub state: ToolState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Map<String, Value>>,
}

/// 会话中的统一片段枚举。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Part {
    #[serde(rename = "text")]
    Text(TextPart),
    #[serde(rename = "subtask")]
    Subtask(SubtaskPart),
    #[serde(rename = "reasoning")]
    Reasoning(ReasoningPart),
    #[serde(rename = "file")]
    File(FilePart),
    #[serde(rename = "tool")]
    Tool(ToolPart),
    #[serde(rename = "step-start")]
    StepStart(StepStartPart),
    #[serde(rename = "step-finish")]
    StepFinish(StepFinishPart),
    #[serde(rename = "snapshot")]
    Snapshot(SnapshotPart),
    #[serde(rename = "patch")]
    Patch(PatchPart),
    #[serde(rename = "agent")]
    Agent(AgentPart),
    #[serde(rename = "retry")]
    Retry(RetryPart),
    #[serde(rename = "compaction")]
    Compaction(CompactionPart),
}

impl Part {
    /// 返回片段 ID。
    pub fn id(&self) -> &str {
        match self {
            Part::Text(p) => &p.base.id,
            Part::Subtask(p) => &p.base.id,
            Part::Reasoning(p) => &p.base.id,
            Part::File(p) => &p.base.id,
            Part::Tool(p) => &p.base.id,
            Part::StepStart(p) => &p.base.id,
            Part::StepFinish(p) => &p.base.id,
            Part::Snapshot(p) => &p.base.id,
            Part::Patch(p) => &p.base.id,
            Part::Agent(p) => &p.base.id,
            Part::Retry(p) => &p.base.id,
            Part::Compaction(p) => &p.base.id,
        }
    }

    /// 返回片段所属会话 ID。
    pub fn session_id(&self) -> &str {
        match self {
            Part::Text(p) => &p.base.session_id,
            Part::Subtask(p) => &p.base.session_id,
            Part::Reasoning(p) => &p.base.session_id,
            Part::File(p) => &p.base.session_id,
            Part::Tool(p) => &p.base.session_id,
            Part::StepStart(p) => &p.base.session_id,
            Part::StepFinish(p) => &p.base.session_id,
            Part::Snapshot(p) => &p.base.session_id,
            Part::Patch(p) => &p.base.session_id,
            Part::Agent(p) => &p.base.session_id,
            Part::Retry(p) => &p.base.session_id,
            Part::Compaction(p) => &p.base.session_id,
        }
    }

    /// 返回片段所属消息 ID。
    pub fn message_id(&self) -> &str {
        match self {
            Part::Text(p) => &p.base.message_id,
            Part::Subtask(p) => &p.base.message_id,
            Part::Reasoning(p) => &p.base.message_id,
            Part::File(p) => &p.base.message_id,
            Part::Tool(p) => &p.base.message_id,
            Part::StepStart(p) => &p.base.message_id,
            Part::StepFinish(p) => &p.base.message_id,
            Part::Snapshot(p) => &p.base.message_id,
            Part::Patch(p) => &p.base.message_id,
            Part::Agent(p) => &p.base.message_id,
            Part::Retry(p) => &p.base.message_id,
            Part::Compaction(p) => &p.base.message_id,
        }
    }

    /// 设置片段 ID。
    pub fn set_id(&mut self, id: &str) {
        let id = id.to_string();
        match self {
            Part::Text(p) => p.base.id = id,
            Part::Subtask(p) => p.base.id = id,
            Part::Reasoning(p) => p.base.id = id,
            Part::File(p) => p.base.id = id,
            Part::Tool(p) => p.base.id = id,
            Part::StepStart(p) => p.base.id = id,
            Part::StepFinish(p) => p.base.id = id,
            Part::Snapshot(p) => p.base.id = id,
            Part::Patch(p) => p.base.id = id,
            Part::Agent(p) => p.base.id = id,
            Part::Retry(p) => p.base.id = id,
            Part::Compaction(p) => p.base.id = id,
        }
    }

    /// 设置片段所属会话 ID。
    pub fn set_session_id(&mut self, session_id: &str) {
        let session_id = session_id.to_string();
        match self {
            Part::Text(p) => p.base.session_id = session_id,
            Part::Subtask(p) => p.base.session_id = session_id,
            Part::Reasoning(p) => p.base.session_id = session_id,
            Part::File(p) => p.base.session_id = session_id,
            Part::Tool(p) => p.base.session_id = session_id,
            Part::StepStart(p) => p.base.session_id = session_id,
            Part::StepFinish(p) => p.base.session_id = session_id,
            Part::Snapshot(p) => p.base.session_id = session_id,
            Part::Patch(p) => p.base.session_id = session_id,
            Part::Agent(p) => p.base.session_id = session_id,
            Part::Retry(p) => p.base.session_id = session_id,
            Part::Compaction(p) => p.base.session_id = session_id,
        }
    }

    /// 设置片段所属消息 ID。
    pub fn set_message_id(&mut self, message_id: &str) {
        let message_id = message_id.to_string();
        match self {
            Part::Text(p) => p.base.message_id = message_id,
            Part::Subtask(p) => p.base.message_id = message_id,
            Part::Reasoning(p) => p.base.message_id = message_id,
            Part::File(p) => p.base.message_id = message_id,
            Part::Tool(p) => p.base.message_id = message_id,
            Part::StepStart(p) => p.base.message_id = message_id,
            Part::StepFinish(p) => p.base.message_id = message_id,
            Part::Snapshot(p) => p.base.message_id = message_id,
            Part::Patch(p) => p.base.message_id = message_id,
            Part::Agent(p) => p.base.message_id = message_id,
            Part::Retry(p) => p.base.message_id = message_id,
            Part::Compaction(p) => p.base.message_id = message_id,
        }
    }
}

/// 带完整片段列表的消息对象。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithParts {
    pub info: Info,
    pub parts: Vec<Part>,
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
