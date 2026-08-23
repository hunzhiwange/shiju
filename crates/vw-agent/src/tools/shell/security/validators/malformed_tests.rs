//! 验证畸形 shell 输入校验器的稳定名称与拒绝边界。
//! 完整普通命令不得被标记，确保解析失败和合法输入之间界线明确。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    MalformedValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(MalformedValidator.name(), "malformed");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("echo \"unterminated");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo ok").is_empty());
}
