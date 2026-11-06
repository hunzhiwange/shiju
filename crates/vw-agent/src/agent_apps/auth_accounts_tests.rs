use super::*;
use vw_api_types::agent_apps::*;

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
