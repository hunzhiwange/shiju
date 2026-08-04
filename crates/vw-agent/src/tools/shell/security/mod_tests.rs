use crate::security::AutonomyLevel;
use crate::tools::shell::security::{
    SecurityCategory, SecurityFinding, SecurityPipeline, SecurityReport, Severity,
};
use vw_config_types::security::BashValidatorChecksConfig;

#[test]
fn block_message_returns_none_without_blocking_findings() {
    let report = SecurityReport {
        blocked: false,
        findings: vec![SecurityFinding {
            severity: Severity::Warn,
            category: SecurityCategory::Obfuscation,
            message: "warn".into(),
            suggestion: None,
        }],
    };

    assert_eq!(report.block_message(), None);
}

#[test]
fn block_message_joins_only_blocking_findings() {
    let report = SecurityReport {
        blocked: true,
        findings: vec![
            SecurityFinding {
                severity: Severity::Block,
                category: SecurityCategory::Injection,
                message: "first".into(),
                suggestion: None,
            },
            SecurityFinding {
                severity: Severity::Warn,
                category: SecurityCategory::Obfuscation,
                message: "warn".into(),
                suggestion: None,
            },
            SecurityFinding {
                severity: Severity::Block,
                category: SecurityCategory::UnsafePattern,
                message: "second".into(),
                suggestion: None,
            },
        ],
    };

    assert_eq!(report.block_message(), Some("first; second".into()));
}

#[test]
fn quoted_single_line_heredoc_short_circuits_to_default_report() {
    let report = SecurityPipeline::new(true).validate_command("cat <<'EOF'");

    assert_eq!(report, SecurityReport::default());
}

#[test]
fn multiline_quoted_heredoc_drops_only_heredoc_specific_finding() {
    let report = SecurityPipeline::new(true).validate_command("cat <<'EOF'\n$(whoami)\nEOF");

    assert!(report.blocked);
    assert!(report.findings.iter().all(|finding| {
        finding.message
            != "Unquoted heredoc marker allows variable expansion inside the heredoc body"
    }));
}

#[test]
fn for_autonomy_is_strict_when_not_full_or_explicitly_disallowed() {
    let supervised = SecurityPipeline::for_autonomy(AutonomyLevel::Supervised, true);
    let full_relaxed = SecurityPipeline::for_autonomy(AutonomyLevel::Full, true);
    let full_strict = SecurityPipeline::for_autonomy(AutonomyLevel::Full, false);

    assert!(supervised.validate_command("echo $(whoami)").blocked);
    assert!(!full_relaxed.validate_command("echo $(whoami)").blocked);
    assert!(full_strict.validate_command("echo $(whoami)").blocked);
}

#[test]
fn configured_pipeline_removes_only_the_disabled_validator() {
    let mut config = BashValidatorChecksConfig::default();
    config.jq = false;
    let pipeline = SecurityPipeline::new_with_config(true, &config);

    assert!(!pipeline.validate_command("jq 'def x: .; x'").blocked);
    assert!(pipeline.validate_command("echo {1..3}").blocked);
}
