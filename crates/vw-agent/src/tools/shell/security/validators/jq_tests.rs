//! 验证 jq 校验器对危险过滤器用法与常规查询的分类。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    JqValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(JqValidator.name(), "jq");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("jq 'def x: .; x'");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("jq '.name'").is_empty());
}
