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
