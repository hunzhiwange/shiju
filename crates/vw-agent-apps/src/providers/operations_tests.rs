use super::operations::*;
use vw_api_types::{
    app_providers::operations::ProviderOperationPhase, id::TaskId, task::TaskStatus,
};
#[test]
fn cc04_operations_cancel_before_publish_changes_nothing() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("native");
    std::fs::write(&file, b"old").unwrap();
    let op = ProviderOperation::new(TaskId::from("op"), "request".into());
    op.cancel();
    let result = op.run(|| {
        publication()?;
        std::fs::write(&file, b"new").unwrap();
        Ok(())
    });
    assert!(result.is_err(), "cancelled operation must refuse publication");
    assert_eq!(std::fs::read(file).unwrap(), b"old");
    assert_eq!(op.snapshot().status, TaskStatus::Cancelled);
}
#[test]
fn cc04_operations_cancel_after_publish_reports_settling() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("native");
    let op = ProviderOperation::new(TaskId::from("op"), "request".into());
    op.run(|| {
        planned(2);
        publication()?;
        std::fs::write(&file, b"first").unwrap();
        advanced();
        assert_eq!(op.cancel().phase, ProviderOperationPhase::Settling);
        publication()?;
        std::fs::write(&file, b"settled").unwrap();
        advanced();
        Ok(())
    })
    .unwrap();
    assert_eq!(std::fs::read(file).unwrap(), b"settled");
    assert_eq!(op.snapshot().status, TaskStatus::Completed);
    assert_eq!(op.snapshot().completed, 2);
}

#[cfg(target_os = "macos")]
#[test]
fn cc04_operations_real_transaction_settles_files_and_pointer_after_cancel() {
    use super::live::{
        commit::{ExecutionHostId, HOOK, PendingTarget, Phase, ProviderTransactions},
        stage::FileChange,
        write_guard::{FilePrivacy, lock_app},
    };
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    std::fs::set_permissions(
        root.path(),
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o700),
    )
    .unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    std::fs::write(live.path().join("native"), b"old").unwrap();
    let op = ProviderOperation::new(TaskId::from("op"), "request".into());
    let cancel = op.clone();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, at| {
            if at == Phase::Marked {
                assert_eq!(cancel.cancel().phase, ProviderOperationPhase::Settling);
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    op.run(|| {
        tx.commit(
            &TenantScope::new("p", "a").unwrap(),
            &mut guard,
            &[FileChange {
                name: std::ffi::OsStr::new("native"),
                privacy: FilePrivacy::Private,
                patch: &|_: Option<&[u8]>| Ok(b"new".to_vec()),
            }],
            PendingTarget { pointer: Some("target".into()), ..Default::default() },
        )
        .map_err(|_| "write failed".into())
    })
    .unwrap();
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(std::fs::read(live.path().join("native")).unwrap(), b"new");
    assert_eq!(tx.state(AppId::Claude).unwrap().pointer.unwrap().id, "target");
    assert!(tx.pending(AppId::Claude).unwrap().is_none());
    assert_eq!(op.snapshot().status, TaskStatus::Completed);
}

#[cfg(target_os = "macos")]
#[test]
fn cc04_operations_failed_publication_retains_pending_and_recovers() {
    use super::live::{
        commit::{ExecutionHostId, HOOK, PendingTarget, Phase, ProviderTransactions},
        stage::FileChange,
        write_guard::{FilePrivacy, WriteError, lock_app},
    };
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    std::fs::set_permissions(
        root.path(),
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o700),
    )
    .unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let op = ProviderOperation::new(TaskId::from("op"), "request".into());
    let cancel = op.clone();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, at| {
            if at == Phase::Marked {
                cancel.cancel();
                return Err(WriteError::Io);
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    assert!(
        op.run(|| tx
            .commit(
                &TenantScope::new("p", "a").unwrap(),
                &mut guard,
                &[FileChange {
                    name: std::ffi::OsStr::new("native"),
                    privacy: FilePrivacy::Private,
                    patch: &|_: Option<&[u8]>| Ok(b"new".to_vec())
                }],
                PendingTarget { pointer: Some("target".into()), ..Default::default() }
            )
            .map_err(|_| "write failed".into()))
            .is_err()
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(op.snapshot().phase, ProviderOperationPhase::Settling);
    assert_eq!(op.snapshot().status, TaskStatus::Failed);
    assert!(tx.pending(AppId::Claude).unwrap().unwrap().published);
    tx.recover(&mut guard).unwrap();
    assert_eq!(std::fs::read(live.path().join("native")).unwrap(), b"new");
    assert_eq!(tx.state(AppId::Claude).unwrap().pointer.unwrap().id, "target");
    assert!(tx.pending(AppId::Claude).unwrap().is_none());
}

#[tokio::test]
async fn cc04_operations_existing_executor_finishes_without_subscribers_and_rejects_resubmission() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("result");
    let op = ProviderOperation::new(TaskId::from("op"), "request".into());
    let work_file = file.clone();
    op.start(move || {
        publication()?;
        std::fs::write(work_file, b"complete").unwrap();
        Ok(())
    })
    .unwrap();
    assert!(op.start(|| Ok(())).is_err());
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while matches!(op.snapshot().status, TaskStatus::Queued | TaskStatus::Running) {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(std::fs::read(file).unwrap(), b"complete");
    assert_eq!(op.snapshot().status, TaskStatus::Completed);
}

#[cfg(target_os = "macos")]
#[test]
fn cc04_operations_cancel_during_staging_preserves_native_and_state() {
    use super::live::{
        commit::{ExecutionHostId, PendingTarget, ProviderTransactions},
        stage::FileChange,
        write_guard::{FilePrivacy, lock_app},
    };
    use vw_api_types::{agent_apps::AppId, id::TenantScope};
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    std::fs::write(live.path().join("native"), b"old").unwrap();
    std::fs::set_permissions(
        root.path(),
        <std::fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o700),
    )
    .unwrap();
    let tx = ProviderTransactions::new(root.path(), ExecutionHostId::new("host").unwrap()).unwrap();
    let op = ProviderOperation::new(TaskId::from("op"), "request".into());
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let patch = |_: Option<&[u8]>| {
        op.cancel();
        Ok(b"new".to_vec())
    };
    assert!(
        op.run(|| tx
            .commit(
                &TenantScope::new("p", "a").unwrap(),
                &mut guard,
                &[FileChange {
                    name: std::ffi::OsStr::new("native"),
                    privacy: FilePrivacy::Private,
                    patch: &patch
                }],
                PendingTarget { pointer: Some("target".into()), ..Default::default() }
            )
            .map_err(|_| "write cancelled".into()))
            .is_err()
    );
    assert_eq!(op.snapshot().status, TaskStatus::Cancelled);
    assert_eq!(std::fs::read(live.path().join("native")).unwrap(), b"old");
    assert!(tx.state(AppId::Claude).unwrap().pointer.is_none());
    assert!(tx.pending(AppId::Claude).unwrap().is_none());
    assert_eq!(std::fs::read_dir(live.path()).unwrap().count(), 1);
}
