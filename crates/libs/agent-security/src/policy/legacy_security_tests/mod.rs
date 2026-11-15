//! 安全策略测试模块入口。
//!
//! 这里集中声明策略相关测试子模块，并提供构造常用策略的测试辅助函数。测试
//! 仍按行为拆分到独立文件，避免把授权、路径、配置和限流场景混在一起。

use std::path::PathBuf;

use config_security::AutonomyConfig;
use super::{
    ActionTracker, AutonomyLevel, CommandRiskLevel, SecurityPolicy, ShellRedirectPolicy,
    ToolOperation,
};

mod autonomy;
mod command_policy;
mod config;
mod path_policy;
mod shell_safety;
mod tracker;

/// 构造默认监督模式策略。
fn default_policy() -> SecurityPolicy {
    SecurityPolicy::default()
}

/// 构造只读自治策略，用于验证写操作与高风险路径被拒绝。
fn readonly_policy() -> SecurityPolicy {
    SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() }
}

/// 构造完全自治策略，用于验证无需监督批准的路径。
fn full_policy() -> SecurityPolicy {
    SecurityPolicy { autonomy: AutonomyLevel::Full, ..SecurityPolicy::default() }
}
