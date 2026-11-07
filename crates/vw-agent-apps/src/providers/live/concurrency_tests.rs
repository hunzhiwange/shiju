use super::{
    commit::{ExecutionHostId, HOOK, PendingTarget, Phase, ProviderTransactions},
    concurrency::Guarded,
    stage::FileChange,
    write_guard::{FilePrivacy, WriteError, lock_app},
};
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

struct ResetHook;
impl Drop for ResetHook {
    fn drop(&mut self) {
        HOOK.with_borrow_mut(|h| *h = None);
    }
}
fn store(root: &std::path::Path) -> ProviderTransactions {
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    ProviderTransactions::new(root, ExecutionHostId::new("host-a").unwrap()).unwrap()
}
fn target() -> PendingTarget {
    PendingTarget { pointer: Some("provider-b".into()), mode: None, ..Default::default() }
}
fn key_patch(before: Option<&[u8]>) -> Result<Vec<u8>, WriteError> {
    let mut value: serde_json::Value =
        serde_json::from_slice(before.unwrap()).map_err(|_| WriteError::InvalidResource)?;
    value["key"] = "new".into();
    serde_json::to_vec(&value).map_err(|_| WriteError::InvalidResource)
}
fn no_staging(live: &std::path::Path) {
    assert!(
        fs::read_dir(live).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".shiju-live-"))
    );
}

#[test]
fn cc04_concurrency_replans_key_patch() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::write(live.path().join("config"), br#"{"key":"old"}"#).unwrap();
    let transactions = store(root.path());
    let path = live.path().join("config");
    let _reset = ResetHook;
    let mut fired = false;
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::BeforePublish && !fired {
                fired = true;
                fs::write(&path, br#"{"key":"old","user":42}"#).unwrap();
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    transactions
        .commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &[FileChange {
                name: OsStr::new("config"),
                privacy: FilePrivacy::Private,
                patch: &key_patch,
            }],
            target(),
        )
        .unwrap();
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(live.path().join("config")).unwrap()).unwrap();
    assert_eq!(result, serde_json::json!({"key":"new","user":42}));
    assert!(transactions.pending(AppId::Claude).unwrap().is_none());
    assert_eq!(transactions.state(AppId::Claude).unwrap().pointer.unwrap().id, "provider-b");
    no_staging(live.path());
}

#[test]
fn cc04_concurrency_guarded_write_rejects_rotated_auth() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::write(live.path().join("auth"), b"fake-old-login").unwrap();
    let transactions = store(root.path());
    let guarded = Guarded::new(Some(b"fake-old-login"), b"fake-managed-login".to_vec());
    let patch = |before: Option<&[u8]>| guarded.apply(before);
    let path = live.path().join("auth");
    let _reset = ResetHook;
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::BeforePublish {
                fs::write(&path, b"fake-rotated-login").unwrap();
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Codex, live.path()).unwrap();
    assert_eq!(
        transactions.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &[FileChange {
                name: OsStr::new("auth"),
                privacy: FilePrivacy::Private,
                patch: &patch
            }],
            target()
        ),
        Err(WriteError::Conflict)
    );
    assert_eq!(fs::read(live.path().join("auth")).unwrap(), b"fake-rotated-login");
    assert!(transactions.pending(AppId::Codex).unwrap().is_none());
    assert!(transactions.state(AppId::Codex).unwrap().pointer.is_none());
    no_staging(live.path());
}

#[test]
fn cc04_concurrency_exhaustion_preserves_external_edits_and_state() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::write(live.path().join("a"), br#"{"key":"old","user":0}"#).unwrap();
    fs::write(live.path().join("b"), b"untouched").unwrap();
    let transactions = store(root.path());
    let path = live.path().join("a");
    let _reset = ResetHook;
    let mut round = 0;
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::BeforePublish {
                round += 1;
                fs::write(&path, format!(r#"{{"key":"old","user":{round}}}"#)).unwrap();
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let other = |_: Option<&[u8]>| Ok(b"changed".to_vec());
    let changes = [
        FileChange { name: OsStr::new("a"), privacy: FilePrivacy::Private, patch: &key_patch },
        FileChange { name: OsStr::new("b"), privacy: FilePrivacy::Private, patch: &other },
    ];
    assert_eq!(
        transactions.commit(&TenantScope::new("p", "c").unwrap(), &mut guard, &changes, target()),
        Err(WriteError::Conflict)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(live.path().join("a")).unwrap()).unwrap();
    assert_eq!(value, serde_json::json!({"key":"old","user":4}));
    assert_eq!(fs::read(live.path().join("b")).unwrap(), b"untouched");
    assert!(transactions.pending(AppId::Claude).unwrap().is_none());
    assert!(transactions.state(AppId::Claude).unwrap().pointer.is_none());
    assert!(transactions.recover(&mut guard).unwrap().is_none());
    no_staging(live.path());
}

#[test]
fn cc04_concurrency_partial_conflict_retains_recoverable_intent() {
    use super::recovery::RecoveryOutcome;
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::write(live.path().join("config"), br#"{"key":"old"}"#).unwrap();
    fs::write(live.path().join("auth"), b"fake-old-login").unwrap();
    let transactions = store(root.path());
    let guarded = Guarded::new(Some(b"fake-old-login"), b"fake-managed-login".to_vec());
    let patch = |before: Option<&[u8]>| guarded.apply(before);
    let path = live.path().join("auth");
    let _reset = ResetHook;
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::FilePublished {
                fs::write(&path, b"fake-rotated-login").unwrap();
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Codex, live.path()).unwrap();
    let changes = [
        FileChange { name: OsStr::new("config"), privacy: FilePrivacy::Private, patch: &key_patch },
        FileChange { name: OsStr::new("auth"), privacy: FilePrivacy::Private, patch: &patch },
    ];
    assert_eq!(
        transactions.commit(&TenantScope::new("p", "c").unwrap(), &mut guard, &changes, target()),
        Err(WriteError::Conflict)
    );
    assert!(transactions.pending(AppId::Codex).unwrap().unwrap().published);
    assert_eq!(fs::read(live.path().join("config")).unwrap(), br#"{"key":"new"}"#);
    assert!(transactions.state(AppId::Codex).unwrap().pointer.is_none());
    assert_eq!(
        store(root.path()).recover(&mut guard).unwrap(),
        Some(RecoveryOutcome::RolledForwardExcept { names: vec![b"auth".to_vec()] })
    );
    assert_eq!(fs::read(live.path().join("auth")).unwrap(), b"fake-rotated-login");
    assert_eq!(transactions.state(AppId::Codex).unwrap().pointer.unwrap().id, "provider-b");
    no_staging(live.path());
}

#[test]
fn cc04_concurrency_replanned_pending_survives_interruption() {
    use super::{recovery::RecoveryOutcome, stage::digest};
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::write(live.path().join("config"), br#"{"key":"old"}"#).unwrap();
    let transactions = store(root.path());
    let path = live.path().to_owned();
    let _reset = ResetHook;
    let mut round = 0;
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |s, app, phase| {
            if phase == Phase::BeforePublish {
                round += 1;
                if round == 1 {
                    fs::write(path.join("config"), br#"{"key":"old","user":42}"#).unwrap();
                } else {
                    let pending = s.pending(app).unwrap().unwrap();
                    assert_eq!(
                        pending.files[0].pre,
                        Some(digest(&fs::read(path.join("config")).unwrap()))
                    );
                    let bytes =
                        fs::read(path.join(OsStr::from_bytes(&pending.files[0].staged))).unwrap();
                    assert_eq!(pending.files[0].planned, Some(digest(&bytes)));
                    assert_eq!(
                        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
                        serde_json::json!({"key":"new","user":42})
                    );
                    panic!("simulated crash after durable replan");
                }
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let crashed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        transactions.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &[FileChange {
                name: OsStr::new("config"),
                privacy: FilePrivacy::Private,
                patch: &key_patch,
            }],
            target(),
        )
    }));
    assert!(crashed.is_err());
    assert!(transactions.pending(AppId::Claude).unwrap().unwrap().published);
    assert_eq!(
        store(root.path()).recover(&mut guard).unwrap(),
        Some(RecoveryOutcome::RolledForward)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(live.path().join("config")).unwrap()).unwrap();
    assert_eq!(value, serde_json::json!({"key":"new","user":42}));
    assert_eq!(transactions.state(AppId::Claude).unwrap().pointer.unwrap().id, "provider-b");
    no_staging(live.path());
}

#[test]
fn cc04_concurrency_guarded_rejects_missing_and_rotated_snapshots() {
    let absent = Guarded::new(None, b"fake-managed".to_vec());
    assert_eq!(absent.apply(Some(b"fake-login")), Err(WriteError::Conflict));
    let present = Guarded::new(Some(b"fake-old"), b"fake-managed".to_vec());
    assert_eq!(present.apply(None), Err(WriteError::Conflict));
    assert_eq!(present.apply(Some(b"fake-rotated")), Err(WriteError::Conflict));
    assert_eq!(present.apply(Some(b"fake-old")), Ok(b"fake-managed".to_vec()));
}

#[test]
fn cc04_concurrency_symlink_rotation_rejects_without_touching_destination() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(live.path().join("config"), br#"{"key":"old"}"#).unwrap();
    fs::write(outside.path().join("config"), b"outside").unwrap();
    let transactions = store(root.path());
    let path = live.path().join("config");
    let destination = outside.path().join("config");
    let _reset = ResetHook;
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::BeforePublish {
                fs::remove_file(&path).unwrap();
                symlink(&destination, &path).unwrap();
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    assert_eq!(
        transactions.commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &[FileChange {
                name: OsStr::new("config"),
                privacy: FilePrivacy::Private,
                patch: &key_patch
            }],
            target()
        ),
        Err(WriteError::InvalidResource)
    );
    assert!(fs::symlink_metadata(live.path().join("config")).unwrap().file_type().is_symlink());
    assert_eq!(fs::read(outside.path().join("config")).unwrap(), b"outside");
    assert!(transactions.pending(AppId::Claude).unwrap().is_none());
    assert!(transactions.state(AppId::Claude).unwrap().pointer.is_none());
    no_staging(live.path());
}

#[test]
fn cc04_concurrency_replan_to_noop_cleans_staging() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::write(live.path().join("config"), br#"{"key":"old"}"#).unwrap();
    let transactions = store(root.path());
    let path = live.path().join("config");
    let _reset = ResetHook;
    let mut fired = false;
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::BeforePublish && !fired {
                fired = true;
                fs::write(&path, br#"{"key":"new","user":42}"#).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    transactions
        .commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &[FileChange {
                name: OsStr::new("config"),
                privacy: FilePrivacy::Private,
                patch: &key_patch,
            }],
            target(),
        )
        .unwrap();
    assert_eq!(fs::read(live.path().join("config")).unwrap(), br#"{"key":"new","user":42}"#);
    // No rename: preserve the external file including its permissions.
    assert_eq!(
        fs::metadata(live.path().join("config")).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert!(transactions.pending(AppId::Claude).unwrap().is_none());
    assert_eq!(transactions.state(AppId::Claude).unwrap().pointer.unwrap().id, "provider-b");
    no_staging(live.path());
}
