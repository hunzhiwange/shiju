use std::collections::BTreeMap;
use serde_json::json;
use vw_api_types::{id::{ProjectId, ProjectScope, TenantScope}, project_profiles::*};
use super::{profiles::*, preview::{Decision, Difference, PreviewError}, source::*};

fn fixture() -> (SourceScope, SourceSnapshot, ReferenceMap) {
    let scope = SourceScope::new(ProjectScope::new(
        TenantScope::new("fixture-platform", "fixture-company").unwrap(),
        ProjectId::from("fixture-project"),
    ).unwrap(), "fixture-host".into(), "fixture-user".into()).unwrap();
    let payload = json!({"providers":{"claude":"p"},"mcp":{"claude":["m"]},
        "skills":{"claude":["s"]},"prompts":{"claude":"r","codex":""}});
    let source = SourceSnapshot {
        scope: scope.clone(), summary: SourceSummary {
            format: SourceFormat::Sqlite, version: 24, main_sha256: [13;32],
            wal_sha256: None, counts: BTreeMap::new(),
        }, records: vec![SourceRecord { category: SourceCategory::Profiles,
            origin: "profiles".into(), fields: json!({"id":"fixture-profile", "name":"Anonymous",
                "payload":payload.to_string(), "sort_order":2, "created_at":1, "updated_at":2}) }],
    };
    let maps = [("provider","p"),("mcp","m"),("skill","s"),("prompt","r")].into_iter()
        .map(|(category,id)| ((category.into(),"claude".into(),id.into()),format!("target-{id}"))).collect();
    (scope, source, maps)
}

#[test]
fn cc_data_13_profiles_remap_selected_references() {
    let (scope, source, maps) = fixture();
    let preview = prepare(&scope, &source, &[], &maps, |_,_,id| id.starts_with("target-")).unwrap();
    let rows = preview.resolve(&scope, &source, &[], &[], |_,_,_| true).unwrap();
    let payload = &rows[0].payload;
    assert_eq!(payload.providers.claude.as_deref(), Some("target-p"));
    assert_eq!(payload.mcp.claude.as_ref().unwrap(), &["target-m"]);
    assert_eq!(payload.skills.claude.as_ref().unwrap()[0].installation_id, "target-s");
    assert_eq!(payload.prompts.claude.as_deref(), Some("target-r"));
    assert_eq!(payload.prompts.codex.as_deref(), Some(""));
    assert_eq!(payload.providers.codex, None);
    let repeated = prepare(&scope, &source, &rows, &maps, |_,_,_| true).unwrap();
    assert_eq!(repeated.entries()[0].difference, Difference::Identical);
    assert!(repeated.resolve(&scope, &source, &rows, &[], |_,_,_| true).unwrap().is_empty());
    let mut changed = rows.clone(); changed[0].name = "Changed".into();
    let conflict = prepare(&scope, &source, &changed, &maps, |_,_,_| true).unwrap();
    let id = conflict.entries()[0].id;
    assert!(conflict.resolve(&scope, &source, &changed, &[(id,Decision::Skip)], |_,_,_| true).unwrap().is_empty());
    assert_eq!(conflict.resolve(&scope, &source, &changed, &[(id,Decision::Replace)], |_,_,_| true).unwrap()[0].name, "Anonymous");
}

#[test]
fn cc_data_13_profiles_reject_dangling_reference() {
    let (scope, source, mut maps) = fixture();
    maps.remove(&("skill".into(),"claude".into(),"s".into()));
    let preview = prepare(&scope, &source, &[], &maps, |_,_,_| true).unwrap();
    assert_eq!(preview.entries()[0].difference, Difference::InvalidReference);
    let id = preview.entries()[0].id;
    assert_eq!(preview.resolve(&scope, &source, &[], &[(id,Decision::Replace)], |_,_,_| true), Err(PreviewError::InvalidDecision));
    assert!(preview.resolve(&scope, &source, &[], &[(id,Decision::Skip)], |_,_,_| true).unwrap().is_empty());
    let (_, _, maps) = fixture();
    let preview = prepare(&scope, &source, &[], &maps, |_,_,_| true).unwrap();
    assert_eq!(preview.resolve(&scope, &source, &[], &[], |category,_,_| category != ProjectProfileCategory::Skill), Err(PreviewError::InvalidCandidate));
}
