//! 注入风险筛选辅助函数的行为测试。

use super::injection::{has_blocking_injection, injection_findings};
use super::{SecurityCategory, SecurityFinding, SecurityPipeline, SecurityReport, Severity};

#[test]
fn injection_helpers_filter_injection_findings() {
    let report = SecurityPipeline::new(true).validate_command("echo $(uname -a)");
    assert!(has_blocking_injection(&report));
    assert!(!injection_findings(&report).is_empty());
}

#[test]
fn injection_helpers_include_non_blocking_privilege_escalation_and_ignore_unrelated_findings() {
    let report = SecurityReport {
        blocked: true,
        findings: vec![
            SecurityFinding {
                severity: Severity::Warn,
                category: SecurityCategory::PrivilegeEscalation,
                message: "privilege boundary".into(),
                suggestion: None,
            },
            SecurityFinding {
                severity: Severity::Block,
                category: SecurityCategory::UnsafePattern,
                message: "unrelated block".into(),
                suggestion: None,
            },
        ],
    };

    let findings = injection_findings(&report);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].category, SecurityCategory::PrivilegeEscalation);
    assert!(!has_blocking_injection(&report));
}
