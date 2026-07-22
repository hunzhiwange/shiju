//! 监督模式下的交互式审批工作流模块。
//!
//! 本模块提供了一个执行前钩子，用于在工具调用前提示用户进行确认。
//! 支持会话级别的"始终允许"白名单机制，并记录完整的审批决策审计日志。
//!
//! # 核心功能
//!
//! - **交互式审批提示**：在 CLI 环境下，代理执行工具前会向用户展示确认提示
//! - **会话白名单**：用户可以选择"始终允许"某个工具，后续调用无需再次确认
//! - **非 CLI 通道审批**：支持 Telegram、Slack 等非命令行通道的审批流程
//! - **审批请求队列**：管理待处理的审批请求，支持过期清理
//! - **审计日志**：记录所有审批决策，包括时间戳、工具名称、参数摘要等
//!
//! # 自主级别
//!
//! 系统支持三种自主运行级别（通过 [`AutonomyLevel`] 配置）：
//!
//! - `Full`：完全自主，无需任何审批
//! - `Supervised`：监督模式，需要用户审批（默认）
//! - `ReadOnly`：只读模式，禁止所有写操作
//!
//! # 示例
//!
//! ```ignore
//! use crate::app::agent::approval::{ApprovalManager, ApprovalRequest, ApprovalResponse};
//! use crate::app::agent::config::AutonomyConfig;
//!
//! // 从配置创建审批管理器
//! let config = AutonomyConfig::default();
//! let manager = ApprovalManager::from_config(&config);
//!
//! // 检查工具是否需要审批
//! if manager.needs_approval("shell") {
//!     let request = ApprovalRequest {
//!         tool_name: "shell".to_string(),
//!         arguments: serde_json::json!({"command": "ls -la"}),
//!     };
//!     let response = manager.prompt_cli(&request);
//!     manager.record_decision("shell", &request.arguments, response, "cli");
//! }
//! ```
mod cli;
mod manager;
mod pending;
mod policy;
mod summary;
mod types;

use cli::prompt_cli_interactive;
use summary::summarize_args;

pub use manager::ApprovalManager;
pub use types::{
    ApprovalLogEntry, ApprovalRequest, ApprovalResponse, PendingApprovalError,
    PendingNonCliApprovalRequest,
};

// ═══════════════════════════════════════════════════════════════════════════
// 测试模块
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;
#[cfg(test)]
#[path = "manager_tests.rs"]
mod manager_tests;
#[cfg(test)]
#[path = "pending_tests.rs"]
mod pending_tests;
#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
#[cfg(test)]
#[path = "summary_tests.rs"]
mod summary_tests;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
