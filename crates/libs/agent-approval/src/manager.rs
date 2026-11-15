use super::{ApprovalLogEntry, ApprovalResponse, PendingNonCliApprovalRequest};
use config_security::NonCliNaturalLanguageApprovalMode;
use config_security::AutonomyLevel;
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet};

/// 交互式审批工作流的核心管理器。
///
/// `ApprovalManager` 负责管理整个审批流程，包括：
///
/// - 检查工具是否需要审批
/// - 维护会话级和持久化的白名单
/// - 管理待处理的审批请求队列
/// - 记录审计日志
///
/// # 线程安全
///
/// 所有内部状态都使用 `Mutex` 或 `RwLock` 保护，支持多线程并发访问。
///
/// # 配置来源
///
/// 管理器的初始配置来自 [`AutonomyConfig`]，包括：
/// - `auto_approve`：自动批准的工具列表
/// - `always_ask`：始终需要确认的工具列表
/// - `level`：自主运行级别
/// - 非 CLI 审批相关配置
#[derive(Debug)]
pub struct ApprovalManager {
    /// 自动批准的工具集合（配置 + 运行时更新）
    ///
    /// 在此集合中的工具调用将跳过审批流程。
    pub(super) auto_approve: RwLock<HashSet<String>>,
    /// 始终需要确认的工具集合（配置 + 运行时更新）
    ///
    /// 即使工具在会话白名单中，此集合中的工具仍需审批。
    pub(super) always_ask: RwLock<HashSet<String>>,
    /// 配置中的自主运行级别
    pub(super) autonomy_level: AutonomyLevel,
    /// 会话级白名单（来自用户的"Always"响应）
    pub(super) session_allowlist: Mutex<HashSet<String>>,
    /// 非 CLI 通道的会话级白名单（经人工明确批准后）
    pub(super) non_cli_allowlist: Mutex<HashSet<String>>,
    /// 非 CLI"一次性允许所有工具"的剩余令牌数
    ///
    /// 当此值大于 0 时，非 CLI 通道可以跳过一轮完整的工具循环提示。
    pub(super) non_cli_allow_all_once_remaining: Mutex<u32>,
    /// 允许管理非 CLI 审批的发送者白名单（可选）
    pub(super) non_cli_approval_approvers: RwLock<HashSet<String>>,
    /// 非 CLI 审批管理命令的默认自然语言处理模式
    pub(super) non_cli_natural_language_approval_mode: RwLock<NonCliNaturalLanguageApprovalMode>,
    /// 按通道覆盖的自然语言审批模式（可选）
    pub(super) non_cli_natural_language_approval_mode_by_channel:
        RwLock<HashMap<String, NonCliNaturalLanguageApprovalMode>>,
    /// 待处理的非 CLI 审批请求队列
    pub(super) pending_non_cli_requests: Mutex<HashMap<String, PendingNonCliApprovalRequest>>,
    /// 已解析的非 CLI 请求决策快照
    ///
    /// 当待处理请求被确认或拒绝后，决策会存储在此处供等待的工具循环消费。
    pub(super) resolved_non_cli_requests: Mutex<HashMap<String, ApprovalResponse>>,
    /// 审批决策的审计日志
    pub(super) audit_log: Mutex<Vec<ApprovalLogEntry>>,
}
