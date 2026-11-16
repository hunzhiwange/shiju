use agent_app_services::auth_accounts::*;
use agent_database::{GlobalDatabase, test_support::PostgresTestDatabase};
use vw_api_transport::{TimestampMs, id::TenantScope};
use vw_api_app_auth::{AuthErrorCode, AuthService};
use vw_agent_apps::auth::credential_file::{AccountCredential, CredentialFileStore};
fn account_scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "principal",
        host: "host",
        service: AuthService::ClaudeOfficial,
    }
}

fn private_credentials_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    root
}

fn insert_default_fixture(store: &AuthAccountStore, scope: AuthAccountScope<'_>, id: &str, root: &std::path::Path) {
    let metadata = AuthAccountMetadata::new(id, TimestampMs(1)).unwrap();
    store.insert_metadata(&scope, id, &metadata).unwrap();
    CredentialFileStore::new(root).unwrap().lease_copilot(scope, id, true).unwrap()
        .save(&AccountCredential::CopilotToken("synthetic-token".into())).unwrap();
}
#[cfg(target_os = "macos")]
#[test]
fn auth6_remove_store_sqlite() {
    let home = tempfile::tempdir().unwrap();
    assert_scoped_account_removal(|| GlobalDatabase::for_home(home.path()).unwrap());
}

#[cfg(target_os = "macos")]
#[test]
fn auth6_remove_store_postgres() {
    let fixture = PostgresTestDatabase::from_environment().unwrap();
    assert_scoped_account_removal(|| fixture.reopen().unwrap());
}

#[cfg(target_os = "macos")]
fn assert_scoped_account_removal(open: impl Fn() -> GlobalDatabase) {
    use crate::AppProviderStore;

    let credentials = private_credentials_root();
    let root = credentials.path();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope { service: AuthService::GithubCopilot, ..account_scope(&tenant) };
    let neighbor = AuthAccountScope { principal: "other", ..scope };
    let accounts = AuthAccountStore::new(open(), root).unwrap();
    let providers = AppProviderStore::new(open()).unwrap();
    for (scope, id) in [(scope, "selected"), (scope, "spare"), (neighbor, "selected")] {
        insert_default_fixture(&accounts, scope, id, root);
    }
    for scope in [scope, neighbor] {
        accounts.set_managed_default_account(scope, "selected").unwrap();
    }

    assert_eq!(providers.remove_auth_account(scope, root, "spare", None), Ok(()));
    assert_eq!(accounts.managed_default_account(scope), Ok(Some("selected".into())));
    assert_eq!(providers.remove_auth_account(scope, root, "selected", None), Ok(()));
    assert_eq!(
        providers.remove_auth_account(scope, root, "selected", None),
        Err(AuthErrorCode::NotFound)
    );
    let reopened = AuthAccountStore::new(open(), root).unwrap();
    assert_eq!(reopened.read_metadata(&scope, "selected"), Err(AuthErrorCode::NotFound));
    assert_eq!(reopened.managed_default_account(scope), Ok(None));
    assert_eq!(reopened.managed_default_account(neighbor), Ok(Some("selected".into())));
    assert!(reopened.read_metadata(&neighbor, "selected").is_ok());
    assert!(
        CredentialFileStore::new(root).unwrap().lease_copilot(neighbor, "selected", false).unwrap().read().is_ok()
    );
    for removed in ["spare", "selected"] {
        assert_eq!(
            CredentialFileStore::new(root).unwrap()
                .lease_copilot(scope, removed, false)
                .unwrap()
                .read()
                .err()
                .as_deref(),
            Some("official account not found")
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn auth6_remove_store_rolls_back_on_sqlite_and_file_errors() {
    use crate::AppProviderStore;
    let home = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::for_home(home.path()).unwrap();
    let credentials = private_credentials_root();
    let root = credentials.path();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope { service: AuthService::GithubCopilot, ..account_scope(&tenant) };
    let accounts = AuthAccountStore::new(database.clone(), root).unwrap();
    let providers = AppProviderStore::new(database.clone()).unwrap();
    let id = "selected";
    insert_default_fixture(&accounts, scope, id, root);
    accounts.set_managed_default_account(scope, id).unwrap();
    let connection = database.connect_sqlite().unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_fixture_default BEFORE DELETE ON gateway_settings
         WHEN OLD.namespace = 'agent_apps.auth_defaults.v2'
         BEGIN SELECT RAISE(ABORT, 'fixture default delete failure'); END;",
        )
        .unwrap();
    let read_secret = || CredentialFileStore::new(root).unwrap().lease_copilot(scope, id, false).unwrap().read();
    assert_eq!(
        providers.remove_auth_account(scope, root, id, None),
        Err(AuthErrorCode::StorageUnavailable)
    );
    assert!(read_secret().is_ok());
    connection.execute_batch("DROP TRIGGER reject_fixture_default").unwrap();

    let lease = CredentialFileStore::new(root).unwrap().lease_copilot(scope, id, false).unwrap();
    let original = root.join(".managed-v1");
    let moved = root.join("moved");
    std::fs::rename(&original, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &original).unwrap();
    assert_eq!(
        accounts.remove_managed_file_account(scope, id, &lease),
        Err(AuthErrorCode::StorageUnavailable)
    );
    assert_eq!(accounts.managed_default_account(scope), Ok(Some(id.into())));
    assert!(accounts.read_metadata(&scope, id).is_ok());
    std::fs::remove_file(&original).unwrap();
    std::fs::rename(&moved, &original).unwrap();
    drop(lease);
    assert!(read_secret().is_ok());
    assert_eq!(providers.remove_auth_account(scope, root, id, None), Ok(()));
}
