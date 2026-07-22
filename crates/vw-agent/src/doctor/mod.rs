//!
//! # 诊断模块 (Doctor Module)
//!
//! 本模块提供 VibeWindow 代理系统的健康检查与诊断功能。
//!
//! ## 核心功能
//!
//! - **配置语义验证**：检查配置文件完整性、提供者有效性、模型配置、温度范围、网关端口等
//! - **工作空间完整性**：验证工作目录存在性、可写性、磁盘空间
//! - **守护进程状态**：检查守护进程心跳、调度器健康度、通道状态
//! - **运行环境检查**：验证必要的系统工具（git、shell、curl 等）
//! - **CLI 工具发现**：扫描并报告可用的命令行工具
//! - **模型探测**：提供者模型目录探测（当前已禁用）
//! - **运行时追踪**：查询和展示运行时事件追踪数据

use crate::app::agent::config::Config;
use anyhow::Result;

mod config_checks;
mod daemon_checks;
mod environment_checks;
mod model_probe;
mod traces;
mod utils;
mod workspace_checks;

#[cfg(test)]
#[path = "config_checks_tests.rs"]
mod config_checks_tests;
#[cfg(test)]
#[path = "daemon_checks_tests.rs"]
mod daemon_checks_tests;
#[cfg(test)]
#[path = "environment_checks_tests.rs"]
mod environment_checks_tests;
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "model_probe_tests.rs"]
mod model_probe_tests;
#[cfg(test)]
#[path = "traces_tests.rs"]
mod traces_tests;
#[cfg(test)]
#[path = "utils_tests.rs"]
mod utils_tests;
#[cfg(test)]
#[path = "workspace_checks_tests.rs"]
mod workspace_checks_tests;

use self::config_checks::check_config_semantics;
use self::daemon_checks::check_daemon_state;
use self::environment_checks::{check_cli_tools, check_environment};
pub use self::model_probe::run_models;
pub use self::traces::run_traces;
use self::workspace_checks::check_workspace;

#[cfg(test)]
use self::config_checks::provider_validation_error;
#[cfg(test)]
use self::model_probe::{ModelProbeOutcome, classify_model_probe_error};
#[cfg(test)]
use self::utils::truncate_for_display;
#[cfg(test)]
use self::workspace_checks::{parse_df_available_mb, workspace_probe_path};

/// 守护进程心跳过期阈值（秒）
const DAEMON_STALE_SECONDS: i64 = 30;

/// 调度器健康检查过期阈值（秒）
const SCHEDULER_STALE_SECONDS: i64 = 120;

/// 通道健康检查过期阈值（秒）
const CHANNEL_STALE_SECONDS: i64 = 300;

/// 命令版本信息显示的最大字符数
const COMMAND_VERSION_PREVIEW_CHARS: usize = 60;

/// 诊断结果的严重性等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Ok,
    Warn,
    Error,
}

/// 结构化诊断结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct DiagResult {
    pub severity: Severity,
    pub category: String,
    pub message: String,
}

/// 内部诊断项（私有）
struct DiagItem {
    severity: Severity,
    category: &'static str,
    message: String,
}

impl DiagItem {
    fn ok(category: &'static str, msg: impl Into<String>) -> Self {
        Self { severity: Severity::Ok, category, message: msg.into() }
    }

    fn warn(category: &'static str, msg: impl Into<String>) -> Self {
        Self { severity: Severity::Warn, category, message: msg.into() }
    }

    fn error(category: &'static str, msg: impl Into<String>) -> Self {
        Self { severity: Severity::Error, category, message: msg.into() }
    }

    fn icon(&self) -> &'static str {
        match self.severity {
            Severity::Ok => "✅",
            Severity::Warn => "⚠️ ",
            Severity::Error => "❌",
        }
    }

    fn into_result(self) -> DiagResult {
        DiagResult {
            severity: self.severity,
            category: self.category.to_string(),
            message: self.message,
        }
    }
}

impl DiagResult {
    fn icon(&self) -> &'static str {
        match self.severity {
            Severity::Ok => "✅",
            Severity::Warn => "⚠️ ",
            Severity::Error => "❌",
        }
    }
}

/// 运行诊断检查并返回结构化结果
pub fn diagnose(config: &Config) -> Vec<DiagResult> {
    let mut items: Vec<DiagItem> = Vec::new();

    check_config_semantics(config, &mut items);
    check_workspace(config, &mut items);
    check_daemon_state(config, &mut items);
    check_environment(&mut items);
    check_cli_tools(&mut items);

    items.into_iter().map(DiagItem::into_result).collect()
}

/// 运行诊断检查并打印人类可读的报告
pub fn run(config: &Config) -> Result<()> {
    let results = diagnose(config);

    println!("🩺 VibeWindow Doctor (enhanced)");
    println!();

    let mut current_cat = "";
    for item in &results {
        if item.category != current_cat {
            current_cat = &item.category;
            println!("  [{current_cat}]");
        }
        println!("    {} {}", item.icon(), item.message);
    }

    let errors = results.iter().filter(|i| i.severity == Severity::Error).count();
    let warns = results.iter().filter(|i| i.severity == Severity::Warn).count();
    let oks = results.iter().filter(|i| i.severity == Severity::Ok).count();

    println!();
    println!("  Summary: {oks} ok, {warns} warnings, {errors} errors");

    if errors > 0 {
        println!("  💡 Fix the errors above, then run `vibewindow doctor` again.");
    }

    Ok(())
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
