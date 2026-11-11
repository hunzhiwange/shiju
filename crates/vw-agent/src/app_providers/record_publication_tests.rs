use super::*;
use std::{ffi::OsStr, fs, os::unix::fs::PermissionsExt};
use vw_agent_apps::providers::live::{
    commit::*,
    record_publication::*,
    stage::FileChange,
    write_guard::{FilePrivacy, WriteError, lock_app},
};
use vw_api_types::id::ProjectScope;

struct Fixture {
    _root: tempfile::TempDir,
    home: tempfile::TempDir,
    tx: ProviderTransactions,
    store: AppProviderStore,
    change: ProviderRecordChange,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let home = tempfile::tempdir().unwrap();
        let store = AppProviderStore::new(GlobalDatabase::for_home(root.path()).unwrap()).unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        let scope = ProjectScope::new(tenant, "project".into()).unwrap();
        let change = ProviderRecordChange {
            operation_id: "operation".into(),
            host: ExecutionHostId::new("host").unwrap(),
            project: scope.clone(),
            app: AppId::Claude,
            provider_id: "alpha".into(),
            before: r#"{"name":"Before"}"#.into(),
            after: r#"{"name":"After"}"#.into(),
            dependencies: vec![],
        };
        store
            .settings
            .0
            .upsert(scope.tenant_scope(), NAMESPACE, "\"claude\":alpha", &change.before)
            .unwrap();
        let tx = ProviderTransactions::new(root.path(), change.host.clone()).unwrap();
        let mut guard = lock_app(AppId::Claude, home.path()).unwrap();
        tx.commit_project_checked(
            &scope,
            &mut guard,
            &tx.state(AppId::Claude).unwrap(),
            true,
            &[],
            PendingTarget::default(),
            || Ok(()),
        )
        .unwrap();
        fs::write(home.path().join("config"), b"old").unwrap();
        Self { _root: root, home, tx, store, change }
    }
    fn raw(&self) -> String {
        self.store
            .settings
            .get(self.change.project.tenant_scope(), NAMESPACE, "\"claude\":alpha")
            .unwrap()
            .unwrap()
    }
    fn commit(&self, participant: &dyn ProviderRecordParticipant) -> Result<(), WriteError> {
        let mut guard = lock_app(AppId::Claude, self.home.path()).unwrap();
        let patch = |_: Option<&[u8]>| Ok(b"new".to_vec());
        self.tx.commit_project_record(
            &self.change.project,
            &mut guard,
            &self.tx.state(AppId::Claude)?,
            &[FileChange {
                name: OsStr::new("config"),
                privacy: FilePrivacy::Private,
                patch: &patch,
            }],
            PendingTarget::default(),
            &self.change,
            participant,
            || Ok(()),
        )
    }
}
#[test]
fn aggregation_t16_aggregation_resync_preserves_contract() {
    let f = Fixture::new();
    f.store.reserve(&f.change).unwrap();
    assert_eq!(f.raw(), f.change.before);
    assert!(
        !f.store
            .settings
            .compare_exchange(
                f.change.project.tenant_scope(),
                NAMESPACE,
                "\"claude\":alpha",
                &f.change.before,
                r#"{"name":"racing"}"#
            )
            .unwrap(),
        "a concurrent record write must not replace durable publication reservation"
    );
    f.store.finish(&f.change, true).unwrap();
    assert_eq!(f.raw(), f.change.after);
    f.store.finish(&f.change, true).unwrap();
}
struct Fail<'a> {
    store: &'a AppProviderStore,
    after_files: bool,
}
impl ProviderRecordParticipant for Fail<'_> {
    fn reserve(&self, change: &ProviderRecordChange) -> Result<(), WriteError> {
        self.store.reserve(change)?;
        if self.after_files { Ok(()) } else { Err(WriteError::Io) }
    }
    fn finish(&self, _: &ProviderRecordChange, _: bool) -> Result<(), WriteError> {
        Err(WriteError::Io)
    }
}
#[test]
fn aggregation_t16_record_publication_recovers_both_sides_of_marker() {
    for after_files in [false, true] {
        let f = Fixture::new();
        assert!(f.commit(&Fail { store: &f.store, after_files }).is_err());
        assert_eq!(f.raw(), f.change.before);
        let pending = f.tx.pending(AppId::Claude).unwrap().unwrap();
        assert_eq!(pending.published, after_files);
        let mut guard = lock_app(AppId::Claude, f.home.path()).unwrap();
        assert!(
            f.tx.recover_project(&f.change.project, &mut guard).is_err(),
            "unaware recovery must retain joint intent"
        );
        assert!(f.tx.pending(AppId::Claude).unwrap().is_some());
        let wrong =
            ProjectScope::new(f.change.project.tenant_scope().clone(), "other".into()).unwrap();
        assert!(f.tx.recover_project_record(&wrong, &mut guard, &f.store).is_err());
        let reopened = ProviderTransactions::open_existing(
            f._root.path(),
            ExecutionHostId::new("host").unwrap(),
        )
        .unwrap()
        .unwrap();
        let reopened_store =
            AppProviderStore::new(GlobalDatabase::for_home(f._root.path()).unwrap()).unwrap();
        reopened.recover_project_record(&f.change.project, &mut guard, &reopened_store).unwrap();
        assert!(f.tx.pending(AppId::Claude).unwrap().is_none());
        assert_eq!(
            f.raw(),
            if after_files { f.change.after.as_str() } else { f.change.before.as_str() }
        );
        assert_eq!(
            fs::read(f.home.path().join("config")).unwrap(),
            if after_files { b"new" } else { b"old" }
        );
    }
}
#[test]
fn aggregation_t16_record_intent_rejects_foreign_reservation_and_stale_cas() {
    let f = Fixture::new();
    f.store.reserve(&f.change).unwrap();
    let mut other = f.change.clone();
    other.operation_id = "other".into();
    assert!(f.store.reserve(&other).is_err());
    assert!(f.store.finish(&other, true).is_err());
    assert!(
        !f.store
            .settings
            .delete(f.change.project.tenant_scope(), NAMESPACE, "\"claude\":alpha")
            .unwrap()
    );
    f.store.finish(&f.change, false).unwrap();
    assert_eq!(f.raw(), f.change.before);
    f.store
        .settings
        .0
        .upsert(
            f.change.project.tenant_scope(),
            NAMESPACE,
            "\"claude\":alpha",
            r#"{"name":"concurrent"}"#,
        )
        .unwrap();
    assert!(f.store.reserve(&f.change).is_err());
}

#[test]
fn aggregation_t16_all_catalog_inputs_are_reserved_and_recovered() {
    let mut f = Fixture::new();
    let key = "\"claude\":beta";
    let tenant = f.change.project.tenant_scope();
    f.store.settings.0.upsert(tenant, NAMESPACE, key, r#"{"name":"Beta before"}"#).unwrap();
    f.change.dependencies.push(ProviderRecordDelta {
        provider_id: "beta".into(),
        before: r#"{"name":"Beta before"}"#.into(),
        after: r#"{"name":"Beta after"}"#.into(),
    });
    assert!(f.commit(&Fail { store: &f.store, after_files: true }).is_err());
    assert!(
        !f.store
            .settings
            .compare_exchange(
                tenant,
                NAMESPACE,
                key,
                r#"{"name":"Beta before"}"#,
                r#"{"name":"racing"}"#
            )
            .unwrap()
    );
    let mut guard = lock_app(AppId::Claude, f.home.path()).unwrap();
    // Simulate a process dying after one database row settled.
    let raw = f.store.settings.0.get(tenant, NAMESPACE, "\"claude\":alpha").unwrap().unwrap();
    assert!(
        f.store
            .settings
            .0
            .compare_exchange(tenant, NAMESPACE, "\"claude\":alpha", &raw, &f.change.after)
            .unwrap()
    );
    f.tx.recover_project_record(&f.change.project, &mut guard, &f.store).unwrap();
    assert_eq!(f.raw(), f.change.after);
    assert_eq!(
        f.store.settings.get(tenant, NAMESPACE, key).unwrap().as_deref(),
        Some(r#"{"name":"Beta after"}"#)
    );
}
#[test]
fn aggregation_t16_concurrent_reservations_have_one_owner() {
    let f = Fixture::new();
    let mut other = f.change.clone();
    other.operation_id = "other".into();
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let first = scope.spawn(|| {
            barrier.wait();
            f.store.reserve(&f.change)
        });
        let second = scope.spawn(|| {
            barrier.wait();
            f.store.reserve(&other)
        });
        (first.join().unwrap(), second.join().unwrap())
    });
    assert_ne!(results.0.is_ok(), results.1.is_ok());
    f.store.finish(&f.change, false).unwrap();
    f.store.finish(&other, false).unwrap();
    assert_eq!(f.raw(), f.change.before);
}
#[test]
fn aggregation_t16_external_edit_retains_joint_recovery_intent() {
    let f = Fixture::new();
    assert!(f.commit(&Fail { store: &f.store, after_files: true }).is_err());
    fs::write(f.home.path().join("config"), b"external").unwrap();
    let mut guard = lock_app(AppId::Claude, f.home.path()).unwrap();
    assert!(f.tx.recover_project_record(&f.change.project, &mut guard, &f.store).is_err());
    assert!(f.tx.pending(AppId::Claude).unwrap().is_some());
    assert_eq!(f.raw(), f.change.before);
    assert_eq!(fs::read(f.home.path().join("config")).unwrap(), b"external");
    fs::write(f.home.path().join("config"), b"new").unwrap();
    f.tx.recover_project_record(&f.change.project, &mut guard, &f.store).unwrap();
    assert_eq!(f.raw(), f.change.after);
}
