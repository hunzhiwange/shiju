//! 处理器与外部协议共享的事件类型。

use serde_json::Value;

/// 工具调用在类型化生命周期中的阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolEventKind {
    /// 工具开始执行。
    Start,
    /// 工具仍在执行且状态或输出有更新。
    Update,
    /// 工具到达终态。
    Complete,
}

/// 工具调用的协议中立状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallStatus {
    /// 正在执行。
    Running,
    /// 等待后台工作完成。
    Waiting,
    /// 成功完成。
    Completed,
    /// 执行失败。
    Failed,
    /// 权限被拒绝。
    Denied,
    /// 已取消。
    Cancelled,
    /// 兼容未知但已净化的状态值。
    Other(String),
}

/// 可直接映射到外部协议的类型化工具事件。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolEvent {
    /// 同一次工具调用在开始、更新和完成事件中保持稳定的 ID。
    pub call_id: String,
    /// 工具注册名称。
    pub name: String,
    /// 当前生命周期阶段。
    pub kind: ToolEventKind,
    /// 当前工具状态。
    pub status: ToolCallStatus,
    /// 生命周期事件发生时的 Unix 毫秒时间戳。
    pub occurred_ms: u64,
    /// 已复用 UI 脱敏规则处理的结构化载荷。
    pub payload: Value,
}

/// 自动压缩生命周期阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionPhase {
    /// 已确认存在可摘要历史，即将请求内部摘要。
    Started,
    /// 摘要已成功写回临时会话历史。
    Completed,
}

/// 模型恢复的非终止状态；最终错误仍由 Error 事件报告。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryPhase {
    Waiting,
    Recovered,
    Stopped,
    Cancelled,
}

/// 一次有界恢复状态，不包含原始响应体、请求头或配置内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRecovery {
    /// 当前已执行的尝试数。
    pub attempt: u64,
    pub max_attempts: u64,
    pub wait_ms: u64,
    pub reason: String,
    pub phase: RecoveryPhase,
    pub next_action: String,
}
