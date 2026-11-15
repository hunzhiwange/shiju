//! Shell 权限模型入口，定义审批结果、执行上下文并重导出规则引擎。

use std::path::PathBuf;

use crate::policy::AutonomyLevel;
use crate::shell::security::SecurityFinding;

/// 声明 mode 子模块，保持当前领域的职责拆分清晰。
pub mod mode;
/// 声明 rules 子模块，保持当前领域的职责拆分清晰。
pub mod rules;
/// 声明 sandbox_allow 子模块，保持当前领域的职责拆分清晰。
pub mod sandbox_allow;
/// 声明 warning 子模块，保持当前领域的职责拆分清晰。
pub mod warning;

/// 重导出 mode::PermissionMode，保持外部调用路径稳定。
pub use mode::PermissionMode;
/// 重导出 rules::{PermissionRule, RuleAction, RuleCondition, RuleEngine, RulePattern}，保持外部调用路径稳定。
pub use rules::{PermissionRule, RuleAction, RuleCondition, RuleEngine, RulePattern};

/// Permission 枚举描述当前模块中一组明确的状态或分类。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    Allow,
    Deny { reason: String },
    Ask { reason: String, warning: Option<String> },
}

/// PermissionResult 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionResult {
    /// permission 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub permission: Option<Permission>,
    /// security_findings 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub security_findings: Vec<SecurityFinding>,
}

impl PermissionResult {
    /// 执行 allow 操作，并返回调用方需要的结果。
    pub fn allow() -> Self {
        Self { permission: Some(Permission::Allow), security_findings: Vec::new() }
    }

    /// 执行 deny 操作，并返回调用方需要的结果。
    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            permission: Some(Permission::Deny { reason: reason.into() }),
            security_findings: Vec::new(),
        }
    }

    /// 执行 ask 操作，并返回调用方需要的结果。
    pub fn ask(reason: impl Into<String>, warning: Option<String>) -> Self {
        Self {
            permission: Some(Permission::Ask { reason: reason.into(), warning }),
            security_findings: Vec::new(),
        }
    }

    /// 执行 with_findings 操作，并返回调用方需要的结果。
    pub fn with_findings(mut self, findings: Vec<SecurityFinding>) -> Self {
        self.security_findings = findings;
        self
    }
}

/// PermissionContext 结构体保存当前模块对外暴露的数据。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionContext {
    /// autonomy 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub autonomy: AutonomyLevel,
    /// in_sandbox 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub in_sandbox: bool,
    /// mode 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub mode: PermissionMode,
    /// approved 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub approved: bool,
    /// workspace_dir 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub workspace_dir: PathBuf,
    /// allowed_roots 字段由调用方显式提供或读取，避免隐藏默认行为。
    pub allowed_roots: Vec<PathBuf>,
}

impl PermissionContext {
    /// 执行 new 操作，并返回调用方需要的结果。
    pub fn new(autonomy: AutonomyLevel, workspace_dir: PathBuf) -> Self {
        Self {
            autonomy,
            in_sandbox: false,
            mode: PermissionMode::Normal,
            approved: false,
            workspace_dir,
            allowed_roots: Vec::new(),
        }
    }
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod rules_tests;

#[cfg(test)]
#[path = "sandbox_allow_tests.rs"]
mod sandbox_allow_tests;

#[cfg(test)]
#[path = "mode_tests.rs"]
mod mode_tests;

#[cfg(test)]
#[path = "warning_tests.rs"]
mod warning_tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
