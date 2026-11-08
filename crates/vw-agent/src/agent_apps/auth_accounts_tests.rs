use super::*;
use crate::global_database::test_support::PostgresTestDatabase;
use vw_api_types::agent_apps::*;
use vw_api_types::agent_apps_auth::AuthService;

const ACCOUNT_ID: &str = "managed-account";
const EXTERNAL_IDENTITY: &str = "fixture-external-identity";

#[test]
fn auth6_store_commits_expected_scoped_result() {
    let home = tempfile::tempdir().unwrap();
    assert_scoped_metadata_persists(|| GlobalDatabase::for_home(home.path()).unwrap());
}

#[test]
fn auth6_store_commits_expected_scoped_result_postgres() {
    let fixture = PostgresTestDatabase::from_environment().unwrap();
    assert_scoped_metadata_persists(|| fixture.reopen().unwrap());
}

#[test]
fn auth6_store_rejects_invalid_or_stale_input() {
    let home = tempfile::tempdir().unwrap();
    assert_invalid_or_stale_metadata_rejected(GlobalDatabase::for_home(home.path()).unwrap());
}

#[test]
fn auth6_store_rejects_invalid_or_stale_input_postgres() {
    let fixture = PostgresTestDatabase::from_environment().unwrap();
    assert_invalid_or_stale_metadata_rejected(fixture.database().clone());
}

fn assert_scoped_metadata_persists(open: impl Fn() -> GlobalDatabase) {
    let credentials = private_credentials_root();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let other_company = TenantScope::new("platform", "other-company").unwrap();
    let other_platform = TenantScope::new("other-platform", "company").unwrap();
    let scopes = [
        account_scope(&tenant),
        AuthAccountScope { tenant: &other_company, ..account_scope(&tenant) },
        AuthAccountScope { host: "other-host", ..account_scope(&tenant) },
        AuthAccountScope { principal: "oidc:other-principal", ..account_scope(&tenant) },
        AuthAccountScope { service: AuthService::CodexOauth, ..account_scope(&tenant) },
        AuthAccountScope { tenant: &other_platform, ..account_scope(&tenant) },
    ];
    let store = AuthAccountStore::new(open(), credentials.path()).unwrap();
    let expected: Vec<_> = (1..=6)
        .map(|time| AuthAccountMetadata::new(EXTERNAL_IDENTITY, TimestampMs(time)).unwrap())
        .collect();
    for (scope, metadata) in scopes.iter().zip(&expected) {
        assert_eq!(store.insert_metadata(scope, ACCOUNT_ID, metadata), Ok(()));
    }
    drop(store);

    let store = AuthAccountStore::new(open(), credentials.path()).unwrap();
    for (scope, metadata) in scopes.iter().zip(expected) {
        assert_eq!(store.read_metadata(scope, ACCOUNT_ID), Ok(metadata));
    }
    for tenant in [&tenant, &other_company, &other_platform] {
        for entry in store.metadata.list(tenant, MANAGED_ACCOUNT_NAMESPACE).unwrap() {
            assert!(!entry.payload_json.contains(EXTERNAL_IDENTITY));
        }
    }
    assert!(std::fs::read_dir(credentials.path()).unwrap().next().is_none());
}

fn assert_invalid_or_stale_metadata_rejected(database: GlobalDatabase) {
    let credentials = private_credentials_root();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = account_scope(&tenant);
    let store = AuthAccountStore::new(database, credentials.path()).unwrap();
    let original = AuthAccountMetadata::new(EXTERNAL_IDENTITY, TimestampMs(1)).unwrap();
    assert_eq!(
        store.insert_metadata(&scope, "../invalid", &original),
        Err(AuthErrorCode::InvalidInput)
    );
    assert_eq!(
        store.insert_metadata(&AuthAccountScope { principal: "", ..scope }, ACCOUNT_ID, &original),
        Err(AuthErrorCode::InvalidInput)
    );

    let fixture_token = "fixture-private-legacy-token";
    store
        .save(
            &tenant,
            scope.host,
            AppId::Claude,
            ACCOUNT_ID,
            &AccountCredential::ClaudeToken(fixture_token.into()),
        )
        .unwrap();
    assert_eq!(store.read_metadata(&scope, ACCOUNT_ID), Err(AuthErrorCode::NotFound));
    store.insert_metadata(&scope, ACCOUNT_ID, &original).unwrap();
    let stale = AuthAccountMetadata::new("different-external-identity", TimestampMs(2)).unwrap();
    assert_eq!(store.insert_metadata(&scope, ACCOUNT_ID, &stale), Err(AuthErrorCode::Conflict));
    assert_eq!(store.read_metadata(&scope, ACCOUNT_ID), Ok(original));
    assert_eq!(store.read_metadata(&scope, "other-account"), Err(AuthErrorCode::NotFound));
    assert_eq!(store.list(&tenant, scope.host, AppId::Claude).unwrap(), [ACCOUNT_ID]);
    let managed = store.metadata.list(&tenant, MANAGED_ACCOUNT_NAMESPACE).unwrap();
    assert_eq!(managed.len(), 1);
    assert!(!managed[0].payload_json.contains(fixture_token));
}

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

#[test]
fn cc_apps_account_launch_isolation_database_owner_reopen_and_cross_scope_refusal() {
    let root = tempfile::tempdir().unwrap();
    let credentials = root.path().join("credentials");
    std::fs::create_dir(&credentials).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let database = GlobalDatabase::for_home(root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let store = AuthAccountStore::new(database.clone(), &credentials).unwrap();
    store
        .save(
            &tenant,
            "host",
            AppId::Claude,
            "selected",
            &AccountCredential::ClaudeToken("fixture-selected".into()),
        )
        .unwrap();
    drop(store);
    let store = AuthAccountStore::new(database, &credentials).unwrap();
    let program = root.path().join("claude");
    std::fs::write(&program, "#!/bin/sh\ntest \"$CLAUDE_CODE_OAUTH_TOKEN\" = fixture-selected || exit 2\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > private-path\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let request = LaunchRequest {
        app_id: AppId::Claude,
        installation_id: AppInstallationId("observed".into()),
        cwd: root.path().to_str().unwrap().into(),
        args: vec![],
        skip_permissions: false,
    };
    let installation = AppInstallation {
        id: request.installation_id.clone(),
        app_id: AppId::Claude,
        target_host_id: "host".into(),
        environment: if cfg!(target_os = "macos") {
            AppInstallationEnvironment::Macos
        } else if cfg!(windows) {
            AppInstallationEnvironment::Windows
        } else {
            AppInstallationEnvironment::Linux
        },
        path: program.to_str().unwrap().into(),
        real_path: None,
        source: AppInstallationSource::Native,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    for other in [
        TenantScope::new("platform", "other").unwrap(),
        TenantScope::new("other", "company").unwrap(),
    ] {
        assert!(
            store
                .prepare_launch(
                    &other,
                    "host",
                    "selected",
                    &request,
                    std::slice::from_ref(&installation),
                    &[root.path().into()],
                    root.path()
                )
                .is_err()
        );
    }
    assert!(
        store
            .prepare_launch(
                &tenant,
                "other-host",
                "selected",
                &request,
                std::slice::from_ref(&installation),
                &[root.path().into()],
                root.path()
            )
            .is_err()
    );
    assert!(
        store
            .prepare_launch(
                &tenant,
                "host",
                "missing",
                &request,
                std::slice::from_ref(&installation),
                &[root.path().into()],
                root.path()
            )
            .is_err()
    );
    assert!(!root.path().join("private-path").exists());
    store
        .prepare_launch(
            &tenant,
            "host",
            "selected",
            &request,
            &[installation],
            &[root.path().into()],
            root.path(),
        )
        .unwrap()
        .run()
        .unwrap();
    let private = std::fs::read_to_string(root.path().join("private-path")).unwrap();
    assert!(!Path::new(&private).exists());
}

fn host_fixture(
    app: AppId,
    body: &str,
) -> (tempfile::TempDir, AuthAccountStore, TenantScope, LaunchRequest, AppInstallation) {
    let root = tempfile::tempdir().unwrap();
    let credentials = root.path().join("credentials");
    std::fs::create_dir(&credentials).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let store = AuthAccountStore::new(GlobalDatabase::for_home(root.path()).unwrap(), &credentials)
        .unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let program = root.path().join(if app == AppId::Codex { "codex" } else { "claude" });
    std::fs::write(&program, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let request = LaunchRequest {
        app_id: app,
        installation_id: AppInstallationId("observed".into()),
        cwd: root.path().to_str().unwrap().into(),
        args: vec![],
        skip_permissions: false,
    };
    let installation = AppInstallation {
        id: request.installation_id.clone(),
        app_id: app,
        target_host_id: "host".into(),
        environment: if cfg!(target_os = "macos") {
            AppInstallationEnvironment::Macos
        } else if cfg!(windows) {
            AppInstallationEnvironment::Windows
        } else {
            AppInstallationEnvironment::Linux
        },
        path: program.to_str().unwrap().into(),
        real_path: None,
        source: AppInstallationSource::Native,
        is_path_default: true,
        version: None,
        runtime_status: AppInstallationRuntimeStatus::Runnable,
    };
    (root, store, tenant, request, installation)
}
fn codex_auth() -> AccountCredential {
    AccountCredential::CodexAuth(
        serde_json::json!({"tokens":{"access_token":"fixture-access", "refresh_token":"fixture-refresh", "id_token":"fixture-id", "account_id":"fixture-account"}}),
    )
}

#[test]
fn cc_apps_account_launch_isolation_owner_persists_rotation_and_holds_lease_until_reaped() {
    let (root, store, tenant, request, installation) = host_fixture(
        AppId::Codex,
        "#!/bin/sh\ncat > \"$CODEX_HOME/auth.json\" <<'AUTH'\n{\"tokens\":{\"access_token\":\"fixture-access-new\",\"refresh_token\":\"fixture-refresh-new\",\"id_token\":\"fixture-id\",\"account_id\":\"fixture-account\"}}\nAUTH\nprintf '%s' \"$CODEX_HOME\" > private-path\nexit 7\n",
    );
    store.save(&tenant, "host", AppId::Codex, "selected", &codex_auth()).unwrap();
    let launch = store
        .prepare_launch(
            &tenant,
            "host",
            "selected",
            &request,
            std::slice::from_ref(&installation),
            &[root.path().into()],
            root.path(),
        )
        .unwrap();
    assert!(store.save(&tenant, "host", AppId::Codex, "selected", &codex_auth()).is_err());
    assert!(
        store
            .prepare_launch(
                &tenant,
                "host",
                "selected",
                &request,
                std::slice::from_ref(&installation),
                &[root.path().into()],
                root.path()
            )
            .is_err()
    );
    assert!(launch.run().is_err());
    let private = std::fs::read_to_string(root.path().join("private-path")).unwrap();
    assert!(!Path::new(&private).exists());
    drop(store);
    let store = AuthAccountStore::new(
        GlobalDatabase::for_home(root.path()).unwrap(),
        &root.path().join("credentials"),
    )
    .unwrap();
    let AccountCredential::CodexAuth(rotated) =
        store.credentials.read(&tenant, "host", AppId::Codex, "selected").unwrap()
    else {
        panic!("wrong service")
    };
    assert_eq!(rotated["tokens"]["refresh_token"], "fixture-refresh-new");
    std::fs::write(&installation.path, "#!/bin/sh\ncp \"$CODEX_HOME/auth.json\" observed-auth\n")
        .unwrap();
    store
        .prepare_launch(
            &tenant,
            "host",
            "selected",
            &request,
            &[installation],
            &[root.path().into()],
            root.path(),
        )
        .unwrap()
        .run()
        .unwrap();
    let actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.path().join("observed-auth")).unwrap()).unwrap();
    assert_eq!(actual["tokens"]["refresh_token"], "fixture-refresh-new");
}

#[test]
fn cc_apps_account_launch_isolation_default_claude_is_scoped_and_uses_owned_directory() {
    let (root, store, tenant, request, installation) = host_fixture(
        AppId::Claude,
        "#!/bin/sh\ntest -z \"$CLAUDE_CODE_OAUTH_TOKEN$ANTHROPIC_API_KEY\" || exit 2\ncat \"$CLAUDE_CONFIG_DIR/fixture-identity\" > observed-subject\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > selected-directory\n",
    );
    assert!(
        store
            .prepare_default_launch(
                &tenant,
                "host",
                &request,
                std::slice::from_ref(&installation),
                &[root.path().into()],
                root.path()
            )
            .is_err()
    );
    assert!(store.set_default(&tenant, "host", AppId::Claude, "missing").is_err());
    let other = TenantScope::new("platform", "other").unwrap();
    for (scope, identity) in [(&tenant, "fixture-company"), (&other, "fixture-other-company")] {
        store.register_claude_directory(scope, "host", "same").unwrap();
        let AccountCredential::ClaudeDirectory(directory) =
            store.credentials.read(scope, "host", AppId::Claude, "same").unwrap()
        else {
            panic!("expected managed directory")
        };
        std::fs::write(directory.path().join("fixture-identity"), identity).unwrap();
        store.set_default(scope, "host", AppId::Claude, "same").unwrap();
    }
    drop(store);
    let store = AuthAccountStore::new(
        GlobalDatabase::for_home(root.path()).unwrap(),
        &root.path().join("credentials"),
    )
    .unwrap();
    assert!(
        store
            .prepare_default_launch(
                &tenant,
                "other-host",
                &request,
                std::slice::from_ref(&installation),
                &[root.path().into()],
                root.path()
            )
            .is_err()
    );
    store
        .prepare_default_launch(
            &tenant,
            "host",
            &request,
            std::slice::from_ref(&installation),
            &[root.path().into()],
            root.path(),
        )
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(root.path().join("observed-subject")).unwrap(),
        "fixture-company"
    );
    let directory = std::fs::read_to_string(root.path().join("selected-directory")).unwrap();
    assert!(Path::new(&directory).is_dir());
    store
        .prepare_default_launch(
            &other,
            "host",
            &request,
            &[installation],
            &[root.path().into()],
            root.path(),
        )
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(root.path().join("observed-subject")).unwrap(),
        "fixture-other-company"
    );
}

#[test]
fn cc_apps_account_launch_isolation_abandoned_preparation_releases_owner_lease() {
    let (root, store, tenant, request, installation) =
        host_fixture(AppId::Codex, "#!/bin/sh\nexit 0\n");
    store.save(&tenant, "host", AppId::Codex, "selected", &codex_auth()).unwrap();
    let launch = store
        .prepare_launch(
            &tenant,
            "host",
            "selected",
            &request,
            std::slice::from_ref(&installation),
            &[root.path().into()],
            root.path(),
        )
        .unwrap();
    drop(launch);
    store.save(&tenant, "host", AppId::Codex, "selected", &codex_auth()).unwrap();
    let launch = store
        .prepare_launch(
            &tenant,
            "host",
            "selected",
            &request,
            &[installation],
            &[root.path().into()],
            root.path(),
        )
        .unwrap();
    std::fs::remove_file(root.path().join("codex")).unwrap();
    assert!(launch.run().is_err());
    store.save(&tenant, "host", AppId::Codex, "selected", &codex_auth()).unwrap();
}

#[cfg(unix)]
#[test]
fn cc_apps_launch_identity_list_official_host_tenant_and_missing_material() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let credentials = root.path().join("credentials");
    std::fs::create_dir(&credentials).unwrap();
    std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o700)).unwrap();
    let store = AuthAccountStore::new(GlobalDatabase::for_home(root.path()).unwrap(), &credentials)
        .unwrap();
    let a = TenantScope::new("p", "a").unwrap();
    let b = TenantScope::new("p", "b").unwrap();
    assert!(store.list(&a, "host", AppId::Claude).unwrap().is_empty());
    store
        .save(
            &a,
            "host",
            AppId::Claude,
            "selected",
            &AccountCredential::ClaudeToken("fixture-private-token".into()),
        )
        .unwrap();
    assert_eq!(store.list(&a, "host", AppId::Claude).unwrap(), ["selected"]);
    assert!(store.list(&b, "host", AppId::Claude).unwrap().is_empty());
    assert!(store.list(&a, "other", AppId::Claude).unwrap().is_empty());
    assert!(store.list(&a, "host", AppId::Codex).unwrap().is_empty());
    assert!(store.list(&a, "host", AppId::Agy).unwrap().is_empty());
    std::fs::remove_dir_all(&credentials).unwrap();
    std::fs::create_dir(&credentials).unwrap();
    std::fs::set_permissions(&credentials, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        store.list(&a, "host", AppId::Claude).unwrap().is_empty(),
        "missing secret material is not advertised"
    );
}

#[cfg(unix)]
fn copilot_account() -> AuthAccount {
    use vw_api_types::agent_apps_auth::AuthCredentialState;

    AuthAccount {
        account_id: ACCOUNT_ID.into(),
        service: AuthService::GithubCopilot,
        label: "octocat".into(),
        credential_state: AuthCredentialState::Ready,
        is_default: false,
        authenticated_at: Some(TimestampMs(1)),
        login_expires_at: None,
        github_domain: Some("github.com".into()),
        region: None,
    }
}

#[cfg(unix)]
fn assert_copilot_commit(open: impl Fn() -> GlobalDatabase) {
    let root = private_credentials_root();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = AuthAccountScope { service: AuthService::GithubCopilot, ..account_scope(&tenant) };
    let account = copilot_account();
    let store = AuthAccountStore::new(open(), root.path()).unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
    runtime
        .block_on(async { store.commit_copilot(owner, &account, "github.com/42", "first-token") })
        .unwrap();
    drop(store);
    let store = AuthAccountStore::new(open(), root.path()).unwrap();
    let saved = store.read_metadata(&owner, ACCOUNT_ID).unwrap();
    let expected = AuthAccountMetadata::new("github.com/42", TimestampMs(1)).unwrap();
    assert_eq!(saved.external_identity_sha256, expected.external_identity_sha256);
    assert_eq!(saved.authenticated_at, expected.authenticated_at);
    assert_eq!(saved.github_domain.as_deref(), Some("github.com"));
    assert_eq!(saved.label.as_deref(), Some("octocat"));
    for unrelated in [
        AuthAccountScope { principal: "other", ..owner },
        AuthAccountScope { host: "other", ..owner },
    ] {
        assert_eq!(store.read_metadata(&unrelated, ACCOUNT_ID), Err(AuthErrorCode::NotFound));
        assert!(store.credentials.lease_copilot(unrelated, ACCOUNT_ID, false).is_err());
    }

    store.commit_copilot(owner, &account, "github.com/42", "new-token").unwrap();
    assert_eq!(
        store.commit_copilot(owner, &account, "other.ghe.com/42", "foreign-token"),
        Err(AuthErrorCode::IdentityMismatch),
    );
    let credential =
        store.credentials.lease_copilot(owner, ACCOUNT_ID, false).unwrap().read().unwrap();
    assert!(matches!(credential, AccountCredential::CopilotToken(token) if token == "new-token"));
    let rows = store.metadata.list(&tenant, MANAGED_ACCOUNT_NAMESPACE).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].payload_json.contains("token"));
    assert!(store.metadata.list(&tenant, "agent_apps.auth_defaults").unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn auth6_copilot_store_commits_scoped_credentials_sqlite() {
    let home = tempfile::tempdir().unwrap();
    assert_copilot_commit(|| GlobalDatabase::for_home(home.path()).unwrap());
}

#[cfg(unix)]
#[test]
fn auth6_copilot_store_commits_scoped_credentials_postgres() {
    let fixture = PostgresTestDatabase::from_environment().unwrap();
    assert_copilot_commit(|| fixture.reopen().unwrap());
    fixture.cleanup().unwrap();
}

#[cfg(unix)]
#[test]
fn auth6_copilot_store_rolls_back_on_metadata_failure() {
    let home = tempfile::tempdir().unwrap();
    let root = private_credentials_root();
    let database = GlobalDatabase::for_home(home.path()).unwrap();
    let store = AuthAccountStore::new(database.clone(), root.path()).unwrap();
    let connection = database.connect_sqlite().unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_auth_insert BEFORE INSERT ON gateway_settings
         BEGIN SELECT RAISE(ABORT, 'fixture metadata write failure'); END;",
        )
        .unwrap();

    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = AuthAccountScope { service: AuthService::GithubCopilot, ..account_scope(&tenant) };
    let account = copilot_account();
    assert_eq!(
        store.commit_copilot(owner, &account, "github.com/42", "fixture-token"),
        Err(AuthErrorCode::StorageUnavailable),
    );
    assert_eq!(store.read_metadata(&owner, ACCOUNT_ID), Err(AuthErrorCode::NotFound));
    let lease = store.credentials.lease_copilot(owner, ACCOUNT_ID, false).unwrap();
    assert!(matches!(lease.read(), Err(error) if error == "official account not found"));
    drop(lease);
    connection.execute_batch("DROP TRIGGER reject_auth_insert;").unwrap();
    store.commit_copilot(owner, &account, "github.com/42", "fixture-token").unwrap();
}

#[cfg(unix)]
fn assert_copilot_session_requires_scoped_metadata(database: GlobalDatabase) {
    let root = private_credentials_root();
    let store = AuthAccountStore::new(database, root.path()).unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let owner = AuthAccountScope { service: AuthService::GithubCopilot, ..account_scope(&tenant) };
    let foreign = AuthAccountScope { principal: "oidc:other", ..owner };
    let adapter = CopilotAuth::new().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let session = |scope, account_id| {
        runtime.block_on(store.copilot_session(&adapter, scope, account_id)).err()
    };

    // A host file alone must never authorize an account, even if its OAuth token is readable.
    store
        .credentials
        .lease_copilot(owner, ACCOUNT_ID, true)
        .unwrap()
        .save(&AccountCredential::CopilotToken("fixture-oauth".to_owned()))
        .unwrap();
    assert_eq!(session(owner, ACCOUNT_ID), Some(AuthErrorCode::NotFound),);
    let metadata = AuthAccountMetadata::new("fixture", TimestampMs(1)).unwrap();
    store.insert_metadata(&owner, ACCOUNT_ID, &metadata).unwrap();
    assert_eq!(session(foreign, ACCOUNT_ID), Some(AuthErrorCode::NotFound),);
    assert_eq!(session(owner, ACCOUNT_ID), Some(AuthErrorCode::StorageUnavailable),);

    let metadata = AuthAccountMetadata {
        github_domain: Some("github.com@foreign.example".to_owned()),
        ..metadata
    };
    store.insert_metadata(&owner, "invalid-domain", &metadata).unwrap();
    assert_eq!(session(owner, "invalid-domain"), Some(AuthErrorCode::InvalidInput),);
}

#[cfg(unix)]
#[test]
fn auth6_copilot_session_scope_sqlite() {
    let home = tempfile::tempdir().unwrap();
    let database = GlobalDatabase::for_home(home.path()).unwrap();
    assert_copilot_session_requires_scoped_metadata(database);
}

#[cfg(unix)]
#[test]
fn auth6_copilot_session_scope_postgres() {
    let fixture = PostgresTestDatabase::from_environment().unwrap();
    assert_copilot_session_requires_scoped_metadata(fixture.reopen().unwrap());
    fixture.cleanup().unwrap();
}

#[cfg(unix)]
fn assert_codex_device_commit(open: impl Fn() -> GlobalDatabase) {
    let root = private_credentials_root();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope { service: AuthService::CodexOauth, ..account_scope(&tenant) };
    let account = AuthAccount {
        service: scope.service,
        label: "ChatGPT".into(),
        github_domain: None,
        ..copilot_account()
    };
    let identity = r#"["fixture-account","fixture-subject"]"#;
    let credential = codex_auth();
    let store = AuthAccountStore::new(open(), root.path()).unwrap();
    store.store_codex_device(scope, &account, identity, &credential).unwrap();
    drop(store);

    let store = AuthAccountStore::new(open(), root.path()).unwrap();
    let mut expected = AuthAccountMetadata::new(identity, TimestampMs(1)).unwrap();
    expected.label = Some("ChatGPT".into());
    assert_eq!(store.read_metadata(&scope, ACCOUNT_ID), Ok(expected));

    let replacement = AccountCredential::CodexAuth(
        serde_json::json!({"tokens":{"access_token":"replacement-access", "refresh_token":"replacement-refresh", "id_token":"replacement-id", "account_id":"fixture-account"}}),
    );
    assert_eq!(
        store.store_codex_device(scope, &account, identity, &replacement),
        Err(AuthErrorCode::CodexOauthDuplicateAccount)
    );
    let owner = store.credentials.for_owner(scope.principal).unwrap();
    let saved = owner.read(&tenant, scope.host, AppId::Codex, ACCOUNT_ID).unwrap();
    assert!(matches!((saved, credential),
        (AccountCredential::CodexAuth(saved), AccountCredential::CodexAuth(expected)) if saved == expected));

    for foreign in [
        AuthAccountScope { principal: "other", ..scope },
        AuthAccountScope { host: "other", ..scope },
    ] {
        assert_eq!(store.read_metadata(&foreign, ACCOUNT_ID), Err(AuthErrorCode::NotFound));
        let foreign_owner = store.credentials.for_owner(foreign.principal).unwrap();
        assert!(foreign_owner.read(&tenant, foreign.host, AppId::Codex, ACCOUNT_ID).is_err());
    }
    let rows = store.metadata.list(&tenant, MANAGED_ACCOUNT_NAMESPACE).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].payload_json.contains("fixture-"));
    assert!(store.metadata.list(&tenant, "agent_apps.auth_defaults").unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn auth6_codex_device_store_sqlite() {
    let home = tempfile::tempdir().unwrap();
    assert_codex_device_commit(|| GlobalDatabase::for_home(home.path()).unwrap());
}

#[cfg(unix)]
#[test]
fn auth6_codex_device_store_postgres() {
    let fixture = PostgresTestDatabase::from_environment().unwrap();
    assert_codex_device_commit(|| fixture.reopen().unwrap());
    fixture.cleanup().unwrap();
}

#[cfg(unix)]
#[test]
fn auth6_codex_device_store_rolls_back_failed_metadata_insert() {
    let home = tempfile::tempdir().unwrap();
    let root = private_credentials_root();
    let database = GlobalDatabase::for_home(home.path()).unwrap();
    let store = AuthAccountStore::new(database.clone(), root.path()).unwrap();
    let connection = database.connect_sqlite().unwrap();
    connection
        .execute_batch(
            "CREATE TRIGGER reject_auth_insert BEFORE INSERT ON gateway_settings
         BEGIN SELECT RAISE(ABORT, 'fixture metadata write failure'); END;",
        )
        .unwrap();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let scope = AuthAccountScope { service: AuthService::CodexOauth, ..account_scope(&tenant) };
    let account = AuthAccount {
        service: scope.service,
        label: "ChatGPT".into(),
        github_domain: None,
        ..copilot_account()
    };
    let credential = codex_auth();
    assert_eq!(
        store.store_codex_device(scope, &account, "fixture-identity", &credential),
        Err(AuthErrorCode::StorageUnavailable)
    );
    assert_eq!(store.read_metadata(&scope, ACCOUNT_ID), Err(AuthErrorCode::NotFound));
    let owner = store.credentials.for_owner(scope.principal).unwrap();
    assert!(
        matches!(owner.read(&tenant, scope.host, AppId::Codex, ACCOUNT_ID), Err(error) if error == "official account not found")
    );
    connection.execute_batch("DROP TRIGGER reject_auth_insert;").unwrap();
    store.store_codex_device(scope, &account, "fixture-identity", &credential).unwrap();
}
