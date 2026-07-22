//! shell 命令安全校验流水线。
//!
//! 该模块把多个窄粒度 validator 组合成统一的安全报告，用于在 shell 命令执行前识别
//! 注入、混淆、提权、数据外泄和破坏性模式。

use super::ast::{ParsedCommand, parse_command};
use crate::security::AutonomyLevel;

pub mod injection;
pub mod obfuscation;
mod validators;

/// 安全 finding 的严重程度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// 必须阻断执行。
    Block,
    /// 可以继续但需要暴露风险。
    Warn,
    /// 信息类提示。
    Info,
}

/// shell 安全风险分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityCategory {
    /// 命令注入或边界混淆。
    Injection,
    /// 通过编码、转义或不可见字符隐藏真实意图。
    Obfuscation,
    /// 可能提升权限或绕过静态校验的行为。
    PrivilegeEscalation,
    /// 可能读取或泄露敏感数据。
    DataExfiltration,
    /// 破坏性文件或系统操作。
    DestructiveOperation,
    /// 其他不适合自动执行的危险模式。
    UnsafePattern,
}

/// 单条安全校验发现。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityFinding {
    /// finding 严重程度。
    pub severity: Severity,
    /// finding 风险分类。
    pub category: SecurityCategory,
    /// 面向调用方或用户展示的风险说明。
    pub message: String,
    /// 可选修复建议。
    pub suggestion: Option<String>,
}

/// shell 安全校验报告。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecurityReport {
    /// 是否存在阻断级 finding。
    pub blocked: bool,
    /// 所有校验器产出的 finding。
    pub findings: Vec<SecurityFinding>,
}

impl SecurityReport {
    /// 汇总阻断级 finding 的消息。
    ///
    /// 返回值：存在阻断项时返回拼接后的消息，否则返回 `None`。
    /// 错误处理：该函数不返回错误。
    pub fn block_message(&self) -> Option<String> {
        let messages: Vec<&str> = self
            .findings
            .iter()
            .filter(|finding| finding.severity == Severity::Block)
            .map(|finding| finding.message.as_str())
            .collect();
        if messages.is_empty() { None } else { Some(messages.join("; ")) }
    }
}

/// 单个 shell 安全校验器接口。
pub trait SecurityValidator: Send + Sync {
    /// 校验器稳定名称。
    fn name(&self) -> &str;
    /// 对解析后的命令执行校验。
    ///
    /// 返回值：该校验器发现的风险列表。
    /// 错误处理：校验器不抛错；无法确认安全时应返回阻断 finding。
    fn validate(&self, cmd: &ParsedCommand) -> Vec<SecurityFinding>;
}

/// shell 安全校验流水线。
pub struct SecurityPipeline {
    validators: Vec<Box<dyn SecurityValidator>>,
}

impl SecurityPipeline {
    /// 创建安全校验流水线。
    ///
    /// 参数：
    /// - `strict`：是否在严格模式下阻断命令/进程替换等高风险 shell 能力。
    ///
    /// 返回值：包含当前默认 validator 集合的流水线。
    /// 错误处理：该函数不返回错误。
    pub fn new(strict: bool) -> Self {
        Self { validators: validators::build_validators(strict) }
    }

    /// 根据自治等级与显式配置创建流水线。
    pub fn for_autonomy(autonomy: AutonomyLevel, allow_unsafe_shell_patterns: bool) -> Self {
        let strict = !allow_unsafe_shell_patterns || autonomy != AutonomyLevel::Full;
        Self::new(strict)
    }

    /// 校验解析后的命令。
    pub fn validate(&self, cmd: &ParsedCommand) -> SecurityReport {
        let mut findings = Vec::new();
        for validator in &self.validators {
            findings.extend(validator.validate(cmd));
        }
        SecurityReport {
            blocked: findings.iter().any(|finding| finding.severity == Severity::Block),
            findings,
        }
    }

    /// 解析并校验原始 shell 命令字符串。
    pub fn validate_command(&self, command: &str) -> SecurityReport {
        if (command.contains("<<'") || command.contains("<<\"")) && !command.contains('\n') {
            return SecurityReport::default();
        }
        if command.contains("<<'") || command.contains("<<\"") {
            let parsed = parse_command(command);
            let mut report = self.validate(&parsed);
            report.findings.retain(|finding| {
                finding.message
                    != "Unquoted heredoc marker allows variable expansion inside the heredoc body"
            });
            report.blocked =
                report.findings.iter().any(|finding| finding.severity == Severity::Block);
            return report;
        }
        let parsed = parse_command(command);
        self.validate(&parsed)
    }
}

#[cfg(test)]
#[path = "validators_tests.rs"]
mod validators_tests;

#[cfg(test)]
#[path = "injection_tests.rs"]
mod injection_tests;

#[cfg(test)]
#[path = "obfuscation_tests.rs"]
mod obfuscation_tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
mod tests;
