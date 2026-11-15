//! 空命令与明显未闭合命令 validator。
//!
//! 空输入通常表示调用方构造错误；未闭合引号或括号可能让后续文本进入同一条 shell
//! 命令，因此统一按注入风险阻断。

use super::{SecurityCategory, SecurityValidator, block, looks_unbalanced_shell, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct EmptyCommandValidator;

impl SecurityValidator for EmptyCommandValidator {
    fn name(&self) -> &str {
        "empty_command"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let command = raw(cmd);
        if command.trim().is_empty() {
            return vec![block(
                SecurityCategory::Injection,
                "Command is empty or only contains whitespace",
                Some("Provide a complete shell command"),
            )];
        }
        if looks_unbalanced_shell(command) {
            return vec![block(
                SecurityCategory::Injection,
                "Command has unbalanced quotes or parentheses",
                Some("Close all quotes and grouping operators before execution"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "empty_command_tests.rs"]
mod empty_command_tests;
