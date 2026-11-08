use super::{account::prepare_account_launch, request::LaunchMaterial};
use crate::auth::credential_file::{AccountCredential, CredentialFileStore};
use vw_api_types::{agent_apps::AppId, id::TenantScope};

fn private_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    root
}
fn executable(path: &std::path::Path, body: &str) {
    std::fs::write(path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
}
fn auth() -> AccountCredential {
    AccountCredential::CodexAuth(
        serde_json::json!({"auth_mode":"chatgpt", "last_refresh":"2026-01-01T00:00:00Z", "tokens":{"access_token":"fixture-access", "refresh_token":"fixture-refresh", "id_token":"eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJmaXh0dXJlLXVzZXIifQ.fixture", "account_id":"fixture-account"}}),
    )
}

#[test]
fn cc_apps_account_launch_isolation_managed_claude_directory_survives_session() {
    let root = private_root();
    let tenant = TenantScope::new("platform", "company").unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    store.register_claude_directory(&tenant, "host", "selected").unwrap();
    let credential = store.read(&tenant, "host", AppId::Claude, "selected").unwrap();
    let AccountCredential::ClaudeDirectory(directory) = &credential else {
        panic!("expected owner-managed directory")
    };
    let directory_path = directory.path().to_path_buf();
    std::fs::write(
        directory_path.join(".claude.json"),
        r#"{"oauthAccount":{"accountUuid":"fixture-account","organizationUuid":"fixture-org"}}"#,
    )
    .unwrap();
    let program = root.path().join("claude");
    executable(
        &program,
        "#!/bin/sh\ntest -z \"$CLAUDE_CODE_OAUTH_TOKEN$ANTHROPIC_API_KEY\" || exit 2\ntest -f \"$CLAUDE_CONFIG_DIR/.claude.json\" || exit 3\nprintf '%s' \"$CLAUDE_CONFIG_DIR\" > directory-path\nprintf 'fixture-session' > \"$CLAUDE_CONFIG_DIR/session-state\"\n",
    );
    prepare_account_launch(
        AppId::Claude,
        credential,
        LaunchMaterial { program, cwd: root.path().into(), args: vec![], skip_permissions: false },
        root.path(),
    )
    .unwrap()
    .run()
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(root.path().join("directory-path")).unwrap(),
        directory_path.to_str().unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(directory_path.join("session-state")).unwrap(),
        "fixture-session"
    );
    assert!(
        !std::fs::read_dir(root.path())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().starts_with("shiju-account-"))
    );
    assert!(
        store
            .read(
                &TenantScope::new("platform", "other").unwrap(),
                "host",
                AppId::Claude,
                "selected"
            )
            .is_err()
    );
    // An opaque path minted for one scope cannot be rebound to another scope.
    let before = std::fs::read_dir(root.path()).unwrap().count();
    assert!(
        store
            .save(
                &tenant,
                "host",
                AppId::Claude,
                "other",
                &store.read(&tenant, "host", AppId::Claude, "selected").unwrap()
            )
            .is_err()
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), before);
}

#[test]
fn cc_apps_account_launch_isolation_codex_rejects_identity_change_and_cleans_on_writeback_failure()
{
    let root = private_root();
    let program = root.path().join("codex");
    for (body, expected_calls) in [
        (
            "#!/bin/sh\ncat > \"$CODEX_HOME/auth.json\" <<'AUTH'\n{\"tokens\":{\"access_token\":\"fixture-access\",\"refresh_token\":\"fixture-refresh\",\"id_token\":\"fixture-id\",\"account_id\":\"different-account\"}}\nAUTH\nprintf '%s' \"$CODEX_HOME\" > private-path\n",
            0,
        ),
        (
            "#!/bin/sh\nsed 's/2026-01-01/2026-01-02/;s/fixture-refresh/fixture-rotated-refresh/' \"$CODEX_HOME/auth.json\" > rotated\ncat rotated > \"$CODEX_HOME/auth.json\"\nprintf '%s' \"$CODEX_HOME\" > private-path\n",
            1,
        ),
        (
            "#!/bin/sh\nprintf 'invalid' > \"$CODEX_HOME/auth.json\"\nprintf '%s' \"$CODEX_HOME\" > private-path\n",
            0,
        ),
    ] {
        executable(&program, body);
        let mut calls = 0;
        let result = prepare_account_launch(
            AppId::Codex,
            auth(),
            LaunchMaterial {
                program: program.clone(),
                cwd: root.path().into(),
                args: vec![],
                skip_permissions: false,
            },
            root.path(),
        )
        .unwrap()
        .run_with_refresh(|_| {
            calls += 1;
            Err("official credential storage unavailable".into())
        });
        assert!(result.is_err());
        assert_eq!(calls, expected_calls);
        let private = std::fs::read_to_string(root.path().join("private-path")).unwrap();
        assert!(!std::path::Path::new(&private).exists());
    }
}

#[test]
fn cc_apps_account_launch_isolation_credential_lease_serializes_processes_and_releases_after_exit()
{
    const CHILD: &str = "SHIJU_OFFICIAL_LEASE_CHILD";
    const EXPECT_BUSY: &str = "SHIJU_OFFICIAL_LEASE_BUSY";
    let tenant = TenantScope::new("platform", "company").unwrap();
    if let Some(path) = std::env::var_os(CHILD) {
        let store = CredentialFileStore::new(std::path::Path::new(&path)).unwrap();
        let result = store.lease(&tenant, "host", AppId::Codex, "selected", false);
        assert_eq!(result.is_err(), std::env::var_os(EXPECT_BUSY).is_some());
        return;
    }
    let root = private_root();
    let store = CredentialFileStore::new(root.path()).unwrap();
    store.save(&tenant, "host", AppId::Codex, "selected", &auth()).unwrap();
    let lease = store.lease(&tenant, "host", AppId::Codex, "selected", false).unwrap();
    assert!(store.save(&tenant, "host", AppId::Codex, "selected", &auth()).is_err());
    let run_child = |busy: bool| {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "apps::launch::account_lifecycle_tests::cc_apps_account_launch_isolation_credential_lease_serializes_processes_and_releases_after_exit"]).env(CHILD, root.path()).env_remove(EXPECT_BUSY);
        if busy {
            command.env(EXPECT_BUSY, "1");
        }
        assert!(command.status().unwrap().success());
    };
    run_child(true);
    drop(lease);
    run_child(false);
    assert!(store.lease(&tenant, "host", AppId::Codex, "selected", false).is_ok());
}

#[test]
fn cc_apps_account_launch_isolation_agy_bound_account_refused_without_global_keychain_side_effects()
{
    let root = private_root();
    // A boundary sentinel stands in for existing host state; the implementation never opens security.
    std::fs::write(root.path().join("host-keychain-sentinel"), "unchanged-fixture").unwrap();
    let program = root.path().join("agy");
    executable(&program, "#!/bin/sh\nprintf 'incorrectly-started' > launched\n");
    let tenant = TenantScope::new("platform", "company").unwrap();
    let store = CredentialFileStore::new(root.path()).unwrap();
    let before = std::fs::read_dir(root.path()).unwrap().count();
    assert!(
        store
            .save(&tenant, "host", AppId::Agy, "bound-account", &auth())
            .unwrap_err()
            .contains("isolated OS keychain")
    );
    let result = prepare_account_launch(
        AppId::Agy,
        auth(),
        LaunchMaterial { program, cwd: root.path().into(), args: vec![], skip_permissions: false },
        root.path(),
    );
    assert!(matches!(result, Err(ref error) if error.contains("isolated OS keychain")));
    assert!(!root.path().join("launched").exists());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), before);
    assert_eq!(
        std::fs::read_to_string(root.path().join("host-keychain-sentinel")).unwrap(),
        "unchanged-fixture"
    );
}
