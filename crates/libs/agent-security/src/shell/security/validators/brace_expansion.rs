//! brace expansion 混淆 validator。
//!
//! 阻断 `{a,b}` 与 `{1..9}` 这类展开语法，避免单个可见参数在执行时变成多个目标或
//! 大量资源访问。

use super::{SecurityCategory, SecurityValidator, block, raw};
use crate::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;

static BRACE_EXPANSION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{[^{}\n]+,[^{}\n]+\}|\{\d+\.\.\d+\}").expect("valid brace expansion regex")
});

pub(super) struct BraceExpansionValidator;

impl SecurityValidator for BraceExpansionValidator {
    fn name(&self) -> &str {
        "brace_expansion"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        if BRACE_EXPANSION_RE.is_match(raw(cmd)) {
            return vec![block(
                SecurityCategory::Obfuscation,
                "Brace expansion can hide fan-out or resource-intensive command shapes",
                Some("Spell out the concrete arguments explicitly"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "brace_expansion_tests.rs"]
mod brace_expansion_tests;
