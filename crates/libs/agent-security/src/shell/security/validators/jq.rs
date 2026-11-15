//! jq 程序安全 validator。
//!
//! 对 jq 命令中的 import/def/modulemeta 做阻断，避免过滤表达式加载模块或定义复杂行为，
//! 保持自动执行路径中的 jq 过滤器可审阅。

use super::{SecurityCategory, SecurityValidator, block, lower_tokens, raw};
use crate::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;

static DANGEROUS_JQ_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(import|def|modulemeta)\b").expect("valid jq validator regex")
});

pub(super) struct JqValidator;

impl SecurityValidator for JqValidator {
    fn name(&self) -> &str {
        "jq"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let tokens = lower_tokens(cmd);
        if tokens.first().map(String::as_str) != Some("jq") {
            return Vec::new();
        }
        if DANGEROUS_JQ_RE.is_match(raw(cmd)) {
            return vec![block(
                SecurityCategory::UnsafePattern,
                "jq program contains import/def/modulemeta, which can load or define unsafe behavior",
                Some("Use a simple jq filter expression without imports or definitions"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "jq_tests.rs"]
mod jq_tests;
