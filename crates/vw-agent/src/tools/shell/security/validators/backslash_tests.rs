//! 验证反斜杠校验器的稳定名称及风险输入识别边界。
//! 代表性危险与普通命令共同约束规则，避免校验器退化为全量拒绝。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    BackslashValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(BackslashValidator.name(), "backslash");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("echo hello\\\\ world");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo hello").is_empty());
}
