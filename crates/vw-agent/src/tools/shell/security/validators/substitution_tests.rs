//! 验证命令替换校验器在严格与宽松模式下的阻断和告警差异。
//! 回退解析仍需识别命令替换和输出进程替换，不能因缺少 AST 而放行。

use super::substitution::SubstitutionValidator;
use crate::tools::shell::ast::{ParsedCommand, parse_command};
use crate::tools::shell::security::{SecurityValidator, Severity};

#[test]
fn name_is_stable() {
    assert_eq!(SubstitutionValidator::new(true).name(), "substitution");
}

#[test]
fn strict_mode_blocks_substitution() {
    let findings = SubstitutionValidator::new(true).validate(&parse_command("echo $(date)"));

    assert_eq!(findings[0].severity, Severity::Block);
}

#[test]
fn relaxed_mode_warns_for_substitution() {
    let findings = SubstitutionValidator::new(false).validate(&parse_command("cat <(echo ok)"));

    assert_eq!(findings[0].severity, Severity::Warn);
}

#[test]
fn fallback_detects_command_and_output_process_substitution() {
    let command = "cat $(date) >(sink)";
    let parsed = ParsedCommand::Fallback {
        raw: command.into(),
        tokens: vec!["cat".into(), "$(date)".into(), ">(sink)".into()],
    };

    let findings = SubstitutionValidator::new(true).validate(&parsed);

    assert_eq!(findings.len(), 2);
    assert!(findings.iter().all(|finding| finding.severity == Severity::Block));
}

#[test]
fn plain_fallback_has_no_substitution_findings() {
    let parsed = ParsedCommand::Fallback {
        raw: "printf literal".into(),
        tokens: vec!["printf".into(), "literal".into()],
    };

    assert!(SubstitutionValidator::new(true).validate(&parsed).is_empty());
}
