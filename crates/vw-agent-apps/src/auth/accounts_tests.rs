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

#[test]
fn auth6_remove_commits_expected_scoped_result() {
    let root = tempfile::Builder::new()
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .tempdir()
        .unwrap();
    let credentials = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "host",
        service: AuthService::GithubCopilot,
    };
    let other = AuthAccountScope { principal: "other", ..scope };
    let lease = credentials.lease_copilot(scope, "shared-id", true).unwrap();
    let neighbor = credentials.lease_copilot(other, "shared-id", true).unwrap();
    for entry in [&lease, &neighbor] {
        entry.save(&AccountCredential::CopilotToken("fixture".into())).unwrap();
    }

    assert_eq!(super::accounts::remove_managed_oauth_file(&lease), Ok(()));
    assert_eq!(lease.read().err().as_deref(), Some("official account not found"));
    assert!(neighbor.read().is_ok());
    assert_eq!(super::accounts::remove_managed_oauth_file(&lease), Ok(()));
}

#[test]
fn auth6_remove_rejects_invalid_or_stale_input() {
    let root = tempfile::Builder::new()
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .tempdir()
        .unwrap();
    let credentials = CredentialFileStore::new(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "host",
        service: AuthService::GithubCopilot,
    };
    let lease = credentials.lease_copilot(scope, "fixture", true).unwrap();
    lease.save(&AccountCredential::CopilotToken("fixture".into())).unwrap();
    let original = root.path().join(".managed-v1");
    let moved = root.path().join("moved");
    std::fs::rename(&original, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &original).unwrap();

    assert_eq!(
        super::accounts::remove_managed_oauth_file(&lease),
        Err(AuthErrorCode::StorageUnavailable)
    );
    std::fs::remove_file(&original).unwrap();
    std::fs::rename(&moved, &original).unwrap();
    assert!(lease.read().is_ok());
}

#[test]
fn auth6_remove_claude_erases_only_verified_scoped_material() {
    use super::claude_keychain::{ClaudeKeychain, MacOsKeychain};
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::fs;
    let root = tempfile::Builder::new()
        .permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .tempdir()
        .unwrap();
    let credentials = CredentialFileStore::new(root.path()).unwrap().for_owner("owner").unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    credentials.register_claude_directory(&tenant, "host", "selected").unwrap();
    let lease = credentials
        .lease(&tenant, "host", vw_api_types::agent_apps::AppId::Claude, "selected", false)
        .unwrap();
    let AccountCredential::ClaudeDirectory(directory) = lease.read().unwrap() else {
        panic!("expected scoped Claude directory");
    };
    let path = directory.path();
    fs::write(
        path.join(".claude.json"),
        json!({
            "oauthAccount": {"accountUuid": "fixture", "organizationUuid": "org"}
        })
        .to_string(),
    )
    .unwrap();
    fs::create_dir(path.join("nested")).unwrap();
    fs::write(path.join("nested/private"), b"fixture").unwrap();
    let outside = root.path().join("outside");
    fs::write(&outside, b"preserve").unwrap();
    std::os::unix::fs::symlink(&outside, path.join("outside-link")).unwrap();

    struct KeychainCleanup<'a>(&'a std::path::Path);
    impl Drop for KeychainCleanup<'_> {
        fn drop(&mut self) {
            let _ = MacOsKeychain.write(Some(self.0), None);
        }
    }
    let _cleanup = KeychainCleanup(path);
    let secret = json!({"claudeAiOauth": {"refreshToken": "fixture"}});
    MacOsKeychain.write(Some(path), Some(&secret)).unwrap();
    assert_eq!(
        super::accounts::remove_managed_claude(&lease, [0; 32]),
        Err(AuthErrorCode::IdentityMismatch)
    );
    assert_eq!(MacOsKeychain.read(Some(path)).unwrap(), Some(secret));
    assert!(path.join("nested/private").exists());
    let identity = Sha256::digest(b"fixture:org").into();
    assert_eq!(super::accounts::remove_managed_claude(&lease, identity), Ok(()));
    assert_eq!(MacOsKeychain.read(Some(path)).unwrap(), None);
    assert!(!path.exists());
    assert_eq!(fs::read(outside).unwrap(), b"preserve");
    assert!(lease.read().is_err());
    assert_eq!(super::accounts::remove_managed_claude(&lease, identity), Ok(()));
}

#[test]
fn auth6_remove_agy_preserves_live_and_referenced_accounts() {
    use super::agy_account::remove_managed_account_in;
    let mut fixture = Fixture::new();
    let active_credential = credential("live", 1);
    let inactive_credential = credential("inactive", 2);
    let active = fixture.managed("live", &active_credential, "ready");
    let inactive = fixture.managed("inactive", &inactive_credential, "reauth_required");
    fixture.switch(&active).unwrap();
    let scope = agy_scope(&fixture.tenant);
    let native = fixture.target(Some(scope));
    let entry = fixture.entry();

    assert_eq!(
        remove_managed_account_in(scope, &active, native, &entry, || panic!("live slot is in use")),
        Err(AuthErrorCode::AccountInUse)
    );
    assert_eq!(
        remove_managed_account_in(scope, &inactive, native, &entry, || Err(
            AuthErrorCode::AccountInUse
        )),
        Err(AuthErrorCode::AccountInUse)
    );
    assert_eq!(fixture.record(&inactive)["credential"], inactive_credential);
    assert_eq!(remove_managed_account_in(scope, &inactive, native, &entry, || Ok(())), Ok(()));
    assert_eq!(
        remove_managed_account_in(scope, &inactive, native, &entry, || Ok(())),
        Err(AuthErrorCode::NotFound)
    );
    assert_eq!(entry.read().unwrap(), Some(encode(&active_credential).into_bytes()));
    assert_eq!(fixture.record(&active)["credential"], active_credential);
    fixture.remove_entries();
}
