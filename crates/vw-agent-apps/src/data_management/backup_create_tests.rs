use super::{backup_create::*, backup_create_files as files, safety_snapshot::*};
use crate::usage::source_settings::{
    UsageSourceSettings, initialize_source_settings, load_source_settings, save_source_settings,
};
use rusqlite::Connection;
use std::{fs, os::unix::fs::PermissionsExt};
use vw_api_types::id::{ProjectId, ProjectScope, TenantScope};

fn scope() -> SafetySnapshotScope {
    SafetySnapshotScope {
        project: ProjectScope::new(
            TenantScope::new("fixture-platform", "fixture-company").unwrap(),
            ProjectId::from("fixture-project"),
        )
        .unwrap(),
        principal_id: "fixture-user".into(),
        host_id: "fixture-host".into(),
    }
}
fn target() -> SnapshotTarget {
    SnapshotTarget {
        category: SnapshotCategory::Preference,
        object_id: "usage-source/claude".into(),
    }
}
fn root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    root
}

#[test]
fn cc_data_22_backup_publishes_readable_snapshot() {
    let root = root();
    let path = root.path().canonicalize().unwrap();
    let scope = scope();
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_source_settings(&connection).unwrap();
    let settings =
        UsageSourceSettings { scan_enabled: false, show_usage: true, refresh_seconds: Some(90) };
    save_source_settings(
        &mut connection,
        &scope.project,
        &scope.host_id,
        "claude",
        0,
        settings.clone(),
    )
    .unwrap();
    // Capture via the real owner while holding one SQLite read transaction.
    let transaction = connection.transaction().unwrap();
    let receipt = create_local_backup(&path, &scope, &[target()], "Anonymous backup", || {
        let state =
            load_source_settings(&transaction, &scope.project, &scope.host_id, "claude").unwrap();
        Ok(vec![OwnerPreimage {
            target: target(),
            state: Some(PresentPreimage {
                revision: state.settings_revision.to_string(),
                bytes: serde_json::to_vec(&state.settings).unwrap(),
            }),
        }])
    })
    .unwrap()
    .unwrap();
    transaction.commit().unwrap();
    save_source_settings(
        &mut connection,
        &scope.project,
        &scope.host_id,
        "claude",
        1,
        UsageSourceSettings::default(),
    )
    .unwrap();
    let backup = open_local_backup(&path, &receipt, &scope).unwrap();
    assert_eq!(backup.name(), "Anonymous backup");
    assert_eq!(backup.scope(), &scope);
    let image = backup.preimages()[0].state.as_ref().unwrap();
    assert_eq!(image.revision, "1");
    assert_eq!(serde_json::from_slice::<UsageSourceSettings>(&image.bytes).unwrap(), settings);
    assert_eq!(fs::read_dir(&path).unwrap().count(), 1);
    let archive = path.join(format!("{}.local-backup", receipt.id()));
    assert_eq!(fs::metadata(&archive).unwrap().permissions().mode() & 0o777, 0o600);
    let mut foreign = scope.clone();
    foreign.host_id = "another-host".into();
    assert!(matches!(
        open_local_backup(&path, &receipt, &foreign),
        Err(SafetySnapshotError::ScopeMismatch)
    ));
    foreign = scope.clone();
    foreign.principal_id = "another-user".into();
    assert!(matches!(
        open_local_backup(&path, &receipt, &foreign),
        Err(SafetySnapshotError::ScopeMismatch)
    ));
    foreign = scope.clone();
    foreign.project = ProjectScope::new(
        TenantScope::new("fixture-platform", "another-company").unwrap(),
        ProjectId::from("fixture-project"),
    )
    .unwrap();
    assert!(matches!(
        open_local_backup(&path, &receipt, &foreign),
        Err(SafetySnapshotError::ScopeMismatch)
    ));
    fs::write(&archive, b"damaged").unwrap();
    assert!(matches!(
        open_local_backup(&path, &receipt, &scope),
        Err(SafetySnapshotError::Corrupt)
    ));
}

#[test]
fn cc_data_22_backup_collision_never_overwrites() {
    let root = root();
    let path = root.path().canonicalize().unwrap();
    let directory = files::open_root(&path).unwrap();
    let threads: Vec<_> = (0..4)
        .map(|index| {
            let dir = directory.try_clone().unwrap();
            std::thread::spawn(move || {
                let bytes = format!("anonymous-{index}").into_bytes();
                let id = files::publish(&dir, "same-id", &bytes, |reread| {
                    assert_eq!(reread, bytes);
                    Ok(())
                })
                .unwrap();
                (id, bytes)
            })
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|thread| thread.join().unwrap()).collect();
    let ids: std::collections::HashSet<_> = results.iter().map(|(id, _)| id).collect();
    assert_eq!(ids.len(), 4);
    for (id, bytes) in results {
        assert_eq!(fs::read(path.join(format!("{id}.local-backup"))).unwrap(), bytes);
    }
    assert_eq!(fs::read_dir(&path).unwrap().count(), 4);
}

#[test]
fn cc_data_22_backup_failure_and_absence_leave_no_archive() {
    let root = root();
    let path = root.path().canonicalize().unwrap();
    let scope = scope();
    let absent = create_local_backup(&path, &scope, &[target()], "Missing", || {
        Ok(vec![OwnerPreimage { target: target(), state: None }])
    })
    .unwrap();
    assert!(absent.is_none());
    assert!(matches!(
        create_local_backup(&path, &scope, &[target()], "Incomplete", || Ok(vec![])),
        Err(SafetySnapshotError::IncompleteCapture)
    ));
    assert!(matches!(
        create_local_backup(&path, &scope, &[target()], "Unavailable", || Err(
            SafetySnapshotError::CaptureFailed
        )),
        Err(SafetySnapshotError::CaptureFailed)
    ));
    let directory = files::open_root(&path).unwrap();
    assert!(matches!(
        files::publish(&directory, "failure", b"fixture", |_| Err(SafetySnapshotError::Corrupt)),
        Err(SafetySnapshotError::Corrupt)
    ));
    // Inject only the OS publication boundary: no writable destination after capture.
    // Open directory descriptors remain valid after unlink; rename must fail.
    let removed = root.path().join("removed");
    fs::create_dir(&removed).unwrap();
    fs::set_permissions(&removed, fs::Permissions::from_mode(0o700)).unwrap();
    let removed_dir = files::open_root(&removed.canonicalize().unwrap()).unwrap();
    fs::remove_dir(&removed).unwrap();
    assert!(files::publish(&removed_dir, "failure", b"fixture", |_| Ok(())).is_err());
    assert_eq!(fs::read_dir(&path).unwrap().count(), 0);
}
