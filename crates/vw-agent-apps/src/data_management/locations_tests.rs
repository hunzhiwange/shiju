use super::{locations::*, safety_snapshot::SafetySnapshotScope};
use std::{fs, os::unix::fs::symlink};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn scope() -> SafetySnapshotScope {
    SafetySnapshotScope {
        project: ProjectScope::new(
            TenantScope::new("fixture", "company").unwrap(),
            ProjectId::from("project"),
        )
        .unwrap(),
        principal_id: "user".into(),
        host_id: "host".into(),
    }
}

#[test]
fn cc_data_30_locations_measures_whitelisted_categories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir_all(root.join("skill-backups/generation/nested")).unwrap();
    fs::write(root.join("skill-backups/generation/nested/a"), b"12345").unwrap();
    fs::write(root.join("skill-backups/.DS_Store"), b"ignored").unwrap();
    fs::write(root.join("env-backup-fixture.json"), b"123").unwrap();
    fs::write(root.join("unrelated.json"), b"ignored").unwrap();
    let scope = scope();
    let roots = BackupLocationRoots {
        scope: &scope,
        local_backups: &root.join("missing"),
        app: &root,
        device: &root,
        environment: &root,
    };
    let locations = list_locations(&roots, &scope).unwrap();
    assert_eq!(locations.len(), 14);
    let skills = locations.iter().find(|l| l.id == "skills").unwrap();
    assert_eq!((skills.item_count, skills.size_bytes), (1, 5));
    assert!(skills.last_modified.is_some());
    let env = locations.iter().find(|l| l.id == "envVars").unwrap();
    assert_eq!((env.item_count, env.size_bytes), (1, 3));
    assert_eq!(locations[0].size_bytes, 0);
    assert!(!locations[0].deletable);
    let mut foreign = scope.clone();
    foreign.host_id = "other".into();
    assert!(list_locations(&roots, &foreign).is_err());
}

#[test]
fn cc_data_30_locations_does_not_follow_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    fs::create_dir(root.join("skill-backups")).unwrap();
    fs::write(root.join("outside"), b"secret").unwrap();
    symlink(root.join("outside"), root.join("skill-backups/link")).unwrap();
    let scope = scope();
    let roots = BackupLocationRoots {
        scope: &scope,
        local_backups: &root.join("missing"),
        app: &root,
        device: &root,
        environment: &root,
    };
    let locations = list_locations(&roots, &scope).unwrap();
    let skills = locations.iter().find(|l| l.id == "skills").unwrap();
    assert_eq!((skills.item_count, skills.size_bytes), (0, 0));
    fs::remove_dir(root.join("skill-backups")).unwrap_err();
    fs::remove_file(root.join("skill-backups/link")).unwrap();
    fs::remove_dir(root.join("skill-backups")).unwrap();
    symlink(&root, root.join("skill-backups")).unwrap();
    assert!(list_locations(&roots, &scope).is_err());
}
