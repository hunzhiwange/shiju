//! 权限请求与响应处理模块
//!
//! 本模块实现了代理系统的核心权限管理逻辑，包括：
//! - 权限规则的定义、解析和评估
//! - 权限请求的创建、排队和处理
//! - 用户响应的处理（允许/拒绝/永久允许）
//! - 全局权限状态的管理
//!
//! # 核心概念
//!
//! ## 权限规则（Rule）
//!
//! 每条规则包含：
//! - `permission`: 权限标识符（如 "edit"、"read"、"bash"）
//! - `pattern`: 目标模式（支持通配符，如 "*"、"src/**/*.rs"）
//! - `action`: 动作类型（Allow/Deny/Ask）
//!
//! ## 权限评估流程
//!
//! 1. 合并所有规则集
//! 2. 从后向前查找匹配的规则（后定义的规则优先级更高）
//! 3. 如果没有匹配规则，默认返回 Ask
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::agent::permission::next::{Action, Rule, evaluate, from_config};
//! use serde_json::json;
//!
//! // 从配置解析规则
//! let config = json!({
//!     "edit": { "src/**": "allow" },
//!     "read": "allow",
//!     "bash": "ask"
//! });
//! let ruleset = from_config(&config);
//!
//! // 评估权限
//! let rule = evaluate("edit", "src/main.rs", &[ruleset]);
//! assert_eq!(rule.action, Action::Allow);
//! ```

pub use vw_shared::permission::{Action, Rule, Ruleset};

use crate::app::agent::global;
use crate::app::agent::id;
use permission_engine::wildcard_match;
pub use permission_engine::{disabled, evaluate, merge};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

/// 展开路径模式中的主目录符号
///
/// 将 `~` 和 `$HOME` 替换为实际的用户主目录路径。
///
/// # 参数
///
/// * `pattern` - 可能包含 `~` 或 `$HOME` 的路径模式
///
/// # 返回值
///
/// 返回展开后的路径字符串。
///
/// # 展开规则
///
/// - `~` → 用户主目录
/// - `~/path` → 用户主目录/path
/// - `$HOME` → 用户主目录
/// - `$HOME/path` → 用户主目录/path
/// - 其他 → 原样返回
///
/// # 示例
///
/// ```ignore
/// // 假设主目录为 /Users/user
/// assert_eq!(expand("~"), "/Users/user");
/// assert_eq!(expand("~/Documents"), "/Users/user/Documents");
/// assert_eq!(expand("$HOME"), "/Users/user");
/// assert_eq!(expand("/usr/local"), "/usr/local");
/// ```
fn expand(pattern: &str) -> String {
    let home = global::paths().home.to_string_lossy().to_string();

    // 处理单独的 ~
    if pattern == "~" {
        return home;
    }

    // 处理 ~/path 形式
    if let Some(rest) = pattern.strip_prefix("~/") {
        return format!("{home}/{rest}");
    }

    // 处理单独的 $HOME
    if pattern == "$HOME" {
        return home;
    }

    // 处理 $HOME/path 形式
    if let Some(rest) = pattern.strip_prefix("$HOME/") {
        return format!("{home}/{rest}");
    }

    pattern.to_string()
}

/// 从配置解析权限规则，并使用应用主目录展开路径模式。
pub fn from_config(permission: &Value) -> Ruleset {
    permission_engine::from_config(permission, expand)
}

/// 工具调用信息
///
/// 记录与权限请求关联的工具调用详情。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// 关联的消息 ID
    pub message_id: String,
    /// 工具调用的 ID
    pub call_id: String,
}

/// 权限请求
///
/// 表示一个等待处理的权限请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// 请求唯一标识符
    pub id: String,
    /// 会话 ID
    pub session_id: String,
    /// 权限类型（如 "edit"、"read"、"bash"）
    pub permission: String,
    /// 目标模式列表（如 ["src/main.rs", "lib/mod.rs"]）
    pub patterns: Vec<String>,
    /// 附加元数据
    pub metadata: Map<String, Value>,
    /// 永久允许的模式列表（用于 Reply::Always 响应）
    pub always: Vec<String>,
    /// 关联的工具信息（可选）
    pub tool: Option<ToolInfo>,
}

/// 用户响应类型
///
/// 定义了用户对权限请求的可能响应。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reply {
    /// 仅本次允许
    Once,
    /// 永久允许（将添加到已批准规则）
    Always,
    /// 拒绝
    Reject,
}

/// 权限错误类型
///
/// 定义了权限处理过程中可能出现的各种错误。
#[derive(Debug)]
pub enum Error {
    /// 用户明确拒绝了该操作
    Rejected,
    /// 用户拒绝并提供了修正反馈
    Corrected(String),
    /// 规则明确禁止该操作
    Denied(Ruleset),
    /// 请求待处理，等待用户响应
    Pending(Request),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Rejected => {
                write!(f, "The user rejected permission to use this specific tool call.")
            }
            Error::Corrected(msg) => write!(
                f,
                "The user rejected permission to use this specific tool call with the following feedback: {}",
                msg
            ),
            Error::Denied(ruleset) => write!(
                f,
                "The user has specified a rule which prevents you from using this specific tool call. Here are some of the relevant rules {}",
                serde_json::to_string(ruleset).unwrap_or_else(|_| "[]".to_string())
            ),
            Error::Pending(_) => write!(f, "Permission requires user approval."),
        }
    }
}

impl std::error::Error for Error {}

/// 权限 ID 计数器
///
/// 用于生成唯一的权限请求 ID。
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// 生成下一个权限请求 ID
///
/// 优先使用 ID 生成器，失败时使用原子计数器作为后备。
///
/// # 返回值
///
/// 返回格式为 "permission-{n}" 或通过 ID 生成器生成的 ID。
fn next_id() -> String {
    id::ascending(id::Prefix::Permission, None)
        .unwrap_or_else(|_| format!("permission-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed)))
}

/// 全局权限状态
///
/// 存储待处理的权限请求和已批准的规则。
#[derive(Default)]
struct State {
    /// 待处理的权限请求映射（请求 ID -> 请求）
    pending: HashMap<String, Request>,
    /// 用户已批准的规则（通过 Reply::Always 添加）
    approved: Ruleset,
}

/// 全局状态实例
///
/// 使用互斥锁保护，确保线程安全。
static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));

/// 列出所有待处理的权限请求
///
/// # 返回值
///
/// 返回所有待处理请求的列表。
///
/// # 示例
///
/// ```ignore
/// let pending = list();
/// for req in pending {
///     println!("Pending request: {} for {}", req.id, req.permission);
/// }
/// ```
pub fn list() -> Vec<Request> {
    STATE.lock().ok().map(|s| s.pending.values().cloned().collect()).unwrap_or_default()
}

/// 请求权限
///
/// 检查给定的权限请求是否被允许，并根据规则决定是否需要用户确认。
///
/// # 参数
///
/// * `req` - 权限请求
/// * `ruleset` - 权限规则集
///
/// # 返回值
///
/// - `Ok(())`: 权限被允许
/// - `Err(Error::Denied)`: 规则明确拒绝
/// - `Err(Error::Pending)`: 需要用户确认
///
/// # 评估流程
///
/// 对每个目标模式：
/// 1. 合并规则集和已批准规则
/// 2. 评估权限
/// 3. 根据动作类型决定结果：
///    - Allow: 继续检查下一个模式
///    - Deny: 立即返回拒绝错误
///    - Ask: 将请求添加到待处理队列
///
/// # 示例
///
/// ```ignore
/// let req = Request {
///     id: String::new(),
///     session_id: "session-1".into(),
///     permission: "edit".into(),
///     patterns: vec!["src/main.rs".into()],
///     metadata: Map::new(),
///     always: vec![],
///     tool: None,
/// };
///
/// match ask(req, &ruleset) {
///     Ok(()) => println!("Permission granted"),
///     Err(Error::Pending(r)) => println!("Waiting for user approval: {}", r.id),
///     Err(Error::Denied(rules)) => println!("Denied by rules: {:?}", rules),
///     _ => {}
/// }
/// ```
pub fn ask(mut req: Request, ruleset: &Ruleset) -> Result<(), Error> {
    for pat in &req.patterns {
        // 合并规则集和已批准规则
        let rule = evaluate(&req.permission, pat, &[ruleset.clone(), approved_rules()]);

        match rule.action {
            // 允许：继续检查下一个模式
            Action::Allow => {}

            // 拒绝：返回相关规则
            Action::Deny => {
                let relevant = ruleset
                    .iter()
                    .filter(|r| wildcard_match(&req.permission, &r.permission))
                    .cloned()
                    .collect::<Vec<_>>();
                return Err(Error::Denied(relevant));
            }

            // 询问：添加到待处理队列
            Action::Ask => {
                // 如果请求没有 ID，生成一个
                if req.id.is_empty() {
                    req.id = next_id();
                }

                // 将请求添加到全局状态
                let mut lock = STATE.lock().map_err(|_| Error::Rejected)?;
                lock.pending.insert(req.id.clone(), req.clone());
                return Err(Error::Pending(req));
            }
        }
    }
    Ok(())
}

/// 获取已批准的规则
///
/// 从全局状态中获取用户通过 "Always" 响应批准的规则。
///
/// # 返回值
///
/// 返回已批准规则的副本。如果锁获取失败，返回空规则集。
fn approved_rules() -> Ruleset {
    STATE.lock().ok().map(|s| s.approved.clone()).unwrap_or_default()
}

/// 响应权限请求
///
/// 处理用户对权限请求的响应。
///
/// # 参数
///
/// * `request_id` - 请求 ID
/// * `reply` - 用户响应类型
/// * `message` - 可选的反馈消息（用于 Reject）
///
/// # 返回值
///
/// - `Ok(Some(Request))`: 请求被批准（Once 或 Always），返回原请求
/// - `Ok(None)`: 请求不存在
/// - `Err(Error::Rejected)`: 请求被拒绝，无反馈
/// - `Err(Error::Corrected)`: 请求被拒绝，有反馈
///
/// # 响应处理
///
/// - `Reply::Reject`: 返回错误，可选带反馈消息
/// - `Reply::Once`: 返回原请求，不添加到已批准规则
/// - `Reply::Always`: 将请求的 `always` 模式添加到已批准规则，返回原请求
///
/// # 示例
///
/// ```ignore
/// // 用户批准一次
/// match reply("req-123", Reply::Once, None) {
///     Ok(Some(req)) => execute_tool(req),
///     Ok(None) => println!("Request not found"),
///     Err(e) => println!("Error: {}", e),
/// }
///
/// // 用户永久批准
/// match reply("req-123", Reply::Always, None) {
///     Ok(Some(req)) => {
///         println!("Permission will be remembered");
///         execute_tool(req);
///     }
///     _ => {}
/// }
///
/// // 用户拒绝并提供反馈
/// match reply("req-123", Reply::Reject, Some("Use a different file".into())) {
///     Err(Error::Corrected(msg)) => println!("User feedback: {}", msg),
///     _ => {}
/// }
/// ```
pub fn reply(
    request_id: &str,
    reply: Reply,
    message: Option<String>,
) -> Result<Option<Request>, Error> {
    let mut lock = STATE.lock().map_err(|_| Error::Rejected)?;
    let existing = lock.pending.remove(request_id);

    let Some(req) = existing else {
        return Ok(None);
    };

    match reply {
        // 拒绝：返回错误，可选带反馈
        Reply::Reject => {
            if let Some(msg) = message {
                Err(Error::Corrected(msg))
            } else {
                Err(Error::Rejected)
            }
        }

        // 仅本次允许：返回请求
        Reply::Once => Ok(Some(req)),

        // 永久允许：添加到已批准规则
        Reply::Always => {
            for pat in &req.always {
                lock.approved.push(Rule {
                    permission: req.permission.clone(),
                    pattern: pat.clone(),
                    action: Action::Allow,
                });
            }
            Ok(Some(req))
        }
    }
}

/// 重置权限状态
///
/// 清除所有待处理的请求和已批准的规则。
///
/// # 用途
///
/// 在会话结束或需要完全重置权限状态时调用。
///
/// # 示例
///
/// ```ignore
/// // 会话结束时重置
/// reset();
/// assert!(list().is_empty());
/// ```
pub fn reset() {
    if let Ok(mut s) = STATE.lock() {
        s.pending.clear();
        s.approved.clear();
    }
}

/// 展开主目录符号（公开接口）
///
/// 提供对内部 `expand` 函数的公开访问。
///
/// # 参数
///
/// * `pattern` - 可能包含 `~` 或 `$HOME` 的路径模式
///
/// # 返回值
///
/// 返回展开后的路径字符串。
///
/// # 示例
///
/// ```ignore
/// let expanded = expand_home("~/Documents");
/// // 结果如 "/Users/user/Documents" 或 "/home/user/Documents"
/// ```
pub fn expand_home(pattern: &str) -> String {
    expand(pattern)
}

/// 获取用户主目录路径
///
/// # 返回值
///
/// 返回用户主目录的路径。
///
/// # 示例
///
/// ```ignore
/// let home = home_dir();
/// println!("Home directory: {:?}", home);
/// ```
pub fn home_dir() -> PathBuf {
    global::paths().home.clone()
}

#[cfg(test)]
#[path = "next_tests.rs"]
mod next_tests;
