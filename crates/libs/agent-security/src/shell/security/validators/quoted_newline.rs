//! 引号内换行 validator。
//!
//! 引号内部的换行会让多行命令伪装成单个字符串参数，影响审阅和日志判断，因此阻断。

use super::{SecurityCategory, SecurityValidator, block, has_quoted_newline, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct QuotedNewlineValidator;

impl SecurityValidator for QuotedNewlineValidator {
    fn name(&self) -> &str {
        "quoted_newline"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        if has_quoted_newline(raw(cmd)) {
            return vec![block(
                SecurityCategory::Injection,
                "Quoted newlines can hide multi-line shell injection",
                Some("Keep quoted strings on a single line"),
            )];
        }
        Vec::new()
    }
}
