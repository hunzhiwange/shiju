use super::{
    accounts::set_managed_default,
    agy_account_tests::{Fixture, credential, encode, scope as agy_scope},
    credential_file::{AccountCredential, CredentialFileStore, XaiCredential},
    flow::AuthAccountScope,
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
    id::TenantScope,
};

#[test]
fn auth6_default_commits_expected_scoped_result() {
    let root = tempfile::Builder::new()
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .tempdir()
        .unwrap();
    let credentials = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "first-principal",
        host: "host",
        service: AuthService::GithubCopilot,
    };
    let neighbor = AuthAccountScope { principal: "second-principal", ..scope };
    for (owner, id) in [(scope, "old"), (scope, "new"), (neighbor, "old")] {
        credentials
            .lease_copilot(owner, id, true)
            .unwrap()
            .save(&AccountCredential::CopilotToken("synthetic-token".into()))
            .unwrap();
    }
    let mut defaults = std::collections::BTreeMap::new();
    for (owner, id) in [(scope, "old"), (neighbor, "old"), (scope, "new")] {
        assert_eq!(
            set_managed_default(&credentials, owner, id, |key| {
                let lease_error = credentials.lease_copilot(owner, id, false).err().unwrap();
                assert_eq!(lease_error, "official account is in use");
                defaults.insert(key.to_owned(), id);
                Ok(())
            }),
            Ok(())
        );
    }
    assert_eq!(defaults.len(), 2);
    assert!(defaults.values().any(|id| *id == "old"));
    assert!(defaults.values().any(|id| *id == "new"));

    let mut fixture = Fixture::new();
    let previous = credential("first", 1);
    let next = credential("second", 2);
    let first = fixture.managed("first", &previous, "ready");
    let second = fixture.managed("second", &next, "ready");
    assert_eq!(fixture.switch(&first), Ok(()));
    assert_eq!(fixture.switch(&second), Ok(()));
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&next).into_bytes()));
    fixture.remove_entries();
}

#[test]
fn auth6_default_rejects_invalid_or_stale_input() {
    let root = tempfile::Builder::new()
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .tempdir()
        .unwrap();
    let credentials = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "principal",
        host: "host",
        service: AuthService::XaiOauth,
    };
    let rejected = "needs-reauth";
    let lease = credentials.lease_xai(scope, rejected, true).unwrap();
    lease
        .save(&AccountCredential::XaiOauth(XaiCredential {
            access_token: "synthetic-access".into(),
            refresh_token: "synthetic-refresh".into(),
            expires_at: TimestampMs(1),
            requires_reauth: true,
        }))
        .unwrap();
    let publish: fn(&str) -> Result<(), AuthErrorCode> =
        |_| panic!("failed selection must never publish");
    assert_eq!(
        set_managed_default(&credentials, scope, "../invalid", publish),
        Err(AuthErrorCode::InvalidInput)
    );
    assert_eq!(
        set_managed_default(&credentials, scope, rejected, publish),
        Err(AuthErrorCode::AccountInUse)
    );
    drop(lease);
    assert_eq!(
        set_managed_default(&credentials, scope, rejected, publish),
        Err(AuthErrorCode::ReauthRequired)
    );
    for service in [AuthService::ClaudeOfficial, AuthService::DshAccount] {
        assert_eq!(
            set_managed_default(
                &credentials,
                AuthAccountScope { service, ..scope },
                "valid",
                publish
            ),
            Err(AuthErrorCode::Unsupported)
        );
    }

    let mut fixture = Fixture::new();
    let previous = credential("first", 1);
    let mut oversized = credential("second", 2);
    oversized["unknown"]["payload"] = serde_json::json!("x".repeat(4096));
    let first = fixture.managed("first", &previous, "ready");
    let second = fixture.managed("second", &oversized, "ready");
    assert_eq!(fixture.switch(&first), Ok(()));
    let native_scope = agy_scope(&fixture.tenant);
    assert_eq!(
        super::agy_account::switch_account_in(
            native_scope,
            &second,
            fixture.target(None),
            &fixture.entry()
        ),
        Err(AuthErrorCode::Forbidden)
    );
    assert_eq!(fixture.switch(&second), Err(AuthErrorCode::KeychainUnavailable));
    assert_eq!(fixture.entry().read().unwrap(), Some(encode(&previous).into_bytes()));
    fixture.remove_entries();
}
