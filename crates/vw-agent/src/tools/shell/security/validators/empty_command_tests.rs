//! 验证空命令校验器的稳定名称与空白输入判定。
//! 非空普通命令必须不产生发现项，避免入口校验与后续规则职责重叠。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    EmptyCommandValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(EmptyCommandValidator.name(), "empty_command");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("   ");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo hi").is_empty());
}
