use std::collections::BTreeMap;
use serde_json::json;
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};
use super::{preview::*, source::*};

fn fixture() -> (SourceScope, SourceSnapshot) {
    let project = ProjectScope::new(
        TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        ProjectId::from("fixture-project"),
    ).unwrap();
    let scope = SourceScope::new(project, "fixture-host".into(), "fixture-user".into()).unwrap();
    let source = SourceSnapshot {
        scope: scope.clone(),
        summary: SourceSummary {
            format: SourceFormat::Json, version: 2, main_sha256: [7; 32],
            wal_sha256: None, counts: BTreeMap::new(),
        },
        records: vec![SourceRecord {
            category: SourceCategory::Rules, origin: "fixture".into(),
            fields: json!({"name":"fixture", "content":"anonymous"}),
        }],
    };
    (scope, source)
}

#[test]
fn cc_data_07_preview_reports_selected_differences() {
    let (scope, source) = fixture();
    let preview = build_preview(&scope, &source, &[SourceCategory::Rules], "revision-1".into(), |category, records| {
        assert_eq!(category, SourceCategory::Rules);
        assert_eq!(records.len(), 1);
        Ok(vec![
            PreviewCandidate { incoming: json!({"name":"new", "token":"fixture-secret"}), existing: None, references_valid: true },
            PreviewCandidate { incoming: json!({"name":"same", "token":"one"}), existing: Some(json!({"name":"same", "token":"two"})), references_valid: true },
            PreviewCandidate { incoming: json!({"name":"changed"}), existing: Some(json!({"name":"old"})), references_valid: true },
            PreviewCandidate { incoming: json!({"name":"missing-ref"}), existing: None, references_valid: false },
        ])
    }).unwrap();
    assert_eq!(preview.entries().iter().map(|entry| entry.difference).collect::<Vec<_>>(),
        vec![Difference::Added, Difference::Identical, Difference::Different, Difference::InvalidReference]);
    assert_eq!(preview.entries()[0].excluded_fields, 1);
    let display = format!("{:?}", preview.entries());
    assert!(!display.contains("fixture-secret"));
    assert!(!display.contains("missing-ref"));
    assert_eq!(source.records(&scope).unwrap()[0].fields["content"], "anonymous");
}

#[test]
fn cc_data_07_preview_blocks_unresolved_conflict() {
    let (scope, mut source) = fixture();
    let preview = build_preview(&scope, &source, &[SourceCategory::Rules], "revision-1".into(), |_, _| {
        Ok(vec![PreviewCandidate {
            incoming: json!({"content":"new"}), existing: Some(json!({"content":"old"})), references_valid: true,
        }])
    }).unwrap();
    assert_eq!(preview.resolve(&scope, &source, "revision-1", &[]), Err(PreviewError::UnresolvedConflict));
    let decision = (preview.entries()[0].id, Decision::Replace);
    assert_eq!(preview.resolve(&scope, &source, "revision-1", &[decision]).unwrap()[&decision.0], Decision::Replace);
    assert_eq!(preview.resolve(&scope, &source, "revision-2", &[decision]), Err(PreviewError::Stale));
    assert_eq!(preview.resolve(&scope, &source, "revision-1", &[decision, decision]), Err(PreviewError::InvalidDecision));
    source.summary.wal_sha256 = Some([9; 32]);
    assert_eq!(preview.resolve(&scope, &source, "revision-1", &[decision]), Err(PreviewError::Stale));
}
