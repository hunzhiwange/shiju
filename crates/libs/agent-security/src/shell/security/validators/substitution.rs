//! 命令替换与进程替换 validator。
//!
//! `$(...)`、反引号和 `<(...)` 会在可见命令内部执行额外命令或创建隐式数据流；
//! 严格模式下阻断，宽松模式下降级为警告。

use super::{SecurityCategory, SecurityValidator, block, info, raw, warn};
use crate::shell::ast::ParsedCommand;

pub(super) struct SubstitutionValidator {
    strict: bool,
}

impl SubstitutionValidator {
    /// 创建替换校验器。
    ///
    /// 参数：
    /// - `strict`：是否把替换语法作为阻断级风险。
    ///
    /// 返回值：新的校验器实例。
    /// 错误处理：该函数不返回错误。
    pub(super) fn new(strict: bool) -> Self {
        Self { strict }
    }
}

impl SecurityValidator for SubstitutionValidator {
    fn name(&self) -> &str {
        "substitution"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let mut findings = Vec::new();
        let command = raw(cmd);
        // AST 标志优先，fallback 情况下再用文本包含检查保持保守覆盖。
        let has_command_substitution = info(cmd)
            .map(|info| info.has_command_substitution || command.contains('`'))
            .unwrap_or_else(|| command.contains("$(") || command.contains('`'));
        let has_process_substitution = info(cmd)
            .map(|info| info.has_process_substitution)
            .unwrap_or_else(|| command.contains("<(") || command.contains(">("));

        if has_command_substitution {
            findings.push(if self.strict {
                block(
                    SecurityCategory::Injection,
                    "Command substitution is blocked in strict shell validation mode",
                    Some("Expand the command first and pass the literal value"),
                )
            } else {
                warn(
                    SecurityCategory::Injection,
                    "Command substitution detected",
                    Some("Prefer literal arguments over command substitution"),
                )
            });
        }

        if has_process_substitution {
            findings.push(if self.strict {
                block(
                    SecurityCategory::Injection,
                    "Process substitution is blocked in strict shell validation mode",
                    Some("Materialize the intermediate file explicitly instead of using <(...)"),
                )
            } else {
                warn(
                    SecurityCategory::Injection,
                    "Process substitution detected",
                    Some("Use an explicit temporary file or pipe"),
                )
            });
        }

        findings
    }
}
