//! # 命令日志记录钩子
//!
//! 提供工具调用审计功能的钩子实现，用于记录所有工具执行结果。
//!
//! ## 功能特性
//!
//! - 记录工具调用的时间戳、名称、执行时长和结果状态
//! - 支持多线程安全的日志存储
//! - 自动输出到 tracing 日志系统
//!
//! ## 使用场景
//!
//! - 审计追踪：记录所有工具调用历史
//! - 性能分析：统计工具执行耗时
//! - 调试排障：回溯工具执行序列
//!
//! ## 优先级说明
//!
//! 该钩子优先级为 `-50`（较低），确保在其他业务钩子执行完毕后再记录日志。

use async_trait::async_trait;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::app::agent::hooks::traits::HookHandler;
use crate::app::agent::tools::traits::ToolResult;

/// 默认保留的命令审计条目数。
pub const DEFAULT_COMMAND_AUDIT_CAPACITY: usize = 256;

/// 不包含工具参数、输出或错误正文的安全审计条目。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandAuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool_name: String,
    pub duration_ms: u64,
    pub success: bool,
}

/// 可克隆、有界的命令审计存储。
#[derive(Clone)]
pub struct CommandAuditStore {
    entries: Arc<Mutex<VecDeque<CommandAuditEntry>>>,
    capacity: usize,
}

impl CommandAuditStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.max(1)))),
            capacity: capacity.max(1),
        }
    }

    pub fn entries(&self) -> Vec<CommandAuditEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .cloned()
            .collect()
    }

    fn push(&self, entry: CommandAuditEntry) {
        let mut entries = self.entries.lock().unwrap_or_else(|error| error.into_inner());
        if entries.len() == self.capacity {
            entries.pop_front();
        }
        entries.push_back(entry);
    }
}

impl Default for CommandAuditStore {
    fn default() -> Self {
        Self::new(DEFAULT_COMMAND_AUDIT_CAPACITY)
    }
}

/// 命令日志记录钩子
///
/// 该钩子实现 [`HookHandler`] trait，用于在工具调用完成后记录审计日志。
/// 安全字段存储在有界、线程安全的内存缓冲区中，同时输出到 tracing 系统。
///
/// # 线程安全性
///
/// 内部审计存储支持多线程并发写入，并在达到容量后淘汰最旧条目。
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::hooks::builtin::CommandLoggerHook;
/// use crate::app::agent::hooks::traits::HookHandler;
///
/// let hook = CommandLoggerHook::new();
/// assert_eq!(hook.name(), "command-logger");
/// assert_eq!(hook.priority(), -50);
/// ```
pub struct CommandLoggerHook {
    audit_store: CommandAuditStore,
}

impl CommandLoggerHook {
    /// 创建新的命令日志记录钩子实例
    ///
    /// 初始化一个空的日志缓冲区，准备接收工具调用记录。
    ///
    /// # 返回值
    ///
    /// 返回一个新创建的 [`CommandLoggerHook`] 实例，其日志缓冲区为空。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let hook = CommandLoggerHook::new();
    /// ```
    pub fn new() -> Self {
        Self::with_store(CommandAuditStore::default())
    }

    pub fn with_store(audit_store: CommandAuditStore) -> Self {
        Self { audit_store }
    }

    /// 获取已记录的所有结构化审计条目（仅测试可用）
    ///
    /// 返回日志缓冲区的完整副本，用于测试验证。
    ///
    /// # 返回值
    ///
    /// 返回所有已记录日志条目的克隆向量。
    ///
    /// # 注意
    ///
    /// 此方法仅在测试配置下可用（`#[cfg(test)]`）。
    #[cfg(test)]
    pub fn entries(&self) -> Vec<CommandAuditEntry> {
        self.audit_store.entries()
    }
}

/// [`HookHandler`] trait 实现
///
/// 为 [`CommandLoggerHook`] 实现钩子处理接口，使其能够接入
/// VibeWindow 的钩子系统。使用条件编译属性以同时支持
/// WASM 和非 WASM 目标平台。
///
/// # 平台兼容性
///
/// - `wasm32` 目标：使用 `?Send` 标记，允许非 Send future
/// - 其他目标：标准 `async_trait` 实现
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl HookHandler for CommandLoggerHook {
    /// 返回钩子名称标识
    ///
    /// 该名称用于在日志和调试输出中标识此钩子实例。
    ///
    /// # 返回值
    ///
    /// 返回静态字符串 `"command-logger"`。
    fn name(&self) -> &str {
        "command-logger"
    }

    /// 返回钩子执行优先级
    ///
    /// 优先级为 `-50`，属于较低优先级。这确保该日志钩子
    /// 在其他业务逻辑钩子执行完毕后才执行，避免影响主流程。
    ///
    /// # 优先级规则
    ///
    /// - 数值越大，优先级越高，越先执行
    /// - 负值表示低优先级，后执行
    ///
    /// # 返回值
    ///
    /// 返回 `-50`。
    fn priority(&self) -> i32 {
        -50
    }

    /// 工具调用完成后的回调处理
    ///
    /// 当工具执行完成后，此方法被调用以记录执行信息。
    /// 日志条目包含时间戳、工具名称、执行时长和成功状态。
    ///
    /// # 参数
    ///
    /// - `tool`: 工具名称标识符
    /// - `result`: 工具执行结果，包含成功状态和输出数据
    /// - `duration`: 工具执行耗时
    ///
    /// # 行为说明
    ///
    /// 1. 构造仅含时间、工具名、耗时与成功状态的结构化条目
    /// 2. 使用 tracing 输出 INFO 级别日志
    /// 3. 将条目追加到有界内存缓冲区
    async fn on_after_tool_call(&self, tool: &str, result: &ToolResult, duration: Duration) {
        let duration_ms = duration.as_millis().min(u128::from(u64::MAX)) as u64;
        let entry = CommandAuditEntry {
            timestamp: chrono::Utc::now(),
            tool_name: tool.to_string(),
            duration_ms,
            success: result.success,
        };

        tracing::info!(
            hook = "command-logger",
            tool = entry.tool_name,
            duration_ms = entry.duration_ms,
            success = entry.success,
            "tool call completed"
        );

        self.audit_store.push(entry);
    }
}

/// 单元测试模块
///
/// 测试文件位于同目录下的 `tests.rs`，通过 `#[path]` 属性指定路径。
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
