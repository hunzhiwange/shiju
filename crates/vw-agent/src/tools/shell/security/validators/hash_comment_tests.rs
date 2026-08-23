//! 验证井号注释校验器识别可能隐藏后续命令语义的输入。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    HashCommentValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(HashCommentValidator.name(), "hash_comment");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("echo#hidden");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo \"#literal\"").is_empty());
}
