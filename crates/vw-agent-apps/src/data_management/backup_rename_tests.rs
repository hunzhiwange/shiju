use super::{backup_create::*, backup_rename::*, safety_snapshot::*};
use crate::usage::source_settings::{initialize_source_settings, load_source_settings};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn scope(company: &str) -> SafetySnapshotScope {
    SafetySnapshotScope {
        project: ProjectScope::new(
            TenantScope::new("fixture-platform", company).unwrap(),
            ProjectId::from("fixture-project"),
        )
        .unwrap(),
        principal_id: "fixture-user".into(),
        host_id: "fixture-host".into(),
    }
}
fn create(root: &Path, scope: &SafetySnapshotScope) -> LocalBackupReceipt {
    let mut connection = rusqlite::Connection::open_in_memory().unwrap();
    initialize_source_settings(&connection).unwrap();
    let transaction = connection.transaction().unwrap();
    let target = SnapshotTarget {
        category: SnapshotCategory::Preference,
        object_id: "usage-source/claude".into(),
    };
    let receipt = create_local_backup(root, scope, &[target.clone()], "Anonymous", || {
        let state =
            load_source_settings(&transaction, &scope.project, &scope.host_id, "claude").unwrap();
        Ok(vec![OwnerPreimage {
            target,
            state: Some(PresentPreimage {
                revision: state.settings_revision.to_string(),
                bytes: serde_json::to_vec(&state.settings).unwrap(),
            }),
        }])
    })
    .unwrap()
    .unwrap();
    transaction.commit().unwrap();
    receipt
}

fn root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    root
}
fn bytes(root: &Path, id: &str) -> Vec<u8> {
    fs::read(root.join(format!("{id}.local-backup"))).unwrap()
}

#[test]
fn cc_data_24_rename_changes_only_selected_backup() {
    let temp = root();
    let root = temp.path().canonicalize().unwrap();
    let scope = scope("fixture-company");
    let selected = create(&root, &scope);
    let other = create(&root, &scope);
    let original = bytes(&root, selected.id());
    let untouched = bytes(&root, other.id());
    let renamed = rename_local_backup(&root, &selected, &scope, "renamed").unwrap();
    assert_eq!(renamed.id(), "renamed");
    assert_eq!(renamed.digest(), selected.digest());
    assert_eq!(bytes(&root, renamed.id()), original);
    assert_eq!(bytes(&root, other.id()), untouched);
    assert!(!root.join(format!("{}.local-backup", selected.id())).exists());
    assert!(open_local_backup(&root, &selected, &scope).is_err());
    assert!(open_local_backup(&root, &renamed, &scope).is_ok());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
}

#[test]
fn cc_data_24_rename_collision_preserves_both_files() {
    let temp = root();
    let root = temp.path().canonicalize().unwrap();
    let scope = scope("fixture-company");
    let first = create(&root, &scope);
    let second = create(&root, &scope);
    let original = bytes(&root, first.id());
    let other = bytes(&root, second.id());
    assert!(matches!(
        rename_local_backup(&root, &first, &scope, second.id()),
        Err(SafetySnapshotError::Conflict)
    ));
    for invalid in ["", "../escape", "a/b", "a\\b", "UPPER", "a\0b"] {
        assert!(rename_local_backup(&root, &first, &scope, invalid).is_err());
    }
    assert!(rename_local_backup(&root, &first, &scope, first.id()).is_err());
    let mut foreign = scope.clone();
    foreign.host_id = "foreign-host".into();
    assert!(matches!(
        rename_local_backup(&root, &first, &foreign, "foreign"),
        Err(SafetySnapshotError::ScopeMismatch)
    ));
    let link = root.join("linked.local-backup");
    symlink(root.join(format!("{}.local-backup", first.id())), &link).unwrap();
    assert!(rename_local_backup(&root, &first, &scope, "linked").is_err());
    fs::remove_file(&link).unwrap();
    assert_eq!(bytes(&root, first.id()), original);
    assert_eq!(bytes(&root, second.id()), other);
    let source_path = root.join(format!("{}.local-backup", first.id()));
    fs::rename(&source_path, &link).unwrap();
    symlink(&link, &source_path).unwrap();
    assert!(rename_local_backup(&root, &first, &scope, "unsafe").is_err());
    fs::remove_file(&source_path).unwrap();
    fs::rename(&link, &source_path).unwrap();
    // A case-equivalent preexisting name must survive on both case-sensitive
    // and case-insensitive macOS volumes.
    fs::write(root.join("CASE.local-backup"), b"occupied").unwrap();
    let result = rename_local_backup(&root, &first, &scope, "case");
    if root.join("case.local-backup").exists() && result.is_err() {
        assert_eq!(bytes(&root, first.id()), original);
    } else {
        assert!(result.is_ok());
        assert_eq!(bytes(&root, "case"), original);
    }
    assert_eq!(fs::read(root.join("CASE.local-backup")).unwrap(), b"occupied");
    assert_eq!(bytes(&root, second.id()), other);
}

#[test]
fn cc_data_24_concurrent_renames_have_one_winner() {
    let temp = root();
    let root = temp.path().canonicalize().unwrap();
    let scope = scope("fixture-company");
    let receipts: Vec<_> = (0..4).map(|_| create(&root, &scope)).collect();
    let originals: Vec<_> = receipts.iter().map(|r| bytes(&root, r.id())).collect();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let threads: Vec<_> = receipts
        .iter()
        .cloned()
        .map(|receipt| {
            let root = root.clone();
            let scope = scope.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                rename_local_backup(&root, &receipt, &scope, "shared")
            })
        })
        .collect();
    let mut winners = 0;
    for (index, thread) in threads.into_iter().enumerate() {
        match thread.join().unwrap() {
            Ok(receipt) => {
                winners += 1;
                assert_eq!(bytes(&root, receipt.id()), originals[index]);
                assert!(open_local_backup(&root, &receipt, &scope).is_ok());
            }
            Err(SafetySnapshotError::Conflict) => {
                assert_eq!(bytes(&root, receipts[index].id()), originals[index])
            }
            Err(error) => panic!("unexpected rename failure: {error}"),
        }
    }
    assert_eq!(winners, 1);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 4);
}
