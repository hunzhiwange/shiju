//! 验证控制字符校验器会阻断异常字符并允许普通命令。
//! 校验结果还固定稳定名称和严重级别，确保安全流水线可预测聚合。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    ControlCharsValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(ControlCharsValidator.name(), "control_chars");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("echo hi\u{7f}");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo hi").is_empty());
}
