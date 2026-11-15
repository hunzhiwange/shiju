//! 安全策略共享类型。
//!
//! 本模块集中放置权限策略中跨文件使用的轻量枚举与外部配置类型重导出，避免
//! 主策略实现文件同时承担类型定义与授权流程。

/// 重导出配置层的自治级别，确保策略层与配置 schema 使用同一组取值。
pub use config_security::{AutonomyLevel, ShellRedirectPolicy};

use std::fmt;

/// Shell 命令的风险等级。
///
/// 风险等级用于决定是否需要人工批准，以及高风险命令是否被策略直接阻断。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRiskLevel {
    /// 只读或低影响操作。
    Low,
    /// 可能修改工作区或运行状态，需要在监督模式下批准。
    Medium,
    /// 可能造成破坏性影响、越权访问或系统级变更的操作。
    High,
}

/// 工具操作的权限类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolOperation {
    /// 只读取状态或文件，不消耗行动预算。
    Read,
    /// 会修改状态、执行命令或产生外部副作用的操作。
    Act,
}

/// Shell 字符扫描时的引用状态。
///
/// 该状态用于区分普通文本、单引号和双引号上下文，避免把引号内字符错误当作
/// shell 控制符处理。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteState {
    /// 当前不在引号内。
    None,
    /// 当前位于单引号内。
    Single,
    /// 当前位于双引号内。
    Double,
}

impl fmt::Display for CommandRiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandRiskLevel::Low => write!(f, "low"),
            CommandRiskLevel::Medium => write!(f, "medium"),
            CommandRiskLevel::High => write!(f, "high"),
        }
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
