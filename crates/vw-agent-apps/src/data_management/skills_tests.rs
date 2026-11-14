use super::{
    preview::{Decision, Difference},
    skills::*,
    source::*,
};
use serde_json::json;
use std::{collections::BTreeMap, fs};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn fixture(directory: &str) -> (SourceScope, SourceSnapshot, tempfile::TempDir) {
    let scope = SourceScope::new(
        ProjectScope::new(
            TenantScope::new("fixture-platform", "fixture-company").unwrap(),
            ProjectId::from("fixture-project"),
        )
        .unwrap(),
        "fixture-host".into(),
        "fixture-principal".into(),
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("fixture")).unwrap();
    fs::write(
        root.path().join("fixture/SKILL.toml"),
        "name = 'fixture'\ndescription = 'anonymous'\n",
    )
    .unwrap();
    let source = SourceSnapshot {
        scope: scope.clone(),
        summary: SourceSummary {
            format: SourceFormat::Sqlite,
            version: 24,
            main_sha256: [1; 32],
            wal_sha256: None,
            counts: BTreeMap::new(),
        },
        records: vec![SourceRecord {
            category: SourceCategory::Skills,
            origin: "skills".into(),
            fields: json!({
                "id": "fixture-id", "directory": directory, "name": "fixture", "description": "anonymous",
                "installed_at": 123, "updated_at": 456, "enabled_claude": 1,
                "repo_owner": "fixture-owner", "repo_name": "fixture-repository", "repo_branch": "main"
            }),
        }],
    };
    (scope, source, root)
}

#[test]
fn cc_data_11_skills_preserve_enabled_metadata() {
    let (scope, source, resources) = fixture("fixture");
    let plan = SkillsPreview::prepare(
        &scope,
        &source,
        "revision".into(),
        resources.path(),
        &BTreeMap::new(),
    )
    .unwrap();
    let selected = plan.selected(&scope, &source, "revision", &[]).unwrap();
    let record = selected[0];
    assert_eq!(record.enabled.get("claude"), Some(&true));
    assert_eq!(record.installed_at, 123);
    assert_eq!(record.updated_at, 456);
    assert_eq!(record.repository.as_ref().unwrap().branch, "main");
    let saved = BTreeMap::from([("fixture".into(), record.clone())]);
    let repeat =
        SkillsPreview::prepare(&scope, &source, "next".into(), resources.path(), &saved).unwrap();
    assert_eq!(repeat.preview().entries()[0].difference, Difference::Identical);
    assert!(repeat.selected(&scope, &source, "next", &[]).unwrap().is_empty());
    let mut changed = saved;
    changed.get_mut("fixture").unwrap().enabled.insert("claude".into(), false);
    let conflict =
        SkillsPreview::prepare(&scope, &source, "next".into(), resources.path(), &changed).unwrap();
    assert_eq!(conflict.preview().entries()[0].difference, Difference::Different);
    assert!(conflict.selected(&scope, &source, "next", &[]).is_err());
    let id = conflict.preview().entries()[0].id;
    assert!(
        conflict.selected(&scope, &source, "next", &[(id, Decision::Skip)]).unwrap().is_empty()
    );
    assert_eq!(
        conflict.selected(&scope, &source, "next", &[(id, Decision::Replace)]).unwrap().len(),
        1
    );
    // Missing authorization for the enabled application must reject before writing.
    let target = tempfile::tempdir().unwrap();
    assert!(
        SkillMigration::prepare(
            target.path(),
            &plan,
            &selected,
            &[],
            &BTreeMap::new(),
            &BTreeMap::new()
        )
        .is_err()
    );
    assert!(!target.path().join("fixture").exists());
}

#[test]
fn cc_data_11_skills_reject_archive_traversal() {
    for path in ["../fixture", "/fixture", "a/fixture", "a\\fixture"] {
        let (scope, source, resources) = fixture(path);
        assert!(
            SkillsPreview::prepare(
                &scope,
                &source,
                "revision".into(),
                resources.path(),
                &BTreeMap::new()
            )
            .is_err()
        );
    }
    let (scope, source, resources) = fixture("fixture");
    fs::write(resources.path().join("outside"), "anonymous").unwrap();
    std::os::unix::fs::symlink(
        resources.path().join("outside"),
        resources.path().join("fixture/link"),
    )
    .unwrap();
    assert!(
        SkillsPreview::prepare(
            &scope,
            &source,
            "revision".into(),
            resources.path(),
            &BTreeMap::new()
        )
        .is_err()
    );
}

#[test]
fn cc_data_11_skills_batch_publication_rolls_back() {
    let (scope, mut source, resources) = fixture("fixture");
    source.records[0].fields["enabled_claude"] = json!(false);
    let plan = SkillsPreview::prepare(
        &scope,
        &source,
        "revision".into(),
        resources.path(),
        &BTreeMap::new(),
    )
    .unwrap();
    let selected = plan.selected(&scope, &source, "revision", &[]).unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut batch = SkillMigration::prepare(
        root.path(),
        &plan,
        &selected,
        &[],
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap();
    batch.publish().unwrap();
    assert!(root.path().join("fixture/SKILL.toml").is_file());
    batch.rollback().unwrap();
    assert!(!root.path().join("fixture").exists());
}
