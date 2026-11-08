use super::*;
use crate::agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt};
use vw_agent_apps::{
    apps::launch::request::LaunchMaterial,
    auth::credential_file::{AccountCredential, CredentialFileStore},
};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
};

#[test]
fn provider_codex_managed_binding_consumes_only_the_scoped_isolated_account() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("private");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let db = GlobalDatabase::for_home(home.path()).unwrap();
    let store = AppProviderStore::new(db.clone()).unwrap();
    let accounts = AuthAccountStore::new(db, &root).unwrap();
    let tenant = TenantScope::new("p", "c").unwrap();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "host",
        service: AuthService::CodexOauth,
    };
    let id = "managed";
    let owner = CredentialFileStore::new(&root).unwrap().for_owner(scope.principal).unwrap();
    let credential = AccountCredential::CodexAuth(json!({
        "auth_mode":"chatgpt",
        "tokens":{"access_token":"fixture-access", "refresh_token":"fixture-refresh", "account_id":"workspace",
                  "id_token":format!("{}.{}.signature", URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#), URL_SAFE_NO_PAD.encode(br#"{"sub":"subject","chatgpt_account_id":"workspace"}"#))}
    }));
    owner.lease(&tenant, scope.host, AppId::Codex, id, true).unwrap().save(&credential).unwrap();
    let metadata = AuthAccountMetadata::new(r#"["workspace","subject"]"#, TimestampMs(1)).unwrap();
    accounts.insert_metadata(&scope, id, &metadata).unwrap();

    let native = home.path().join(".codex");
    fs::create_dir(&native).unwrap();
    fs::write(native.join("config.toml"), "model_provider='openai'\nmodel='fixture-model'\n")
        .unwrap();
    fs::write(native.join("auth.json"), "{}").unwrap();
    let profile = vw_agent_apps::providers::codex_catalog::import(
        Some(&fs::read(native.join("config.toml")).unwrap()),
        Some(b"{}"),
    )
    .unwrap();
    store.codex_save(&tenant, "official", &profile, None, None).unwrap();

    let project = home.path().join("project");
    fs::create_dir_all(project.join(".git")).unwrap();
    let executable = project.join("codex");
    fs::write(
        &executable,
        br#"#!/bin/sh
set -eu
test "$1" = "--model"
test "$2" = "fixture-model"
/usr/bin/grep -q 'model_provider = "openai"' "$CODEX_HOME/config.toml"
/usr/bin/grep -q 'fixture-access' "$CODEX_HOME/auth.json"
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let material = || LaunchMaterial {
        program: executable.clone(),
        cwd: project.clone(),
        args: Vec::new(),
        skip_permissions: false,
    };

    assert!(
        store
            .prepare_bound_official_launch(scope, "official", Some(&root), material())
            .unwrap()
            .is_none()
    );
    store.bind_codex_account(scope, &root, "official", None, Some(id)).unwrap();
    assert_eq!(store.codex_account_binding(scope, "official"), Ok(Some(id.into())));
    assert!(matches!(
        store.prepare_bound_official_launch(
            AuthAccountScope { principal: "foreign", ..scope },
            "official",
            Some(&root),
            material()
        ),
        Err(AuthErrorCode::Forbidden)
    ));
    assert_eq!(
        store.with_codex_provider_unreferenced(scope, &root, id, |_| Ok(())),
        Err(AuthErrorCode::AccountInUse)
    );
    let launch = store
        .prepare_bound_official_launch(scope, "official", Some(&root), material())
        .unwrap()
        .unwrap();
    assert!(owner.lease(&tenant, scope.host, AppId::Codex, id, false).is_err());
    launch.run().unwrap();
    assert_eq!(fs::read(native.join("auth.json")).unwrap(), b"{}");
    store.bind_codex_account(scope, &root, "official", Some(id), None).unwrap();
    assert_eq!(store.with_codex_provider_unreferenced(scope, &root, id, |_| Ok(())), Ok(()));

    let mut custom = profile;
    custom.config = "openai_base_url='https://untrusted.invalid'".into();
    store.codex_save(&tenant, "custom", &custom, None, None).unwrap();
    assert_eq!(
        store.bind_codex_account(scope, &root, "custom", None, Some(id)),
        Err(AuthErrorCode::Unsupported)
    );
}
