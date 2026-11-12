use serde::{Deserialize, Serialize};

/// 工具执行前的审批请求。
///
/// 当代理尝试执行某个工具时，如果该工具需要审批，系统会创建此请求结构
/// 并等待用户的确认响应。
///
/// # 字段说明
///
/// - `tool_name`：待执行的工具名称（如 "shell"、"file_write" 等）
/// - `arguments`：传递给工具的参数，以 JSON 格式存储
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// 待执行的工具名称
    pub tool_name: String,
    /// 传递给工具的参数（JSON 格式）
    pub arguments: serde_json::Value,
}

/// 用户对审批请求的响应类型。
///
/// 定义了用户可以对审批请求做出的三种响应选择。
/// `Yes` 和 `No` 是一次性决策，而 `Always` 会将会话级别的权限
/// 添加到白名单中。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalResponse {
    /// 执行本次调用（仅此一次）
    Yes,
    /// 拒绝本次调用
    No,
    /// 执行并将工具添加到会话白名单（后续调用自动批准）
    Always,
}

/// 单条审批决策的审计日志条目。
///
/// 每次用户做出审批决策时，系统都会创建一条审计日志记录，
/// 用于追踪和审计所有工具执行的行为。
///
/// # 用途
///
/// - 安全审计：追溯谁在何时批准了什么操作
/// - 调试分析：了解代理的行为模式
/// - 合规记录：满足企业安全合规要求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalLogEntry {
    /// 决策时间戳（RFC3339 格式）
    pub timestamp: String,
    /// 涉及的工具名称
    pub tool_name: String,
    /// 参数摘要（截断后的可读描述）
    pub arguments_summary: String,
    /// 用户的审批决策
    pub decision: ApprovalResponse,
    /// 触发此审批的通道（如 "cli"、"telegram" 等）
    pub channel: String,
}

/// 待处理的非 CLI 审批请求。
///
/// 当非命令行通道（如 Telegram、Slack）需要审批时，系统会创建一个
/// 待处理请求并等待人工确认。此结构包含请求的所有相关信息。
///
/// # 生命周期
///
/// 1. 代理发起工具调用请求 -> 创建 `PendingNonCliApprovalRequest`
/// 2. 请求被存储在管理器的待处理队列中
/// 3. 人工用户通过命令确认或拒绝
/// 4. 决策被记录，请求从队列中移除
///
/// # 过期机制
///
/// 请求默认有效期为 30 分钟；ACP 请求持续挂起，不自动过期。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingNonCliApprovalRequest {
    /// 请求的唯一标识符（格式：apr-{uuid8}）
    pub request_id: String,
    /// 待审批的工具名称
    pub tool_name: String,
    /// 待审批工具的参数快照
    pub arguments: serde_json::Value,
    /// 触发该审批的消息 ID
    pub message_id: Option<String>,
    /// 触发该审批的工具调用 ID
    pub call_id: Option<String>,
    /// 发起请求的用户标识
    pub requested_by: String,
    /// 发起请求的通道标识
    pub requested_channel: String,
    /// 回复目标地址（用于发送审批结果通知）
    pub requested_reply_target: String,
    /// 请求原因说明（可选）
    pub reason: Option<String>,
    /// 请求创建时间（RFC3339 格式）
    pub created_at: String,
    /// 请求过期时间（RFC3339 格式）；ACP 不过期，此字段为空。
    pub expires_at: String,
}

/// 待处理审批请求操作时可能发生的错误。
///
/// 当尝试确认、拒绝或查询待处理请求时，可能遇到以下错误情况。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingApprovalError {
    /// 请求不存在（可能已被处理或从未创建）
    NotFound,
    /// 请求已过期
    Expired,
    /// 请求者不匹配（确认/拒绝者与原始请求者不同）
    RequesterMismatch,
}
