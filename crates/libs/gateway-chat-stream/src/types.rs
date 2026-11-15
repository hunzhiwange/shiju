//! 聊天流客户端事件与恢复契约。

/// 直接透出新的流式聊天事件类型。
pub use vw_api_types::chat::{GatewayChatStreamEvent, GatewayChatStreamRequest};

/// 将网关 usage 载荷规整后的轻量统计结构。
///
/// 当前网关的 `chat.step_finish` 与 `chat.done` 都会携带这组字段；
/// 在 CLI/TUI 侧先统一成稳定结构，避免继续裸用 JSON。
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct GatewayChatUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    #[serde(default)]
    pub reasoning_tokens: i64,
}

/// `chat.step_start` 的 typed 事件载荷。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayChatStepStartEvent {
    pub step_index: u32,
    pub created_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// `chat.step_finish` 的 typed 事件载荷。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayChatStepFinishEvent {
    pub step_index: u32,
    pub finished_ms: u64,
    #[serde(default)]
    pub usage: GatewayChatUsage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// `chat.post_tool_round` 的 typed 事件载荷。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayChatPostToolRoundEvent {
    pub step_index: u32,
}

/// `chat.tool` 生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayChatToolEventKind {
    Start,
    Update,
    Complete,
}

/// `chat.tool` 的协议中立状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayChatToolStatus {
    Running,
    Waiting,
    Completed,
    Failed,
    Denied,
    Cancelled,
    Other(String),
}

/// 网关规整后的类型化工具生命周期事件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayChatToolEvent {
    pub step_index: u32,
    pub call_id: String,
    pub name: String,
    pub kind: GatewayChatToolEventKind,
    pub status: GatewayChatToolStatus,
    pub occurred_ms: u64,
    pub payload: serde_json::Value,
}

impl GatewayChatToolEvent {
    /// 保留旧聊天工具卡消费的文本协议。
    pub fn legacy_delta(&self) -> String {
        format!("tool {}\n{}\n", self.name, self.payload)
    }
}

/// 模型恢复进度；所有阶段均等待独立的 done/error 终态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatewayModelRecoveryPhase {
    Waiting,
    Recovered,
    Stopped,
    Cancelled,
}

/// 已脱敏的模型恢复状态，独立于模型正文和工具事件。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayModelRecoveryEvent {
    pub session_id: String,
    pub attempt: u64,
    pub max_attempts: u64,
    pub wait_ms: u64,
    pub phase: GatewayModelRecoveryPhase,
    pub reason: String,
    pub next_action: String,
}

/// 网关流事件经客户端规整后的 typed 结果。
///
/// `GatewayChatStreamEvent` 仍保留原始兼容接口；新调用点应尽量改用
/// 这里的 typed 结果，避免直接在 UI 层解析 `Other(JSON)`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayTypedChatStreamEvent {
    ModelRecovery(GatewayModelRecoveryEvent),
    Delta(String),
    ContextLoaded {
        paths: Vec<String>,
    },
    StepStart(GatewayChatStepStartEvent),
    StepFinish(GatewayChatStepFinishEvent),
    PostToolRound(GatewayChatPostToolRoundEvent),
    TodoUpdated {
        session_id: Option<String>,
    },
    QuestionRaised {
        session_id: Option<String>,
    },
    QuestionResolved {
        session_id: Option<String>,
    },
    UsageUpdated {
        session_id: Option<String>,
        usage: GatewayChatUsage,
    },
    TitleUpdated {
        session_id: Option<String>,
        title: String,
    },
    SessionUpdated {
        session_id: Option<String>,
        title: Option<String>,
    },
    Done {
        finish_reason: Option<String>,
        usage: Option<GatewayChatUsage>,
        message_id: Option<String>,
        parent_message_id: Option<String>,
    },
    Error(String),
    Unknown {
        event_type: Option<String>,
    },
}

#[deprecated(note = "use GatewayChatStreamEvent")]
/// 兼容旧命名的流式聊天事件类型。
pub type ChatStreamEvent = GatewayChatStreamEvent;
#[deprecated(note = "use GatewayChatStreamRequest")]
/// 兼容旧命名的流式聊天请求类型。
pub type ChatStreamRequest = GatewayChatStreamRequest;

#[allow(dead_code)]
/// 建立流连接时的默认超时时间，单位为秒。
pub const STREAM_CONNECT_TIMEOUT_SECS: u64 = 30;
#[allow(dead_code)]
/// 单次流式请求允许持续的最长时间，单位为秒。
pub const STREAM_REQUEST_TIMEOUT_SECS: u64 = 60 * 60 * 3;
/// 标识同一次可恢复流运行的请求头。
pub const STREAM_REQUEST_ID_HEADER: &str = "x-vw-stream-request-id";
/// 指示服务端只回放该序号之后事件的标准 SSE 请求头。
pub const LAST_EVENT_ID_HEADER: &str = "last-event-id";

/// 单次可恢复流请求携带的稳定运行标识与已确认游标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayStreamResume {
    pub request_id: String,
    pub last_event_id: Option<u64>,
}

/// 带服务端单调 SSE 序号的业务事件。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatewayStreamEvent<T> {
    pub id: u64,
    pub event: T,
}

/// 通用 Gateway SSE 帧，保留可选 id、event 名和 JSON data。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GatewayJsonSseEvent {
    pub id: Option<u64>,
    pub event: Option<String>,
    pub data: serde_json::Value,
}

/// 一次可恢复流订阅尝试的结束原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayStreamAttemptResult {
    /// 已收到业务协议定义的明确终态。
    Terminated,
    /// 调用方拒绝继续接收事件。
    CallerStopped,
    /// 连接在明确终态之前结束，调用方可使用原请求标识和游标重连。
    Incomplete,
}
