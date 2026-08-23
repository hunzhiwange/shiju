//! 验证花括号展开校验器对危险与普通输入的分类。
//! 用例同时固定校验器名称和阻断严重级别，供配置与报告层稳定引用。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    BraceExpansionValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(BraceExpansionValidator.name(), "brace_expansion");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("echo {1..3}");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo one two").is_empty());
}
