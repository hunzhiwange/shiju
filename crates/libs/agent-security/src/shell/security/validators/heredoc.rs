//! heredoc 安全 validator。
//!
//! 未引用 heredoc 标记会在正文中展开变量和命令替换；在代理执行场景中这可能泄露环境
//! 或改变实际写入内容，因此默认阻断。

use super::{SecurityCategory, SecurityValidator, block, raw};
use crate::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;

static HEREDOC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<<-?\s*([A-Za-z_][A-Za-z0-9_]*)").expect("valid heredoc regex"));
static QUOTED_HEREDOC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<<-?\s*(?:'[^'\n]+'|"[^"\n]+")"#).expect("valid quoted heredoc regex")
});

pub(super) struct HeredocValidator;

impl SecurityValidator for HeredocValidator {
    fn name(&self) -> &str {
        "heredoc"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let command = raw(cmd);
        if HEREDOC_RE.is_match(command) && !QUOTED_HEREDOC_RE.is_match(command) {
            return vec![block(
                SecurityCategory::DataExfiltration,
                "Unquoted heredoc marker allows variable expansion inside the heredoc body",
                Some("Use <<'EOF' or <<\"EOF\" for literal heredoc content"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "heredoc_tests.rs"]
mod heredoc_tests;
