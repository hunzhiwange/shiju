//! 验证注释与引号失同步校验器的风险识别边界。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    CommentQuoteDesyncValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(CommentQuoteDesyncValidator.name(), "comment_quote_desync");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("echo \"unterminated # note");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo \"closed\" # note").is_empty());
}
