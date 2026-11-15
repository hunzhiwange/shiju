/// 非 CLI 审批提示信息
///
/// 用于向用户展示需要审批的工具调用详情
#[derive(Debug, Clone)]
pub struct NonCliApprovalPrompt {
    /// 审批请求的唯一标识符
    pub request_id: String,
    /// 待审批的工具名称
    pub tool_name: String,
    /// 工具调用的参数
    pub arguments: serde_json::Value,
    /// 可选的稳定工具调用 ID。
    pub call_id: Option<String>,
    /// 原始审批请求者。
    pub sender: String,
    /// 原始审批通道。
    pub channel: String,
    /// 原始审批回复目标。
    pub reply_target: String,
}

/// 非 CLI 审批上下文
///
/// 包含用于发送审批提示和回复的通道信息
#[derive(Debug, Clone)]
pub struct NonCliApprovalContext {
    /// 发起审批请求的发送者标识
    pub sender: String,
    /// 回复目标地址（如频道、用户等）
    pub reply_target: String,
    /// 用于发送审批提示的无界通道发送端
    pub prompt_tx: tokio::sync::mpsc::UnboundedSender<NonCliApprovalPrompt>,
}

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
