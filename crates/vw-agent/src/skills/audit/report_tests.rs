//! 验证技能审计报告的分级、汇总与稳定呈现行为。
//! 报告顺序与发现顺序一致，干净结果必须产生空摘要而非占位告警。

use super::*;

#[test]
fn clean_report_has_empty_summary() {
    let report = SkillAuditReport::default();
    assert!(report.is_clean());
    assert_eq!(report.summary(), "");
}

#[test]
fn summary_joins_findings_in_order() {
    let report =
        SkillAuditReport { files_scanned: 2, findings: vec!["first".into(), "second".into()] };
    assert!(!report.is_clean());
    assert_eq!(report.summary(), "first; second");
}
