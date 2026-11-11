use super::{commit::*, recovery::RecoveryOutcome, stage::FileChange, write_guard::*};
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

fn store(root: &std::path::Path) -> ProviderTransactions {
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    ProviderTransactions::new(root, ExecutionHostId::new("host-a").unwrap()).unwrap()
}
fn interrupt(store: &ProviderTransactions, guard: &mut AppWriteGuard, phase: Phase) {
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) }))
    });
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    let changes = ["a", "b"].map(|name| FileChange {
        name: OsStr::new(name),
        privacy: FilePrivacy::Private,
        patch: &patch,
    });
    assert!(
        store
            .commit(
                &TenantScope::new("p", "c").unwrap(),
                guard,
                &changes,
                PendingTarget { pointer: Some("next".into()), mode: None, ..Default::default() }
            )
            .is_err()
    );
    HOOK.with_borrow_mut(|h| *h = None);
}
#[test]
fn cc04_recovery_discards_unpublished() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    fs::write(live.path().join("a"), b"old").unwrap();
    let s = store(root.path());
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    interrupt(&s, &mut guard, Phase::Intent);
    assert_eq!(s.recover(&mut guard).unwrap(), Some(RecoveryOutcome::Discarded));
    assert_eq!(fs::read(live.path().join("a")).unwrap(), b"old");
    assert!(!live.path().join("b").exists());
    assert!(s.state(AppId::Claude).unwrap().pointer.is_none());
    assert!(s.pending(AppId::Claude).unwrap().is_none());
    assert_eq!(fs::read_dir(live.path()).unwrap().count(), 1);
}
#[test]
fn cc04_recovery_rolls_forward_without_overwriting_external_edits() {
    for phase in [Phase::Marked, Phase::FilePublished, Phase::Published, Phase::Settled] {
        let root = tempfile::tempdir().unwrap();
        let live = tempfile::tempdir().unwrap();
        fs::write(live.path().join("a"), b"old").unwrap();
        let s = store(root.path());
        let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
        interrupt(&s, &mut guard, phase);
        fs::write(live.path().join("a"), b"external").unwrap();
        assert_eq!(
            s.recover(&mut guard).unwrap(),
            Some(RecoveryOutcome::RolledForwardExcept { names: vec![b"a".to_vec()] })
        );
        assert_eq!(fs::read(live.path().join("a")).unwrap(), b"external");
        assert_eq!(fs::read(live.path().join("b")).unwrap(), b"new");
        assert_eq!(s.state(AppId::Claude).unwrap().pointer.unwrap().id, "next");
        assert!(s.pending(AppId::Claude).unwrap().is_none());
        assert_eq!(fs::read_dir(live.path()).unwrap().count(), 2);
        assert_eq!(s.recover(&mut guard).unwrap(), None);
    }
}

#[test]
fn cc04_recovery_startup_reopens_only_registered_identity() {
    for replace in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let parent = tempfile::tempdir().unwrap();
        let path = parent.path().join("live");
        fs::create_dir(&path).unwrap();
        let s = store(root.path());
        let mut guard = lock_app(AppId::Claude, &path).unwrap();
        interrupt(&s, &mut guard, Phase::Marked);
        drop(guard);
        drop(s);
        if replace {
            fs::rename(&path, parent.path().join("old")).unwrap();
            fs::create_dir(&path).unwrap();
            fs::write(path.join("a"), b"external").unwrap();
        }
        let restarted = store(root.path());
        let outcomes = restarted.recover_on_startup();
        assert_eq!(outcomes.len(), 1);
        if replace {
            assert_eq!(outcomes[0].1, Err(WriteError::InvalidResource));
            assert_eq!(fs::read(path.join("a")).unwrap(), b"external");
            assert!(restarted.pending(AppId::Claude).unwrap().is_some());
        } else {
            assert_eq!(outcomes[0].1, Ok(RecoveryOutcome::RolledForward));
            assert_eq!(fs::read(path.join("a")).unwrap(), b"new");
            assert!(restarted.pending(AppId::Claude).unwrap().is_none());
        }
    }
}
#[test]
fn cc04_recovery_next_commit_recovers_and_requires_replanning() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let s = store(root.path());
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    interrupt(&s, &mut guard, Phase::Marked);
    let tenant = TenantScope::new("other", "tenant").unwrap();
    let target =
        PendingTarget { pointer: Some("unplanned".into()), mode: None, ..Default::default() };
    assert_eq!(s.commit(&tenant, &mut guard, &[], target.clone()), Err(WriteError::Recovered));
    assert_eq!(s.state(AppId::Claude).unwrap().pointer.unwrap().id, "next");
    assert_eq!(s.commit(&tenant, &mut guard, &[], target), Ok(()));
    assert_eq!(s.state(AppId::Claude).unwrap().pointer.unwrap().id, "unplanned");
}
#[test]
fn cc04_recovery_missing_staging_is_partial_and_other_files_finish() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let s = store(root.path());
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    interrupt(&s, &mut guard, Phase::Marked);
    let pending = s.pending(AppId::Claude).unwrap().unwrap();
    use std::os::unix::ffi::OsStrExt;
    fs::remove_file(live.path().join(OsStr::from_bytes(&pending.files[0].staged))).unwrap();
    assert_eq!(
        s.recover(&mut guard).unwrap(),
        Some(RecoveryOutcome::RolledForwardExcept { names: vec![b"a".to_vec()] })
    );
    assert!(!live.path().join("a").exists());
    assert_eq!(fs::read(live.path().join("b")).unwrap(), b"new");
    assert_eq!(s.state(AppId::Claude).unwrap().pointer.unwrap().id, "next");
}
#[test]
fn cc04_recovery_rejects_unsafe_pending_names_without_side_effects() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let s = store(root.path());
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    interrupt(&s, &mut guard, Phase::Marked);
    let mut pending = s.pending(AppId::Claude).unwrap().unwrap();
    pending.files[1].staged = b"../outside".to_vec();
    s.save(AppId::Claude, "pending", &pending).unwrap();
    assert_eq!(s.recover(&mut guard), Err(WriteError::InvalidResource));
    assert!(!live.path().join("a").exists());
    assert!(s.state(AppId::Claude).unwrap().pointer.is_none());
    assert!(s.pending(AppId::Claude).unwrap().is_some());
}

#[test]
fn cc04_recovery_startup_rejects_symlink_and_retains_intent() {
    let root = tempfile::tempdir().unwrap();
    let parent = tempfile::tempdir().unwrap();
    let path = parent.path().join("live");
    fs::create_dir(&path).unwrap();
    let s = store(root.path());
    let mut guard = lock_app(AppId::Claude, &path).unwrap();
    interrupt(&s, &mut guard, Phase::Marked);
    drop(guard);
    let moved = parent.path().join("moved");
    fs::rename(&path, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &path).unwrap();
    assert_eq!(s.recover_on_startup()[0].1, Err(WriteError::InvalidResource));
    assert!(s.pending(AppId::Claude).unwrap().is_some());
    assert!(!moved.join("a").exists());
    fs::remove_file(&path).unwrap();
    fs::rename(&moved, &path).unwrap();
    assert_eq!(s.recover_on_startup()[0].1, Ok(RecoveryOutcome::RolledForward));
}
#[test]
fn cc04_recovery_state_failure_retains_intent_and_retry_is_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let live = tempfile::tempdir().unwrap();
    let s = store(root.path());
    let mut guard = lock_app(AppId::Claude, live.path()).unwrap();
    interrupt(&s, &mut guard, Phase::Marked);
    let state = root.path().join("provider-live/aG9zdC1h/claude.state.json");
    fs::create_dir(&state).unwrap();
    assert!(s.recover(&mut guard).is_err());
    assert!(s.pending(AppId::Claude).unwrap().is_some());
    assert!(
        !live.path().join("b").exists(),
        "unreadable project ownership must stop recovery before native publication"
    );
    fs::remove_dir(state).unwrap();
    assert_eq!(s.recover(&mut guard).unwrap(), Some(RecoveryOutcome::RolledForward));
    assert_eq!(s.recover(&mut guard).unwrap(), None);
}
