//! 危险 shell 变量 validator。
//!
//! 阻断会改变 shell 分词、执行上下文或暴露执行字符串的变量引用，同时对环境指纹变量
//! 给出警告。

use super::{SecurityCategory, SecurityValidator, block, raw, warn};
use crate::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;

static BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        \$BASH_EXECUTION_STRING
        |\$ENV\b
        |\$\{PATH##[^}]+\}
        |\bIFS=
        |\$\{?IFS\}?
    ",
    )
    .expect("valid dangerous vars regex")
});
static WARN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\$(SHLVL|SECONDS)\b").expect("valid fingerprint regex"));

pub(super) struct DangerousVarsValidator;

impl SecurityValidator for DangerousVarsValidator {
    fn name(&self) -> &str {
        "dangerous_vars"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let command = raw(cmd);
        let mut findings = Vec::new();
        if BLOCK_RE.is_match(command) {
            // IFS 与执行字符串类变量会改变命令解释方式或泄露上下文，不能在自动 shell
            // 执行路径中默许。
            findings.push(block(
                SecurityCategory::PrivilegeEscalation,
                "Command references dangerous shell variables or mutates IFS",
                Some("Remove variable-based shell reconfiguration from the command"),
            ));
        }
        if WARN_RE.is_match(command) {
            // 指纹变量不一定危险，但常用于探测运行环境，保留为警告便于审计。
            findings.push(warn(
                SecurityCategory::UnsafePattern,
                "Command inspects shell fingerprinting variables such as SHLVL or SECONDS",
                Some("Avoid environment fingerprinting unless it is required"),
            ));
        }
        findings
    }
}
#[cfg(test)]
#[path = "dangerous_vars_tests.rs"]
mod dangerous_vars_tests;
