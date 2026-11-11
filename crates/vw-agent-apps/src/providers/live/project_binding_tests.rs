use super::{
    commit::*,
    stage::FileChange,
    write_guard::{FilePrivacy, WriteError, lock_app},
};
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt};
use vw_api_types::{
    agent_apps::AppId,
    id::{ProjectId, ProjectScope, TenantScope},
};
fn scope(id: &str) -> ProjectScope {
    ProjectScope::new(TenantScope::new("p", "c").unwrap(), ProjectId::from(id)).unwrap()
}
struct Fixture {
    _private: tempfile::TempDir,
    live: tempfile::TempDir,
    tx: ProviderTransactions,
}
impl Fixture {
    fn new() -> Self {
        let private = tempfile::tempdir().unwrap();
        let live = tempfile::tempdir().unwrap();
        fs::set_permissions(private.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let tx = ProviderTransactions::new(private.path(), ExecutionHostId::new("host").unwrap())
            .unwrap();
        fs::write(live.path().join("config"), b"old").unwrap();
        Self { _private: private, live, tx }
    }
}
#[test]
fn aggregation_t14_project_claim_is_explicit_atomic_and_exclusive() {
    let f = Fixture::new();
    let mut guard = lock_app(AppId::Claude, f.live.path()).unwrap();
    let a = scope("a");
    let b = scope("b");
    let state = f.tx.state(AppId::Claude).unwrap();
    assert!(state.project_scope.is_none());
    assert_eq!(state.require_project(&a), Err(WriteError::Conflict));
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    let changes =
        [FileChange { name: OsStr::new("config"), privacy: FilePrivacy::Private, patch: &patch }];
    assert_eq!(
        f.tx.commit_project_checked(
            &a,
            &mut guard,
            &state,
            false,
            &changes,
            PendingTarget::default(),
            || Ok(())
        ),
        Err(WriteError::Conflict)
    );
    assert_eq!(
        f.tx.commit_project_checked(
            &a,
            &mut guard,
            &state,
            true,
            &changes,
            PendingTarget::default(),
            || Err(WriteError::Conflict)
        ),
        Err(WriteError::Conflict)
    );
    assert_eq!(fs::read(f.live.path().join("config")).unwrap(), b"old");
    assert!(f.tx.state(AppId::Claude).unwrap().project_scope.is_none());
    f.tx.commit_project_checked(
        &a,
        &mut guard,
        &state,
        true,
        &changes,
        PendingTarget::default(),
        || Ok(()),
    )
    .unwrap();
    let bound = f.tx.state(AppId::Claude).unwrap();
    bound.require_project(&a).unwrap();
    assert!(bound.require_project(&b).is_err());
    assert_eq!(
        f.tx.commit_project_checked(
            &b,
            &mut guard,
            &bound,
            true,
            &changes,
            PendingTarget::default(),
            || Ok(())
        ),
        Err(WriteError::Conflict)
    );
    assert_eq!(
        f.tx.commit(a.tenant_scope(), &mut guard, &[], PendingTarget::default()),
        Err(WriteError::Conflict)
    );
    assert_eq!(
        f.tx.commit_project_checked(
            &a,
            &mut guard,
            &state,
            true,
            &[],
            PendingTarget::default(),
            || Ok(())
        ),
        Err(WriteError::Conflict)
    );
    assert_eq!(fs::read(f.live.path().join("config")).unwrap(), b"new");
}
#[test]
fn aggregation_t14_project_recovery_retains_binding_and_external_edits() {
    for phase in [Phase::Intent, Phase::Marked, Phase::Published, Phase::Settled] {
        let f = Fixture::new();
        let a = scope("a");
        let b = scope("b");
        let mut guard = lock_app(AppId::Claude, f.live.path()).unwrap();
        let state = f.tx.state(AppId::Claude).unwrap();
        HOOK.with_borrow_mut(|h| {
            *h = Some(Box::new(
                move |_, _, at| if at == phase { Err(WriteError::Io) } else { Ok(()) },
            ))
        });
        let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
        let changes = [FileChange {
            name: OsStr::new("config"),
            privacy: FilePrivacy::Private,
            patch: &patch,
        }];
        assert_eq!(
            f.tx.commit_project_checked(
                &a,
                &mut guard,
                &state,
                true,
                &changes,
                PendingTarget::default(),
                || Ok(())
            ),
            Err(WriteError::Io)
        );
        HOOK.with_borrow_mut(|h| *h = None);
        let pending = f.tx.pending(AppId::Claude).unwrap().unwrap();
        assert_eq!(pending.version, 7);
        assert_eq!(pending.target.project_scope, Some(a.clone()));
        assert_eq!(f.tx.recover_project(&b, &mut guard), Err(WriteError::Conflict));
        assert_eq!(f.tx.recover(&mut guard), Err(WriteError::Conflict));
        if phase == Phase::Marked {
            fs::write(f.live.path().join("config"), b"external").unwrap();
        }
        f.tx.recover_project(&a, &mut guard).unwrap();
        let result = f.tx.state(AppId::Claude).unwrap();
        if phase == Phase::Intent {
            assert!(result.project_scope.is_none());
            assert_eq!(fs::read(f.live.path().join("config")).unwrap(), b"old");
        } else {
            result.require_project(&a).unwrap();
        }
        if phase == Phase::Marked {
            assert_eq!(fs::read(f.live.path().join("config")).unwrap(), b"external");
        }
        assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
    }
}
#[test]
fn aggregation_t14_project_recovery_rejects_changed_preimage_before_files() {
    let f = Fixture::new();
    let a = scope("a");
    let b = scope("b");
    let mut guard = lock_app(AppId::Claude, f.live.path()).unwrap();
    let state = f.tx.state(AppId::Claude).unwrap();
    HOOK.with_borrow_mut(|h| {
        *h = Some(Box::new(
            |_, _, at| if at == Phase::Marked { Err(WriteError::Io) } else { Ok(()) },
        ))
    });
    let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
    let changes =
        [FileChange { name: OsStr::new("config"), privacy: FilePrivacy::Private, patch: &patch }];
    assert!(
        f.tx.commit_project_checked(
            &a,
            &mut guard,
            &state,
            true,
            &changes,
            PendingTarget::default(),
            || Ok(())
        )
        .is_err()
    );
    HOOK.with_borrow_mut(|h| *h = None);
    let mut other = state;
    other.project_scope = Some(b);
    f.tx.save(AppId::Claude, "state", &other).unwrap();
    assert_eq!(f.tx.recover_project(&a, &mut guard), Err(WriteError::Conflict));
    assert_eq!(fs::read(f.live.path().join("config")).unwrap(), b"old");
    assert!(f.tx.pending(AppId::Claude).unwrap().is_some());
}
#[test]
fn aggregation_t14_project_legacy_reads_never_claim_and_tenant_mismatch_rejects() {
    let legacy: AppState =
        serde_json::from_str(r#"{"pointer":null,"mode":null,"mode_tenant":null}"#).unwrap();
    assert!(legacy.project_scope.is_none());
    assert!(legacy.require_project(&scope("a")).is_err());
    let f = Fixture::new();
    let mut guard = lock_app(AppId::Claude, f.live.path()).unwrap();
    let target = PendingTarget { project_scope: Some(scope("a")), ..Default::default() };
    assert_eq!(
        f.tx.commit(&TenantScope::new("other", "tenant").unwrap(), &mut guard, &[], target),
        Err(WriteError::Conflict)
    );
    assert!(f.tx.state(AppId::Claude).unwrap().project_scope.is_none());
}
