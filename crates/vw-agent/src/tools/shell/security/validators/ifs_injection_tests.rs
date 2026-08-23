//! 验证 IFS 注入校验器阻断分词语义篡改并放行普通输入。
//! 名称与阻断严重级别保持稳定，便于配置开关和审计输出精确定位。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    IfsInjectionValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(IfsInjectionValidator.name(), "ifs_injection");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("IFS=/; echo test");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo test").is_empty());
}
