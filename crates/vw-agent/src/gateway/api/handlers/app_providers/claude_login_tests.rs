use super::*;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};
use vw_agent_apps::auth::claude_keychain::KeychainError;
use vw_agent_apps::{
    auth::{claude_keychain::ClaudeKeychain, credential_file::AccountCredential},
    providers::claude_login::LoginMaterial,
};
use vw_api_types::app_providers::claude_login::ClaudeLoginBinding;
#[derive(Default)]
struct Keychain(Mutex<BTreeMap<Option<PathBuf>, Value>>);
impl ClaudeKeychain for Keychain {
    fn authorize_home(&self, _: &Path) -> Result<(), KeychainError> {
        Ok(())
    }
    fn read(&self, path: Option<&Path>) -> Result<Option<Value>, KeychainError> {
        Ok(self.0.lock().unwrap().get(&path.map(Path::to_path_buf)).cloned())
    }
    fn write(&self, path: Option<&Path>, value: Option<&Value>) -> Result<(), KeychainError> {
        let mut map = self.0.lock().unwrap();
        let key = path.map(Path::to_path_buf);
        if let Some(value) = value {
            map.insert(key, value.clone());
        } else {
            map.remove(&key);
        }
        Ok(())
    }
}
fn profile(id: &str) -> Value {
    json!({"oauthAccount":{"accountUuid":id,"organizationUuid":"org"}})
}
fn secret(id: &str) -> Value {
    json!({"claudeAiOauth":{"refreshToken":format!("fake-{id}")},"organizationUuid":"org"})
}
fn register(f: &Fixture, k: &Keychain) -> PathBuf {
    let tenant = TenantScope::new("p", "a").unwrap();
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&f.policy).unwrap()).unwrap();
    policy["principals"][0]["grants"][0]["identities"] = json!([
        {"appId":"claude","kind":"official_account","id":"existing"},
        {"appId":"claude","kind":"official_account","id":"absent"}
    ]);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    let root = f.root.path().join("private");
    let accounts =
        crate::agent_apps::auth_accounts::AuthAccountStore::new(f.db.clone(), &root).unwrap();
    accounts.register_claude_directory(&tenant, "host", "existing").unwrap();
    let credentials =
        vw_agent_apps::auth::credential_file::CredentialFileStore::new(&root).unwrap();
    let AccountCredential::ClaudeDirectory(directory) =
        credentials.read(&tenant, "host", AppId::Claude, "existing").unwrap()
    else {
        panic!("directory required")
    };
    std::fs::write(directory.path().join(".claude.json"), profile("target").to_string()).unwrap();
    k.write(Some(directory.path()), Some(&secret("target"))).unwrap();
    k.write(None, Some(&secret("original"))).unwrap();
    std::fs::write(f.root.path().join(".claude.json"), profile("original").to_string()).unwrap();
    assert!(
        LoginMaterial::from_account(directory.path(), k).unwrap().identity().unwrap().is_some()
    );
    directory.path().into()
}
fn bound(f: &Fixture, account: &str) -> ClaudeWrite {
    f.request(
        ClaudeConfig::Official { account: Some(ClaudeLoginBinding { account_id: account.into() }) },
        ClaudeActivation::Switch,
    )
}
#[tokio::test]
async fn cc04_claude_login_sdk_api_applies_bound_account_and_archives_original() {
    let k = Arc::new(Keychain::default());
    let f = Fixture::new_with_keychain(Some(k.clone())).await;
    register(&f, k.as_ref());
    let response =
        f.client.claude_provider_create("bound", &bound(&f, "existing"), TOKEN).await.unwrap();
    assert_eq!(response.live, ClaudeLiveResult::Applied);
    assert_eq!(f.state()["pointer"]["id"], "bound");
    assert_eq!(k.read(None).unwrap().unwrap(), secret("target"));
    assert_eq!(f.live()["hooks"], json!({"x":true}));
    assert!(f.live()["env"].get("ANTHROPIC_AUTH_TOKEN").is_none());
    let accounts = crate::agent_apps::auth_accounts::AuthAccountStore::new(
        f.db.clone(),
        &f.root.path().join("private"),
    )
    .unwrap();
    assert_eq!(
        accounts.list(&TenantScope::new("p", "a").unwrap(), "host", AppId::Claude).unwrap().len(),
        2
    );
    assert!(!f.root.path().join(".claude/.shiju-login.json").exists());
}
#[tokio::test]
async fn cc04_claude_login_sdk_api_denial_missing_account_and_write_failure_preserve_login() {
    let k = Arc::new(Keychain::default());
    let f = Fixture::new_with_keychain(Some(k.clone())).await;
    register(&f, k.as_ref());
    let before = std::fs::read(f.file()).unwrap();
    assert!(
        f.client.claude_provider_create("denied", &bound(&f, "existing"), "wrong").await.is_err()
    );
    assert!(f.record("denied").is_none());
    let granted = std::fs::read(&f.policy).unwrap();
    let mut policy: Value = serde_json::from_slice(&granted).unwrap();
    policy["principals"][0]["grants"][0]["identities"] = json!([]);
    std::fs::write(&f.policy, policy.to_string()).unwrap();
    assert!(
        f.client
            .claude_provider_create("no-identity", &bound(&f, "existing"), TOKEN)
            .await
            .is_err()
    );
    assert!(f.record("no-identity").is_none());
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(k.read(None).unwrap().unwrap(), secret("original"));
    std::fs::write(&f.policy, granted).unwrap();

    assert_eq!(
        f.client.claude_provider_create("missing", &bound(&f, "absent"), TOKEN).await.unwrap().live,
        ClaudeLiveResult::Failed
    );
    assert_eq!(std::fs::read(f.file()).unwrap(), before);
    assert_eq!(k.read(None).unwrap().unwrap(), secret("original"));
    std::fs::write(f.file(), b"invalid json").unwrap();
    let response =
        f.client.claude_provider_create("broken", &bound(&f, "existing"), TOKEN).await.unwrap();
    assert!(response.catalog_saved);
    assert_eq!(response.live, ClaudeLiveResult::Failed);
    assert_eq!(std::fs::read(f.file()).unwrap(), b"invalid json");
    assert_eq!(k.read(None).unwrap().unwrap(), secret("original"));
    assert!(!f.root.path().join("private/provider-live/aG9zdA/claude.state.json").exists());
}

#[path = "claude_managed_binding_tests.rs"]
mod managed_binding_tests;
