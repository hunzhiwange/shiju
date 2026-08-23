//! 验证危险环境变量与指纹变量分别产生阻断或警告发现项。

use super::*;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator, Severity};

fn findings(command: &str) -> Vec<crate::tools::shell::security::SecurityFinding> {
    DangerousVarsValidator.validate(&parse_command(command))
}

#[test]
fn name_is_stable() {
    assert_eq!(DangerousVarsValidator.name(), "dangerous_vars");
}

#[test]
fn blocks_representative_risky_input() {
    let findings = findings("echo $BASH_EXECUTION_STRING");
    assert!(findings.iter().any(|finding| finding.severity == Severity::Block));
}

#[test]
fn allows_representative_plain_input() {
    assert!(findings("echo $HOME").is_empty());
}

#[test]
fn fingerprint_variable_warns_without_blocking() {
    let findings = findings("echo $SHLVL");

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, Severity::Warn);
    assert_eq!(findings[0].category, SecurityCategory::UnsafePattern);
}

#[test]
fn dangerous_and_fingerprint_variables_report_both_risks() {
    let findings = findings("IFS=/; echo $SECONDS");

    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].severity, Severity::Block);
    assert_eq!(findings[1].severity, Severity::Warn);
}
