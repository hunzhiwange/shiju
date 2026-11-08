use super::*;
use crate::{
    agent_apps::auth_accounts::{AuthAccountMetadata, AuthAccountScope, AuthAccountStore},
    app_providers::{AppProviderStore, ClaudeBindingHost},
};
use vw_agent_apps::auth::credential_file::CredentialFileStore;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
};

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "claude",
        host: "host",
        service: AuthService::ClaudeOfficial,
    }
}

fn host<'a>(f: &'a Fixture, root: &'a Path) -> ClaudeBindingHost<'a> {
    ClaudeBindingHost { home: f.root.path(), private_root: root }
}

fn register_managed(f: &Fixture, root: &Path, scope: AuthAccountScope<'_>, k: &Keychain, id: &str) {
    let credentials = CredentialFileStore::new(root).unwrap().for_owner(scope.principal).unwrap();
    credentials.register_claude_directory(scope.tenant, scope.host, id).unwrap();
    let AccountCredential::ClaudeDirectory(directory) =
        credentials.read(scope.tenant, scope.host, AppId::Claude, id).unwrap()
    else {
        panic!("missing scoped directory");
    };
    std::fs::write(directory.path().join(".claude.json"), profile(id).to_string()).unwrap();
    k.write(Some(directory.path()), Some(&secret(id))).unwrap();
    let accounts = AuthAccountStore::new(f.db.clone(), root).unwrap();
    accounts
        .insert_metadata(
            &scope,
            id,
            &AuthAccountMetadata::new(&format!("{id}:org"), TimestampMs(1)).unwrap(),
        )
        .unwrap();
}

fn allow_native_checkout(f: &Fixture) {
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["hosts"][0]["native_login_owners"] = json!([
        {"service":"claude_official","platform_id":"p","company_id":"a","principal":"claude"}
    ]);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
}

async fn save_card(f: &Fixture, id: &str) {
    let request =
        f.request(ClaudeConfig::Official { account: None }, ClaudeActivation::CatalogOnly);
    f.client.claude_provider_create(id, &request, TOKEN).await.unwrap();
}

fn immutable(path: &Path, enabled: bool) {
    assert!(
        std::process::Command::new("/usr/bin/chflags")
            .arg(if enabled { "uchg" } else { "nouchg" })
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
}

#[tokio::test]
async fn provider_claude_managed_binding_retains_catalog_and_live_references_under_lease() {
    let keychain = Arc::new(Keychain::default());
    let f = Fixture::new_with_keychain(Some(keychain.clone())).await;
    let root = f.root.path().join("private");
    let tenant = TenantScope::new("p", "a").unwrap();
    let owner = scope(&tenant);
    let foreign = AuthAccountScope { principal: "other", ..owner };
    let store = AppProviderStore::new(f.db.clone()).unwrap();
    register_managed(&f, &root, owner, &keychain, "same");
    register_managed(&f, &root, foreign, &keychain, "same");
    save_card(&f, "bound").await;
    save_card(&f, "unbound").await;

    assert_eq!(store.claude_account_binding(owner, "bound"), Ok(None));
    assert_eq!(
        store.bind_claude_account(owner, host(&f, &root), "bound", None, Some("same")),
        Ok(())
    );
    assert_eq!(store.claude_account_binding(owner, "bound"), Ok(Some("same".into())));
    assert_eq!(store.claude_account_binding(foreign, "bound"), Err(AuthErrorCode::Forbidden));
    assert_eq!(
        store.with_claude_provider_unreferenced(owner, host(&f, &root), "same", |_| Ok(())),
        Err(AuthErrorCode::AccountInUse)
    );
    assert_eq!(
        store.with_claude_provider_unreferenced(foreign, host(&f, &root), "same", |lease| {
            assert!(matches!(lease.read(), Ok(AccountCredential::ClaudeDirectory(_))));
            let other =
                CredentialFileStore::new(&root).unwrap().for_owner(foreign.principal).unwrap();
            assert!(
                other.lease(foreign.tenant, foreign.host, AppId::Claude, "same", false).is_err()
            );
            Ok(())
        }),
        Ok(())
    );

    assert_eq!(
        f.client.claude_provider_switch("bound", &f.target(), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Failed
    );
    assert_eq!(keychain.read(None).unwrap(), None);
    allow_native_checkout(&f);
    let before = std::fs::read(f.file()).unwrap();
    immutable(&f.file(), true);
    let failed = f.client.claude_provider_switch("bound", &f.target(), TOKEN).await;
    immutable(&f.file(), false);
    assert_eq!(failed.unwrap().live, ClaudeLiveResult::Failed);
    assert_eq!(keychain.read(None).unwrap(), None);
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert!(!f.root.path().join(".claude/.shiju-login.json").exists());
    assert_eq!(
        f.client.claude_provider_switch("bound", &f.target(), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    assert_eq!(keychain.read(None).unwrap(), Some(secret("same")));
    assert_eq!(f.state()["claude_managed_login"]["principal"], "claude");
    assert_eq!(
        store.bind_claude_account(owner, host(&f, &root), "bound", Some("same"), None),
        Err(AuthErrorCode::Conflict)
    );

    assert_eq!(
        f.client.claude_provider_switch("unbound", &f.target(), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Applied
    );
    store.bind_claude_account(owner, host(&f, &root), "bound", Some("same"), None).unwrap();
    assert_eq!(store.claude_account_binding(owner, "bound"), Ok(None));
    assert_eq!(
        store.with_claude_provider_unreferenced(owner, host(&f, &root), "same", |_| Ok(())),
        Err(AuthErrorCode::AccountInUse)
    );

    register_managed(&f, &root, owner, &keychain, "next");
    let mut next =
        f.request(ClaudeConfig::Official { account: None }, ClaudeActivation::CatalogOnly);
    next.claude.model = Some("fixture-next-model".into());
    f.client.claude_provider_create("next", &next, TOKEN).await.unwrap();
    store.bind_claude_account(owner, host(&f, &root), "next", None, Some("next")).unwrap();
    let state_path = root.join("provider-live/aG9zdA/claude.state.json");
    immutable(&state_path, true);
    let failed = f.client.claude_provider_switch("next", &f.target(), TOKEN).await;
    immutable(&state_path, false);
    assert_eq!(failed.unwrap().live, ClaudeLiveResult::RecoveryRequired);
    assert_eq!(keychain.read(None).unwrap(), Some(secret("next")));
    assert_eq!(f.state()["claude_managed_login"]["account_id"], "same");
    use vw_agent_apps::providers::{
        claude_login::LoginHost,
        live::commit::{ExecutionHostId, ProviderTransactions},
    };
    let tx = ProviderTransactions::new(&root, ExecutionHostId::new("host").unwrap()).unwrap();
    let recovered = tx.recover_on_startup();
    assert!(matches!(recovered.as_slice(), [(AppId::Claude, Ok(_))]));
    let login = LoginHost::open(f.root.path()).unwrap();
    login.recover(&tx, keychain.as_ref()).unwrap();
    drop(login);
    assert_eq!(f.state()["claude_managed_login"]["account_id"], "next");
    assert_eq!(keychain.read(None).unwrap(), Some(secret("next")));
    assert_eq!(
        store.with_claude_provider_unreferenced(owner, host(&f, &root), "same", |_| Ok(())),
        Ok(())
    );
}

#[tokio::test]
async fn provider_claude_managed_binding_rejects_legacy_and_unknown_live_identity() {
    let keychain = Arc::new(Keychain::default());
    let f = Fixture::new_with_keychain(Some(keychain.clone())).await;
    let root = f.root.path().join("private");
    let tenant = TenantScope::new("p", "a").unwrap();
    let scope = scope(&tenant);
    register(&f, &keychain);
    let store = AppProviderStore::new(f.db.clone()).unwrap();
    save_card(&f, "candidate").await;
    let raw = f.record("candidate").unwrap();
    let bytes = std::fs::read(f.file()).unwrap();
    assert_eq!(
        store.bind_claude_account(scope, host(&f, &root), "candidate", None, Some("existing")),
        Err(AuthErrorCode::NotFound)
    );
    assert_eq!(f.record("candidate").unwrap(), raw);
    allow_native_checkout(&f);
    register_managed(&f, &root, scope, &keychain, "existing");
    store.bind_claude_account(scope, host(&f, &root), "candidate", None, Some("existing")).unwrap();
    assert_eq!(
        f.client.claude_provider_switch("candidate", &f.target(), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Failed
    );
    assert_eq!(keychain.read(None).unwrap(), Some(secret("original")));
    assert_eq!(std::fs::read(f.file()).unwrap(), bytes);
    assert_eq!(store.claude_account_binding(scope, "candidate"), Ok(Some("existing".into())));
    assert!(!f.root.path().join(".claude/.shiju-login.json").exists());
}
