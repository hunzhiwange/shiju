//! 反斜杠混淆 validator。
//!
//! 阻断用反斜杠隐藏空白或 shell 操作符的命令，因为这类写法会让审阅文本与实际
//! shell token 边界不一致。

use super::{SecurityCategory, SecurityValidator, block, raw};
use crate::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;

static ESCAPED_WHITESPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\[ \t\n]").expect("valid escaped whitespace regex"));
static ESCAPED_OPERATOR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\\[;&|]").expect("valid escaped operator regex"));

pub(super) struct BackslashValidator;

impl SecurityValidator for BackslashValidator {
    fn name(&self) -> &str {
        "backslash"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let command = raw(cmd);
        if ESCAPED_WHITESPACE_RE.is_match(command) || ESCAPED_OPERATOR_RE.is_match(command) {
            return vec![block(
                SecurityCategory::Obfuscation,
                "Backslash escaping hides whitespace or shell operators",
                Some("Use normal whitespace and explicit operators"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "backslash_tests.rs"]
mod backslash_tests;
