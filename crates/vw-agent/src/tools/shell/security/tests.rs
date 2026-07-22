use super::{
    SecurityCategory, SecurityFinding, SecurityPipeline, SecurityReport, SecurityValidator,
    Severity,
};
use crate::security::AutonomyLevel;
use crate::tools::shell::ast::ParsedCommand;

const HEREDOC_EXPANSION_MESSAGE: &str =
    "Unquoted heredoc marker allows variable expansion inside the heredoc body";

struct StaticValidator {
    findings: Vec<SecurityFinding>,
}

impl SecurityValidator for StaticValidator {
    fn name(&self) -> &str {
        "static"
    }

    fn validate(&self, _cmd: &ParsedCommand) -> Vec<SecurityFinding> {
        self.findings.clone()
    }
}

fn finding(severity: Severity, category: SecurityCategory, message: &str) -> SecurityFinding {
    SecurityFinding { severity, category, message: message.into(), suggestion: None }
}

fn pipeline_with(findings: Vec<Vec<SecurityFinding>>) -> SecurityPipeline {
    SecurityPipeline {
        validators: findings
            .into_iter()
            .map(|findings| Box::new(StaticValidator { findings }) as Box<dyn SecurityValidator>)
            .collect(),
    }
}

#[test]
fn block_message_uses_finding_severity_instead_of_cached_blocked_flag() {
    let report = SecurityReport {
        blocked: true,
        findings: vec![finding(Severity::Info, SecurityCategory::UnsafePattern, "informational")],
    };

    assert_eq!(report.block_message(), None);
}

#[test]
fn plain_command_with_no_validators_returns_default_report() {
    let report = pipeline_with(Vec::new()).validate_command("printf safe");

    assert_eq!(report, SecurityReport::default());
}

#[test]
fn validate_keeps_non_blocking_findings_without_blocking_report() {
    let expected = vec![
        finding(Severity::Warn, SecurityCategory::Obfuscation, "review warning"),
        finding(Severity::Info, SecurityCategory::DataExfiltration, "audit detail"),
    ];
    let report = pipeline_with(vec![expected.clone()]).validate_command("printf safe");

    assert!(!report.blocked);
    assert_eq!(report.findings, expected);
}

#[test]
fn validate_aggregates_in_validator_order_and_sets_blocked() {
    let warning = finding(Severity::Warn, SecurityCategory::PrivilegeEscalation, "first validator");
    let blocking =
        finding(Severity::Block, SecurityCategory::DestructiveOperation, "second validator");
    let report = pipeline_with(vec![vec![warning.clone()], vec![blocking.clone()]])
        .validate_command("printf safe");

    assert!(report.blocked);
    assert_eq!(report.findings, vec![warning, blocking]);
}

#[test]
fn double_quoted_single_line_heredoc_short_circuits_before_validation() {
    let blocking =
        finding(Severity::Block, SecurityCategory::Injection, "validator should not run");
    let report = pipeline_with(vec![vec![blocking]]).validate_command("cat <<\"EOF\"");

    assert_eq!(report, SecurityReport::default());
}

#[test]
fn double_quoted_multiline_heredoc_recomputes_blocked_after_filtering() {
    let filtered = finding(Severity::Block, SecurityCategory::Injection, HEREDOC_EXPANSION_MESSAGE);
    let retained = finding(Severity::Warn, SecurityCategory::UnsafePattern, "retained warning");
    let report = pipeline_with(vec![vec![filtered, retained.clone()]])
        .validate_command("cat <<\"EOF\"\nbody\nEOF");

    assert!(!report.blocked);
    assert_eq!(report.findings, vec![retained]);
}

#[test]
fn read_only_autonomy_remains_strict_when_unsafe_patterns_are_allowed() {
    let report = SecurityPipeline::for_autonomy(AutonomyLevel::ReadOnly, true)
        .validate_command("echo $(whoami)");

    assert!(report.blocked);
}
