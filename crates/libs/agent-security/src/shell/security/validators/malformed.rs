//! malformed shell 输入 validator。
//!
//! 当 shell 解析器只能产出部分 AST 或 fallback token 为空时，命令边界不可信，应按
//! 注入风险阻断而不是继续执行。

use super::{SecurityCategory, SecurityValidator, block};
use crate::shell::ast::{ParseQuality, ParsedCommand};

pub(super) struct MalformedValidator;

impl SecurityValidator for MalformedValidator {
    fn name(&self) -> &str {
        "malformed"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        match cmd {
            ParsedCommand::Ast(ast, _) if ast.quality() == ParseQuality::Partial => vec![block(
                SecurityCategory::Injection,
                "Shell parser reported a partially malformed command",
                Some("Rewrite the command with balanced quoting and complete tokens"),
            )],
            ParsedCommand::Fallback { raw, tokens }
                if !raw.trim().is_empty() && tokens.is_empty() =>
            {
                vec![block(
                    SecurityCategory::Injection,
                    "Command could not be tokenized safely",
                    Some("Avoid malformed quoting or unsupported shell syntax"),
                )]
            }
            _ => Vec::new(),
        }
    }
}
#[cfg(test)]
#[path = "malformed_tests.rs"]
mod malformed_tests;
