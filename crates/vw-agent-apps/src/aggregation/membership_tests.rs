use super::{
    member_validation::MemberValidationError,
    membership::{MembershipChange, MembershipError, SavedMember, set_member},
};
use crate::{
    models::edits,
    providers::live::{
        commit::{AppState, ExecutionHostId, Mode, ModeState, PendingTarget, ProviderTransactions},
        write_guard::{AppWriteGuard, FilePrivacy, lock_app},
    },
};
use serde_json::json;
use std::{
    cell::Cell,
    ffi::OsStr,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
};
use tempfile::TempDir;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    id::TenantScope,
    model_catalog::{ModelCatalogEdit, ModelCatalogEditRequest, ModelCatalogIdentity},
};

struct Fixture {
    root: TempDir,
    live: TempDir,
    host: ExecutionHostId,
    tenant: TenantScope,
    transactions: ProviderTransactions,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let host = ExecutionHostId::new("fixture-host").unwrap();
        let transactions = ProviderTransactions::new(root.path(), host.clone()).unwrap();
        Self {
            root,
            live: tempfile::tempdir().unwrap(),
            host,
            tenant: TenantScope::new("platform", "company").unwrap(),
            transactions,
        }
    }

    fn state(&self) -> AppState {
        ProviderTransactions::open_existing(self.root.path(), self.host.clone())
            .unwrap()
            .unwrap()
            .state(AppId::Claude)
            .unwrap()
    }

    fn state_inode(&self) -> u64 {
        fs::metadata(self.root.path().join("provider-live/Zml4dHVyZS1ob3N0/claude.state.json"))
            .unwrap()
            .ino()
    }

    fn set(
        &self,
        guard: &mut AppWriteGuard,
        expected: &AppState,
        change: MembershipChange<'_>,
    ) -> Result<bool, MembershipError> {
        set_member(&self.transactions, guard, &self.tenant, expected, change, || true)
    }
}

fn add(provider_id: &str) -> MembershipChange<'_> {
    MembershipChange::Add {
        provider_id,
        saved: Some(SavedMember { name: "Relay", icon: None, official_service: None }),
    }
}

#[test]
fn aggregation_t06_membership_preserves_contract() {
    let f = Fixture::new();
    let mut guard = lock_app(AppId::Claude, f.live.path()).unwrap();
    let deployment = ModelCatalogIdentity {
        target_host_id: "fixture-host".into(),
        app_id: AppId::Claude,
        provider_id: "first".into(),
        endpoint: "https://fixture.test".into(),
    };
    let provider = json!({"name": "Relay"});
    let expected_revision = edits::read(&f.tenant, &deployment, &provider).unwrap().revision;
    edits::edit(
        &f.tenant,
        &provider,
        &ModelCatalogEditRequest {
            deployment: deployment.clone(),
            expected_revision,
            edits: vec![ModelCatalogEdit::Upsert {
                model_id: "model".into(),
                display_alias: None,
                metadata: Box::default(),
            }],
            default_change: None,
        },
        |updated| {
            guard
                .write(
                    OsStr::new("provider.json"),
                    &serde_json::to_vec(updated).unwrap(),
                    FilePrivacy::Private,
                )
                .unwrap();
            Ok(true)
        },
    )
    .unwrap();
    let saved_bytes = fs::read(f.live.path().join("provider.json")).unwrap();
    let saved = serde_json::from_slice(&saved_bytes).unwrap();
    assert_eq!(edits::read(&f.tenant, &deployment, &saved).unwrap().items.len(), 1);
    let empty = f.state();
    assert!(empty.stack.members.is_empty());

    assert_eq!(f.set(&mut guard, &empty, add("first")), Ok(true));
    let added = f.state();
    assert_eq!(added.stack.members, ["first"]);
    assert_eq!(added.stack.keys.get("relay").map(String::as_str), Some("first"));
    let inode = f.state_inode();
    assert_eq!(f.set(&mut guard, &added, add("first")), Ok(false));
    assert_eq!(f.state_inode(), inode);
    assert_eq!(f.set(&mut guard, &empty, add("second")), Err(MembershipError::RevisionConflict));
    assert!(f.state() == added);

    let remove = || MembershipChange::Remove { provider_id: "first" };
    assert_eq!(f.set(&mut guard, &added, remove()), Ok(true));
    let removed = f.state();
    assert!(removed.stack.members.is_empty());
    assert_eq!(removed.stack.keys, added.stack.keys);
    let inode = f.state_inode();
    assert_eq!(f.set(&mut guard, &removed, remove()), Ok(false));
    assert_eq!(f.state_inode(), inode);
    assert_eq!(f.set(&mut guard, &removed, add("first")), Ok(true));
    assert_eq!(f.state().stack, added.stack);
    assert_eq!(fs::read(f.live.path().join("provider.json")).unwrap(), saved_bytes);
}

#[test]
fn aggregation_t06_rechecks_external_revision_before_settlement() {
    let f = Fixture::new();
    let mut guard = lock_app(AppId::Claude, f.live.path()).unwrap();
    let baseline = f.state();
    let checks = Cell::new(0);
    assert_eq!(
        set_member(&f.transactions, &mut guard, &f.tenant, &baseline, add("first"), || {
            checks.set(checks.get() + 1);
            checks.get() == 1
        }),
        Err(MembershipError::RevisionConflict)
    );
    assert_eq!(checks.get(), 2);
    assert!(f.state() == baseline);
    assert!(f.transactions.pending(AppId::Claude).unwrap().is_none());
}

#[test]
fn aggregation_t06_rejects_unauthorized_and_unpublishable_edits() {
    let f = Fixture::new();
    let mut guard = lock_app(AppId::Claude, f.live.path()).unwrap();
    let empty = f.state();
    let invalid = Err(MembershipError::Validation(MemberValidationError::InvalidMember));
    assert_eq!(
        f.set(&mut guard, &empty, MembershipChange::Add { provider_id: "missing", saved: None }),
        invalid
    );
    let official = MembershipChange::Add {
        provider_id: "official",
        saved: Some(SavedMember {
            name: "Official",
            icon: None,
            official_service: Some(AuthService::ClaudeOfficial),
        }),
    };
    assert_eq!(f.set(&mut guard, &empty, official), invalid);

    assert_eq!(f.set(&mut guard, &empty, add("first")), Ok(true));
    let mut stack = f.state().stack;
    stack.enabled = true;
    f.transactions
        .commit(
            &f.tenant,
            &mut guard,
            &[],
            PendingTarget {
                stack: Some(stack),
                mode: Some(ModeState {
                    mode: Mode::Proxy,
                    attached: true,
                    proxy_route: Some("first".into()),
                    contract: None,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let attached = f.state();
    assert_eq!(
        f.set(&mut guard, &attached, MembershipChange::Remove { provider_id: "first" }),
        Err(MembershipError::Validation(MemberValidationError::DefaultMemberRequired))
    );
    assert_eq!(
        f.set(&mut guard, &attached, add("second")),
        Err(MembershipError::UnsupportedTarget)
    );
    let foreign = TenantScope::new("platform", "other").unwrap();
    assert_eq!(
        set_member(&f.transactions, &mut guard, &foreign, &attached, add("second"), || true),
        Err(MembershipError::Forbidden)
    );
    assert!(f.state() == attached);
}
