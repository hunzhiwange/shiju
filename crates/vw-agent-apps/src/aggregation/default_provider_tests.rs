use super::{MembershipChange, MembershipError, SavedMember, set_member};
use crate::providers::{
    live::{
        commit::{AppState, ExecutionHostId, Mode, ModeState, PendingTarget, ProviderTransactions},
        write_guard::{AppWriteGuard, lock_app},
    },
    mode_state::{StackState, observe},
};
use std::{cell::Cell, fs, os::unix::fs::PermissionsExt};
use tempfile::TempDir;
use vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthService, id::TenantScope};

struct Fixture {
    root: TempDir,
    _live: TempDir,
    host: ExecutionHostId,
    tenant: TenantScope,
    transactions: ProviderTransactions,
    guard: AppWriteGuard,
}

impl Fixture {
    fn new(app: AppId) -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let live = tempfile::tempdir().unwrap();
        let guard = lock_app(app, live.path()).unwrap();
        let host = ExecutionHostId::new("fixture-host").unwrap();
        let transactions = ProviderTransactions::new(root.path(), host.clone()).unwrap();
        let tenant = TenantScope::new("platform", "company").unwrap();
        Self { root, _live: live, host, tenant, transactions, guard }
    }

    fn seed(&mut self, attached: bool) -> AppState {
        self.transactions
            .commit(
                &self.tenant,
                &mut self.guard,
                &[],
                PendingTarget {
                    pointer: Some("direct".into()),
                    mode: Some(ModeState {
                        mode: Mode::Proxy,
                        attached,
                        proxy_route: Some("old".into()),
                        contract: None,
                    }),
                    stack: Some(StackState {
                        enabled: true,
                        members: vec!["old".into(), "peer".into()],
                        keys: [("old".into(), "old".into()), ("peer".into(), "peer".into())].into(),
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        self.state()
    }

    fn state(&self) -> AppState {
        ProviderTransactions::open_existing(self.root.path(), self.host.clone())
            .unwrap()
            .unwrap()
            .state(self.guard.app())
            .unwrap()
    }

    fn change(
        &mut self,
        expected: &AppState,
        change: MembershipChange<'_>,
    ) -> Result<bool, MembershipError> {
        set_member(&self.transactions, &mut self.guard, &self.tenant, expected, change, || true)
    }
}

fn new_default(provider_id: &str) -> MembershipChange<'_> {
    MembershipChange::SetDefault {
        provider_id,
        saved: Some(SavedMember { name: "New Relay", icon: None, official_service: None }),
    }
}

#[test]
fn aggregation_t07_default_provider_preserves_contract() {
    let mut f = Fixture::new(AppId::Claude);
    let old = f.seed(false);
    assert_eq!(observe(&old, &f.tenant).unwrap().in_use_provider.as_deref(), Some("old"));

    assert_eq!(f.change(&old, new_default("new")), Ok(true));
    let switched = f.state();
    let status = observe(&switched, &f.tenant).unwrap();
    assert_eq!(status.in_use_provider.as_deref(), Some("new"));
    assert_eq!(status.proxy_route.as_deref(), Some("new"));
    assert_eq!(status.direct_provider.as_deref(), Some("direct"));
    assert!(!status.attached);
    assert_eq!(switched.stack.members, ["old", "peer", "new"]);
    assert_eq!(switched.stack.keys.get("new-relay").map(String::as_str), Some("new"));
    assert_eq!(f.change(&switched, new_default("new")), Ok(false));

    let remove_old = MembershipChange::Remove { provider_id: "old" };
    assert_eq!(f.change(&switched, remove_old), Ok(true));
    let removed = f.state();
    assert_eq!(removed.stack.members, ["peer", "new"]);
    assert_eq!(removed.stack.keys, switched.stack.keys);
    assert_eq!(observe(&removed, &f.tenant).unwrap().proxy_route.as_deref(), Some("new"));
}

#[test]
fn aggregation_t07_remembers_a_default_without_enabling_aggregation() {
    let mut f = Fixture::new(AppId::Claude);
    let empty = f.state();
    assert_eq!(f.change(&empty, new_default("new")), Ok(true));
    let planned = f.state();
    let status = observe(&planned, &f.tenant).unwrap();
    assert_eq!(status.proxy_route.as_deref(), Some("new"));
    assert!(status.in_use_provider.is_none());
    assert!(!status.stack.enabled);
    assert!(!status.attached);
    assert_eq!(planned.stack.members, ["new"]);
}

#[test]
fn aggregation_t07_rejects_a_late_revision_without_changing_either_field() {
    let mut f = Fixture::new(AppId::Claude);
    let old = f.seed(false);
    let checks = Cell::new(0);
    assert_eq!(
        set_member(&f.transactions, &mut f.guard, &f.tenant, &old, new_default("new"), || {
            checks.set(checks.get() + 1);
            checks.get() == 1
        }),
        Err(MembershipError::RevisionConflict)
    );
    assert_eq!(checks.get(), 2);
    assert!(f.state() == old);
    assert!(f.transactions.pending(AppId::Claude).unwrap().is_none());
}

#[test]
fn aggregation_t07_rejects_changes_without_publication_or_account_verification() {
    let mut f = Fixture::new(AppId::Claude);
    let attached = f.seed(true);
    assert_eq!(f.change(&attached, new_default("new")), Err(MembershipError::UnsupportedTarget));
    let foreign = TenantScope::new("platform", "other").unwrap();
    assert_eq!(
        set_member(&f.transactions, &mut f.guard, &foreign, &attached, new_default("new"), || true),
        Err(MembershipError::Forbidden)
    );
    assert!(f.state() == attached);

    let mut f = Fixture::new(AppId::Codex);
    let old = f.seed(false);
    let official = MembershipChange::SetDefault {
        provider_id: "official",
        saved: Some(SavedMember {
            name: "Official",
            icon: None,
            official_service: Some(AuthService::CodexOauth),
        }),
    };
    assert_eq!(f.change(&old, official), Err(MembershipError::UnsupportedTarget));
    assert!(f.state() == old);
}

#[test]
fn aggregation_t07_preserves_the_previous_default_when_the_target_write_fails() {
    let mut f = Fixture::new(AppId::Claude);
    let old = f.seed(false);
    let missing = MembershipChange::SetDefault { provider_id: "missing", saved: None };
    assert_eq!(f.change(&old, missing), Err(MembershipError::NotFound));

    // Make the real transaction's resource registration unwritable.
    let resource = f.root.path().join("provider-live/Zml4dHVyZS1ob3N0/claude.resource.json");
    fs::remove_file(&resource).unwrap();
    fs::create_dir(resource).unwrap();
    assert_eq!(f.change(&old, new_default("new")), Err(MembershipError::TargetFailed));
    assert!(f.state() == old);
    assert!(f.transactions.pending(AppId::Claude).unwrap().is_none());
}
