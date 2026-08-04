//! 非 CLI 环境下的审批决策处理模块
//!
//! 本模块提供了在非命令行交互环境（如聊天通道、Web 界面等）中
//! 等待和处理用户审批决策的功能。主要用途包括：
//!
//! - 在异步任务中安全地等待外部审批响应
//! - 处理超时、取消等边界情况
//! - 与 ApprovalManager 协同工作，管理审批请求的生命周期

use crate::app::agent::approval::{ApprovalManager, ApprovalResponse};
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// 非 CLI 环境下等待审批响应的超时时间（秒）
///
/// 在此时间后，如果用户未做出决策，系统将自动拒绝请求
const NON_CLI_APPROVAL_WAIT_TIMEOUT_SECS: u64 = 300;

/// 非 CLI 环境下轮询审批决策的间隔时间（毫秒）
///
/// 系统会定期检查是否有新的审批决策可用
const NON_CLI_APPROVAL_POLL_INTERVAL_MS: u64 = 250;

/// 非 CLI 审批提示信息
///
/// 用于向用户展示需要审批的工具调用详情
#[derive(Debug, Clone)]
pub(crate) struct NonCliApprovalPrompt {
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
    pub(crate) sender: String,
    /// 回复目标地址（如频道、用户等）
    pub(crate) reply_target: String,
    /// 用于发送审批提示的无界通道发送端
    pub(crate) prompt_tx: tokio::sync::mpsc::UnboundedSender<NonCliApprovalPrompt>,
}

// 任务本地存储：非 CLI 审批上下文
//
// 使用 Tokio 的 task_local 宏定义的异步任务本地存储，
// 用于在工具循环中访问当前的非 CLI 审批上下文。
// 如果当前任务不在非 CLI 环境中运行，此值为 None。
tokio::task_local! {
    pub(crate) static TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT: Option<NonCliApprovalContext>;
}

#[cfg(test)]
#[path = "approval_tests.rs"]
mod approval_tests;

/// 等待非 CLI 环境下的审批决策
///
/// 此函数会在指定超时时间内轮询审批管理器，等待用户对工具调用做出审批决策。
/// 函数会处理多种退出条件：
///
/// - 用户做出明确的批准或拒绝决策
/// - 审批请求被意外移除（安全失败，返回拒绝）
/// - 任务被取消（返回拒绝）
/// - 超时（自动拒绝）
///
/// # 参数
///
/// - `mgr`: 审批管理器的引用，用于查询和管理审批状态
/// - `request_id`: 待等待的审批请求 ID
/// - `sender`: 发起审批请求的发送者标识
/// - `channel_name`: 审批请求所在的通道名称
/// - `reply_target`: 审批响应的回复目标地址
/// - `cancellation_token`: 可选的取消令牌，用于支持任务取消
///
/// # 返回值
///
/// 返回 `ApprovalResponse` 枚举，表示用户的审批决策或自动拒绝
///
/// # 示例
///
/// ```ignore
/// let response = await_non_cli_approval_decision(
///     &approval_manager,
///     "request-123",
///     "user@example.com",
///     "telegram",
///     "chat-456",
///     Some(cancellation_token),
/// ).await;
///
/// match response {
///     ApprovalResponse::Yes => println!("用户批准"),
///     ApprovalResponse::No => println!("用户拒绝或超时"),
/// }
/// ```
pub(crate) async fn await_non_cli_approval_decision(
    mgr: &ApprovalManager,
    request_id: &str,
    sender: &str,
    channel_name: &str,
    reply_target: &str,
    cancellation_token: Option<&CancellationToken>,
) -> ApprovalResponse {
    let started = Instant::now();

    loop {
        // 优先检查是否有已解决的审批决策
        if let Some(decision) = mgr.take_non_cli_pending_resolution(request_id) {
            return decision;
        }

        // 检查审批请求是否仍然存在
        if !mgr.has_non_cli_pending_request(request_id) {
            // 当请求在没有明确解决的情况下消失时，采用安全失败策略，返回拒绝
            return ApprovalResponse::No;
        }

        // 检查任务是否被取消
        if cancellation_token.is_some_and(CancellationToken::is_cancelled) {
            let _ =
                mgr.reject_non_cli_pending_request(request_id, sender, channel_name, reply_target);
            let _ = mgr.take_non_cli_pending_resolution(request_id);
            return ApprovalResponse::No;
        }

        // 检查是否超时
        if started.elapsed() >= Duration::from_secs(NON_CLI_APPROVAL_WAIT_TIMEOUT_SECS) {
            // 超时后自动拒绝该请求，并清理相关状态
            let _ =
                mgr.reject_non_cli_pending_request(request_id, sender, channel_name, reply_target);
            let _ = mgr.take_non_cli_pending_resolution(request_id);
            return ApprovalResponse::No;
        }

        // 短暂休眠后继续轮询，避免 CPU 空转
        tokio::time::sleep(Duration::from_millis(NON_CLI_APPROVAL_POLL_INTERVAL_MS)).await;
    }
}
