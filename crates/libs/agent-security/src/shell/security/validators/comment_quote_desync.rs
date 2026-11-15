//! 注释与引号边界错位 validator。
//!
//! 当命令同时包含注释字符和未闭合 shell 边界时，审阅者看到的注释范围可能不同于
//! shell 实际解析结果，因此按注入风险阻断。

use super::{SecurityCategory, SecurityValidator, block, looks_unbalanced_shell, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct CommentQuoteDesyncValidator;

impl SecurityValidator for CommentQuoteDesyncValidator {
    fn name(&self) -> &str {
        "comment_quote_desync"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let command = raw(cmd);
        if command.contains('#') && looks_unbalanced_shell(command) {
            return vec![block(
                SecurityCategory::Injection,
                "Comment and quote boundaries are inconsistent",
                Some("Rewrite the command without mixing trailing comments and open quotes"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "comment_quote_desync_tests.rs"]
mod comment_quote_desync_tests;
