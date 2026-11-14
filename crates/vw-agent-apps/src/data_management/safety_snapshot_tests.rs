use super::safety_snapshot::*;
use crate::usage::source_settings::{
    UsageSourceSettings, initialize_source_settings, load_source_settings, save_source_settings,
};
use rusqlite::Connection;
use std::{
    cell::{Cell, RefCell},
    fs,
    os::unix::fs::PermissionsExt,
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
fn targets() -> Vec<SnapshotTarget> {
    vec![SnapshotTarget {
        category: SnapshotCategory::Preference,
        object_id: "usage-source/claude".into(),
    }]
}
fn images(targets: &[SnapshotTarget]) -> Vec<OwnerPreimage> {
    vec![OwnerPreimage { target: targets[0].clone(), state: None }]
}
fn private_root() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700)).unwrap();
    dir
}

#[test]
fn cc_data_08_snapshot_can_restore_current_state() {
    // Real anonymous SQLite domain store. No user database or HOME is opened.
    let root = private_root();
    let path = root.path().canonicalize().unwrap();
    let scope = scope("selected-company");
    let foreign = self::scope("unrelated-company");
    let connection = RefCell::new(Connection::open_in_memory().unwrap());
    initialize_source_settings(&connection.borrow()).unwrap();
    let old =
        UsageSourceSettings { scan_enabled: false, show_usage: false, refresh_seconds: Some(90) };
    save_source_settings(
        &mut connection.borrow_mut(),
        &scope.project,
        &scope.host_id,
        "claude",
        0,
        old.clone(),
    )
    .unwrap();
    save_source_settings(
        &mut connection.borrow_mut(),
        &foreign.project,
        &foreign.host_id,
        "claude",
        0,
        old.clone(),
    )
    .unwrap();
    let targets = targets();
    let (receipt, write_result) = with_safety_snapshot(
        &path,
        &scope,
        &targets,
        || {
            let state = load_source_settings(
                &connection.borrow(),
                &scope.project,
                &scope.host_id,
                "claude",
            )
            .unwrap();
            Ok(vec![OwnerPreimage {
                target: targets[0].clone(),
                state: Some(PresentPreimage {
                    revision: state.settings_revision.to_string(),
                    bytes: serde_json::to_vec(&state.settings).unwrap(),
                }),
            }])
        },
        |receipt| {
            // The durable archive must already exist before this first domain write.
            assert!(open_safety_snapshot(&path, receipt, &scope, &targets).is_ok());
            save_source_settings(
                &mut connection.borrow_mut(),
                &scope.project,
                &scope.host_id,
                "claude",
                1,
                UsageSourceSettings::default(),
            )
        },
    )
    .unwrap();
    let written = write_result.unwrap();
    let recovered = open_safety_snapshot(&path, &receipt, &scope, &targets).unwrap();
    let image = recovered.preimages()[0].state.as_ref().unwrap();
    assert_eq!(image.revision, "1");
    let before: UsageSourceSettings = serde_json::from_slice(&image.bytes).unwrap();
    // Owner CAS restoration preserves monotonically increasing revisions.
    let restored = save_source_settings(
        &mut connection.borrow_mut(),
        &scope.project,
        &scope.host_id,
        "claude",
        written.settings_revision,
        before,
    )
    .unwrap();
    assert_eq!(restored.settings, old);
    assert_eq!(restored.settings_revision, 3);
    let untouched =
        load_source_settings(&connection.borrow(), &foreign.project, &foreign.host_id, "claude")
            .unwrap();
    assert_eq!(untouched.settings, old);
    assert_eq!(untouched.settings_revision, 1);
    let archive = path.join(receipt.id());
    assert_eq!(fs::metadata(&archive).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(
        fs::metadata(archive.join("preimages.json")).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn cc_data_08_snapshot_failure_blocks_writes() {
    let root = private_root();
    let path = root.path().canonicalize().unwrap();
    let scope = scope("selected-company");
    let targets = targets();
    let writes = Cell::new(0);
    let write = |_: &SafetySnapshotReceipt| {
        writes.set(writes.get() + 1);
        Ok::<_, ()>(())
    };
    assert_eq!(
        with_safety_snapshot(
            &path,
            &scope,
            &targets,
            || Err(SafetySnapshotError::CaptureFailed),
            write
        )
        .err(),
        Some(SafetySnapshotError::CaptureFailed)
    );
    assert_eq!(
        with_safety_snapshot(&path, &scope, &targets, || Ok(vec![]), write).err(),
        Some(SafetySnapshotError::IncompleteCapture)
    );
    assert!(
        with_safety_snapshot(
            &path.join("missing"),
            &scope,
            &targets,
            || Ok(images(&targets)),
            write
        )
        .is_err()
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        with_safety_snapshot(&path, &scope, &targets, || Ok(images(&targets)), write).err(),
        Some(SafetySnapshotError::UnsafeStorage)
    );
    assert_eq!(writes.get(), 0);
}

#[test]
fn cc_data_08_snapshot_rejects_corruption_scope_and_symlinks() {
    let root = private_root();
    let path = root.path().canonicalize().unwrap();
    let scope = scope("selected-company");
    let targets = targets();
    let receipt = create_safety_snapshot(&path, &scope, &targets, || Ok(images(&targets))).unwrap();
    assert_eq!(
        open_safety_snapshot(&path, &receipt, &self::scope("foreign"), &targets).err(),
        Some(SafetySnapshotError::ScopeMismatch)
    );
    let file = path.join(receipt.id()).join("preimages.json");
    fs::write(&file, b"corrupt").unwrap();
    assert_eq!(
        open_safety_snapshot(&path, &receipt, &scope, &targets).err(),
        Some(SafetySnapshotError::Corrupt)
    );
    fs::remove_file(&file).unwrap();
    let other = path.join("unrelated");
    fs::write(&other, b"do not read or overwrite").unwrap();
    std::os::unix::fs::symlink(&other, &file).unwrap();
    assert!(open_safety_snapshot(&path, &receipt, &scope, &targets).is_err());
    assert_eq!(fs::read(&other).unwrap(), b"do not read or overwrite");
}

#[test]
fn cc_data_08_snapshot_retains_receipt_after_write_failure() {
    let root = private_root();
    let path = root.path().canonicalize().unwrap();
    let scope = scope("selected-company");
    let targets = targets();
    let (receipt, result) = with_safety_snapshot(
        &path,
        &scope,
        &targets,
        || Ok(images(&targets)),
        |_| Err::<(), _>("commit outcome requires owner recovery"),
    )
    .unwrap();
    assert!(result.is_err());
    assert!(open_safety_snapshot(&path, &receipt, &scope, &targets).is_ok());
}

#[test]
fn cc_data_08_snapshot_readback_failure_discards_unpublished_archive() {
    let root = private_root();
    let path = root.path().canonicalize().unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    // Explicitly replace only the verification boundary; filesystem publication
    // and cleanup are real. A failed readback never produces a usable receipt.
    let result = super::safety_snapshot_files::publish(&path, &id, b"fixture", |_| {
        Err(SafetySnapshotError::Corrupt)
    });
    assert_eq!(result, Err(SafetySnapshotError::Corrupt));
    assert!(!path.join(id).exists());
}
