use super::{
    commit::*,
    stage::FileChange,
    write_guard::{FilePrivacy, lock_app},
};
use std::{
    ffi::OsStr,
    fs,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
};
use vw_api_types::{agent_apps::AppId, id::TenantScope};
fn target() -> PendingTarget {
    PendingTarget {
        pointer: Some("provider-b".into()),
        mode: Some(ModeState {
            mode: Mode::Direct,
            attached: false,
            proxy_route: Some("route-kept".into()),
            contract: None,
        }),
    }
}
fn store(root: &std::path::Path) -> ProviderTransactions {
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    ProviderTransactions::new(root, ExecutionHostId::new("host-a").unwrap()).unwrap()
}
#[test]
fn cc04_commit_persists_intent_before_publish() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::write(live.path().join("config"), b"old").unwrap();
    let transactions = store(root.path());
    let path = live.path().to_owned();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |s, app, phase| {
            let pending =
                s.pending(app).unwrap().expect("durable intent must exist before publication");
            if phase == Phase::Intent || phase == Phase::Marked {
                assert_eq!(fs::read(path.join("config")).unwrap(), b"old");
                assert_eq!(pending.published, phase == Phase::Marked);
            }
            if phase == Phase::Published {
                assert!(pending.published);
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    transactions
        .commit(
            &TenantScope::new("p", "c").unwrap(),
            &mut guard,
            &[FileChange {
                name: OsStr::new("config"),
                privacy: FilePrivacy::Private,
                patch: &patch,
            }],
            target(),
        )
        .unwrap();
    HOOK.with_borrow_mut(|h| *h = None);
    assert!(transactions.pending(AppId::Claude).unwrap().is_none());
    assert_eq!(fs::read(live.path().join("config")).unwrap(), b"new");
    assert_eq!(store(root.path()).state(AppId::Claude).unwrap().pointer.unwrap().id, "provider-b");
}
#[test]
fn cc04_commit_state_failure_retains_pending() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let transactions = store(root.path());
    let state_path = root.path().join("provider-live/aG9zdC1h/claude.state.json");
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::Published {
                fs::create_dir(&state_path).unwrap();
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    assert!(
        transactions
            .commit(
                &TenantScope::new("p", "c").unwrap(),
                &mut guard,
                &[FileChange {
                    name: OsStr::new("config"),
                    privacy: FilePrivacy::Private,
                    patch: &patch
                }],
                target()
            )
            .is_err()
    );
    HOOK.with_borrow_mut(|h| *h = None);
    assert_eq!(fs::read(live.path().join("config")).unwrap(), b"new");
    let pending = store(root.path())
        .pending(AppId::Claude)
        .unwrap()
        .expect("failed settlement retains durable intent");
    assert!(pending.published);
    assert_eq!(pending.target.pointer.as_deref(), Some("provider-b"));
    assert_eq!(
        fs::metadata(root.path().join("provider-live/aG9zdC1h/claude.pending.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn cc04_commit_interruption_retains_durable_intent_and_staging() {
    for phase in [Phase::Intent, Phase::Marked, Phase::Settled] {
        let root = tempfile::tempdir().unwrap();
        let live = tempfile::tempdir().unwrap();
        fs::write(live.path().join("config"), b"old").unwrap();
        let transactions = store(root.path());
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(move |_, _, at| {
                if at == phase {
                    return Err(super::write_guard::WriteError::Io);
                }
                Ok(())
            }))
        });
        let mut guard = lock_app(AppId::Codex, live.path()).unwrap();
        let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
        let tenant = TenantScope::new("p", "c").unwrap();
        let changes = [FileChange {
            name: OsStr::new("config"),
            privacy: FilePrivacy::Private,
            patch: &patch,
        }];
        assert!(transactions.commit(&tenant, &mut guard, &changes, target()).is_err());
        HOOK.with_borrow_mut(|h| *h = None);
        let pending = store(root.path()).pending(AppId::Codex).unwrap().unwrap();
        assert_eq!(pending.published, phase != Phase::Intent);
        assert!(pending.tenant == tenant);
        assert!(pending.app == AppId::Codex);
        assert_eq!(pending.files.len(), 1);
        assert!(pending.files[0].pre.is_some());
        let staged = live.path().join(OsStr::from_bytes(&pending.files[0].staged));
        if phase == Phase::Settled {
            assert_eq!(fs::read(live.path().join("config")).unwrap(), b"new");
            assert!(!staged.exists());
            assert!(transactions.state(AppId::Codex).unwrap().pointer.is_some());
        } else {
            assert_eq!(fs::read(live.path().join("config")).unwrap(), b"old");
            assert_eq!(fs::read(staged).unwrap(), b"new");
            assert!(transactions.state(AppId::Codex).unwrap().pointer.is_none());
        }
        // Neither the originating tenant nor a different tenant may replace intent.
        for scope in [tenant, TenantScope::new("other-p", "other-c").unwrap()] {
            assert!(transactions.commit(&scope, &mut guard, &changes, target()).is_err());
        }
        assert_eq!(
            transactions.pending(AppId::Codex).unwrap().unwrap().files[0].planned,
            pending.files[0].planned
        );
    }
}

#[test]
fn cc04_commit_partial_publish_preserves_remaining_staged_file() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let transactions = store(root.path());
    let path = live.path().to_owned();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, phase| {
            if phase == Phase::Marked {
                fs::create_dir(path.join("b")).unwrap();
            }
            Ok(())
        }))
    });
    let mut guard = lock_app(AppId::Gemini, live.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    let changes = [
        FileChange { name: OsStr::new("a"), privacy: FilePrivacy::Private, patch: &patch },
        FileChange { name: OsStr::new("b"), privacy: FilePrivacy::Private, patch: &patch },
    ];
    assert!(
        transactions
            .commit(&TenantScope::new("p", "c").unwrap(), &mut guard, &changes, target())
            .is_err()
    );
    HOOK.with_borrow_mut(|h| *h = None);
    let pending = store(root.path()).pending(AppId::Gemini).unwrap().unwrap();
    assert!(pending.published);
    assert_eq!(fs::read(live.path().join("a")).unwrap(), b"new");
    assert!(live.path().join("b").is_dir());
    assert_eq!(
        fs::read(live.path().join(OsStr::from_bytes(&pending.files[1].staged))).unwrap(),
        b"new"
    );
    assert!(transactions.state(AppId::Gemini).unwrap().pointer.is_none());
}

#[test]
fn cc04_commit_state_is_idempotent_and_pointer_route_independent() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let transactions = store(root.path());
    let tenant = TenantScope::new("p", "c").unwrap();
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    let changes =
        [FileChange { name: OsStr::new("config"), privacy: FilePrivacy::Private, patch: &patch }];
    transactions.commit(&tenant, &mut guard, &changes, target()).unwrap();
    let first = transactions.state(AppId::Claude).unwrap();
    transactions.commit(&tenant, &mut guard, &changes, target()).unwrap();
    assert!(first == store(root.path()).state(AppId::Claude).unwrap());
    assert!(transactions.pending(AppId::Claude).unwrap().is_none());
    let mode = ModeState {
        mode: Mode::Proxy,
        attached: true,
        proxy_route: Some("another-route".into()),
        contract: None,
    };
    transactions
        .commit(&tenant, &mut guard, &[], PendingTarget { pointer: None, mode: Some(mode.clone()) })
        .unwrap();
    let state = store(root.path()).state(AppId::Claude).unwrap();
    assert!(state.pointer == first.pointer);
    assert!(state.mode == Some(mode));
    assert!(state.mode_tenant == Some(tenant));
    assert!(transactions.state(AppId::Codex).unwrap().pointer.is_none());
    let other_host =
        ProviderTransactions::new(root.path(), ExecutionHostId::new("host-b").unwrap()).unwrap();
    assert!(other_host.state(AppId::Claude).unwrap().pointer.is_none());
}

#[test]
fn cc04_commit_rejects_unsafe_root_and_invalid_target_without_publication() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        ProviderTransactions::new(root.path(), ExecutionHostId::new("host-a").unwrap()).is_err()
    );
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let links = tempfile::tempdir().unwrap();
    symlink(root.path(), links.path().join("root")).unwrap();
    assert!(
        ProviderTransactions::new(
            &links.path().join("root"),
            ExecutionHostId::new("host-a").unwrap()
        )
        .is_err()
    );
    assert!(
        ProviderTransactions::new(
            std::path::Path::new("relative"),
            ExecutionHostId::new("host-a").unwrap()
        )
        .is_err()
    );
    symlink(links.path(), root.path().join("provider-live")).unwrap();
    assert!(
        ProviderTransactions::new(root.path(), ExecutionHostId::new("host-a").unwrap()).is_err()
    );
    fs::remove_file(root.path().join("provider-live")).unwrap();
    let transactions = store(root.path());
    let live = tempfile::tempdir().unwrap();
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    assert!(
        transactions
            .commit(
                &TenantScope::new("p", "c").unwrap(),
                &mut guard,
                &[],
                PendingTarget { pointer: Some("../unsafe".into()), mode: None }
            )
            .is_err()
    );
    assert_eq!(fs::read_dir(live.path()).unwrap().count(), 0);
    assert!(transactions.pending(AppId::Claude).unwrap().is_none());
    assert!(transactions.state(AppId::Claude).unwrap().pointer.is_none());
}
