//! 验证 Git 提交校验器对高风险提交形式与普通操作的区分。
//! 规则只针对可能触发提交的危险形态，不能阻断安全的 Git 读取操作。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    GitCommitValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(GitCommitValidator.name(), "git_commit");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("git commit -m \"$(whoami)\"");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("git status").is_empty());
}
