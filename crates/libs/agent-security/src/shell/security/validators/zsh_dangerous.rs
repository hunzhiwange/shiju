//! zsh 专有危险展开 validator。
//!
//! 阻断 `=cmd`、历史替换和下标 glob 等 zsh 专有语法，避免在不同 shell 审阅模型之间
//! 产生能力差异。

use super::{SecurityCategory, SecurityValidator, block, raw};
use crate::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;

static ZSH_DANGEROUS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(^|\s)=\w+|\^[^^\s]+\^[^^\s]*|~\[[^]]+\]").expect("valid zsh regex")
});

pub(super) struct ZshDangerousValidator;

impl SecurityValidator for ZshDangerousValidator {
    fn name(&self) -> &str {
        "zsh_dangerous"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        if ZSH_DANGEROUS_RE.is_match(raw(cmd)) {
            return vec![block(
                SecurityCategory::UnsafePattern,
                "Command uses zsh-only expansion or history substitution that bypasses review",
                Some("Use portable, literal shell syntax"),
            )];
        }
        Vec::new()
    }
}
