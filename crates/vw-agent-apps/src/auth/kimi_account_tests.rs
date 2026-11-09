use super::{
    copilot_tests::oauth_server,
    credential_file::{AccountCredential, CredentialFileStore, KimiCredential},
    flow::AuthAccountScope,
    kimi::KimiDeviceAuth,
    kimi_account::{KimiAccountTarget, KimiNativeTarget},
};
use crate::providers::live::commit::{ExecutionHostId, ProviderTransactions};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command, sync::Arc};
use tempfile::TempDir;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthRegion, AuthService},
    id::TenantScope,
};

pub(super) const CONFIG: &str = r#"# Kimi configuration
default_model = "original" # user comment

[providers.other]
type = "kimi"

[providers.subscription]
type = "kimi"
base_url = "https://api.kimi.ai/coding/v1"
[providers.subscription.oauth]
storage = "file"
key = "oauth/kimi-code-fixture"
oauth_host = "https://auth.kimi.ai"

[models.original]
provider = "other"
model = "other-model"

[models.chosen]
provider = "subscription"
model = "kimi-for-coding"
"#;
const DEVICE: &str = "d8c8a37b-175c-4e59-9501-07351b0da278";
const HOST: &str = "fixture-host";
const SLOT: &str = "kimi-code-fixture";
pub(super) const ACCOUNT: &str = "managed-account";
const FUTURE: i64 = 4_102_444_800;

pub(super) struct Fixture {
    home: TempDir,
    pub(super) credentials: CredentialFileStore,
    transactions: Arc<ProviderTransactions>,
    tenant: TenantScope,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let native = home.path().join(".kimi-code");
        for path in ["credentials", "oauth"] {
            fs::create_dir_all(native.join(path)).unwrap();
        }
        fs::write(native.join("config.toml"), CONFIG).unwrap();
        fs::write(native.join("device_id"), DEVICE).unwrap();
        let root = home.path().join("private");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let credentials = CredentialFileStore::new(&root).unwrap();
        let transactions = Arc::new(
            ProviderTransactions::new(&root, ExecutionHostId::new(HOST).unwrap()).unwrap(),
        );
        let fixture = Self { home, credentials, transactions, tenant: TenantScope::default() };
        fixture
            .credentials
            .lease_kimi(fixture.target(AuthRegion::Global).scope, ACCOUNT, true)
            .unwrap()
            .save(&AccountCredential::KimiOauth(KimiCredential {
                access_token: "fixture-expired-access".into(),
                refresh_token: "fixture-expired-refresh".into(),
                expires_at: TimestampMs(1_000),
                device_id: DEVICE.into(),
                region: AuthRegion::Global,
            }))
            .unwrap();
        fixture.write_native(&native_tokens("fixture-expired", 1));
        fixture
    }

    pub(super) fn target(&self, region: AuthRegion) -> KimiAccountTarget<'_> {
        KimiAccountTarget {
            scope: AuthAccountScope {
                tenant: &self.tenant,
                principal: "oidc:owner",
                host: HOST,
                service: AuthService::KimiOauth,
            },
            account_id: ACCOUNT,
            region,
            identity_sha256: Sha256::digest(b"global:41").into(),
        }
    }

    pub(super) fn native(&self) -> KimiNativeTarget<'_> {
        KimiNativeTarget {
            home: self.home.path(),
            transactions: &self.transactions,
            provider_id: "subscription",
        }
    }

    pub(super) fn config(&self) -> PathBuf {
        self.home.path().join(".kimi-code/config.toml")
    }

    pub(super) fn credential(&self) -> PathBuf {
        self.home.path().join(format!(".kimi-code/credentials/{SLOT}.json"))
    }

    fn lock(&self) -> PathBuf {
        self.home.path().join(format!(".kimi-code/oauth/{SLOT}.lock"))
    }

    pub(super) fn write_native(&self, tokens: &Value) {
        fs::write(self.credential(), tokens.to_string()).unwrap();
        fs::set_permissions(self.credential(), fs::Permissions::from_mode(0o600)).unwrap();
    }
}

pub(super) fn native_tokens(prefix: &str, expires_at: i64) -> Value {
    json!({
        "access_token":format!("{prefix}-access"), "refresh_token":format!("{prefix}-refresh"),
        "expires_at":expires_at, "expires_in":900, "token_type":"Bearer", "scope":""
    })
}

#[tokio::test]
async fn auth6_kimi_account_commits_expected_scoped_result() {
    let fixture = Fixture::new();
    let auth = KimiDeviceAuth::new().unwrap();
    let (refresh_url, refresh_sent) = oauth_server(
        "200 OK",
        "",
        json!({"access_token":"fixture-new-access","refresh_token":"fixture-new-refresh","expires_in":900}),
    )
    .await;
    let (me_url, me_sent) = oauth_server("200 OK", "", json!({"user_id":41})).await;
    let result = auth
        .use_default_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.home.path(),
            &fixture.transactions,
            |request| {
                assert!(fixture.lock().is_dir());
                match request.url().as_str() {
                    "https://auth.kimi.ai/api/oauth/token" => {
                        *request.url_mut() = refresh_url.clone()
                    }
                    "https://api.kimi.ai/coding/v1/me" => *request.url_mut() = me_url.clone(),
                    _ => panic!("unexpected Kimi endpoint"),
                }
            },
        )
        .await;
    assert_eq!(result, Ok(()), "verified regional account must become the default");
    let sent = refresh_sent.await.unwrap();
    assert!(sent.ends_with("client_id=17e5f671-d194-4dfb-9706-5516cb48c098&grant_type=refresh_token&refresh_token=fixture-expired-refresh"));
    assert!(sent.contains(&format!("\r\nx-msh-device-id: {DEVICE}\r\n")));
    assert!(me_sent.await.unwrap().contains("\r\nauthorization: Bearer fixture-new-access\r\n"));
    assert_eq!(
        fs::read_to_string(fixture.config()).unwrap(),
        CONFIG.replacen("\"original\" #", "\"chosen\" #", 1)
    );
    let saved: Value = serde_json::from_slice(&fs::read(fixture.credential()).unwrap()).unwrap();
    assert_eq!(saved["refresh_token"], "fixture-new-refresh");
    assert!(!fixture.lock().exists());
    let lease = fixture
        .credentials
        .lease_kimi(fixture.target(AuthRegion::Global).scope, ACCOUNT, false)
        .unwrap();
    let AccountCredential::KimiOauth(saved) = lease.read().unwrap() else {
        panic!("expected a Kimi credential");
    };
    assert_eq!(saved.refresh_token, "fixture-new-refresh");
    drop(lease);

    let fixture = Fixture::new();
    let (refresh_url, refresh_sent) =
        oauth_server("400 Bad Request", "", json!({"error":"invalid_grant"})).await;
    let (me_url, me_sent) = oauth_server("200 OK", "", json!({"user_id":41})).await;
    let rotated = native_tokens("fixture-external", FUTURE);
    let staged = fixture.credential().with_extension("rotating");
    fs::write(&staged, rotated.to_string()).unwrap();
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).unwrap();
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |request| match request.url().path() {
                "/api/oauth/token" => {
                    assert!(fixture.lock().is_dir());
                    // A second process has already prepared the next generation.
                    assert!(
                        Command::new("/bin/mv")
                            .arg(&staged)
                            .arg(fixture.credential())
                            .status()
                            .unwrap()
                            .success()
                    );
                    *request.url_mut() = refresh_url.clone();
                }
                "/coding/v1/me" => *request.url_mut() = me_url.clone(),
                _ => panic!("unexpected Kimi endpoint"),
            },
        )
        .await;
    assert_eq!(result, Ok(()));
    assert!(refresh_sent.await.unwrap().ends_with("refresh_token=fixture-expired-refresh"));
    assert!(
        me_sent.await.unwrap().contains("\r\nauthorization: Bearer fixture-external-access\r\n")
    );
    assert_eq!(fs::read_to_string(fixture.credential()).unwrap(), rotated.to_string());
    assert_eq!(
        fs::read_to_string(fixture.config()).unwrap(),
        CONFIG.replacen("\"original\" #", "\"chosen\" #", 1)
    );
    assert!(!fixture.lock().exists());
}

#[tokio::test]
async fn auth6_kimi_account_rejects_invalid_or_stale_input() {
    let fixture = Fixture::new();
    let auth = KimiDeviceAuth::new().unwrap();
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::MainlandCn),
            fixture.native(),
            |_| panic!("region mismatch must fail before contacting Kimi"),
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::IdentityMismatch));

    assert!(Command::new("/bin/mkdir").arg(fixture.lock()).status().unwrap().success());
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |_| panic!("a contended slot must fail before contacting Kimi"),
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::AccountInUse));
    assert!(fixture.lock().exists());
    fs::remove_dir(fixture.lock()).unwrap();

    fixture.write_native(&native_tokens("fixture-foreign", FUTURE));
    let (url, sent) = oauth_server("200 OK", "", json!({"user_id":42})).await;
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |request| {
                assert_eq!(request.url().as_str(), "https://api.kimi.ai/coding/v1/me");
                *request.url_mut() = url.clone();
            },
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::IdentityMismatch));
    sent.await.unwrap();
    assert_eq!(fs::read_to_string(fixture.config()).unwrap(), CONFIG);
    assert_eq!(
        fs::read_to_string(fixture.credential()).unwrap(),
        native_tokens("fixture-foreign", FUTURE).to_string()
    );
    assert!(!fixture.lock().exists());

    fixture.write_native(&native_tokens("fixture-expired-other", 2));
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |_| panic!("an expired independent grant cannot prove its identity"),
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::ReauthRequired));
    assert_eq!(
        fs::read_to_string(fixture.credential()).unwrap(),
        native_tokens("fixture-expired-other", 2).to_string()
    );

    fixture.write_native(&native_tokens("fixture-expired", 1));
    let (url, sent) = oauth_server("400 Bad Request", "", json!({"error":"invalid_grant"})).await;
    let result = auth
        .use_account_with(
            &fixture.credentials,
            fixture.target(AuthRegion::Global),
            fixture.native(),
            |request| {
                assert_eq!(request.url().as_str(), "https://auth.kimi.ai/api/oauth/token");
                *request.url_mut() = url.clone();
            },
        )
        .await;
    assert_eq!(result, Err(AuthErrorCode::ReauthRequired));
    assert!(sent.await.unwrap().ends_with("refresh_token=fixture-expired-refresh"));
    let revoked: Value = serde_json::from_slice(&fs::read(fixture.credential()).unwrap()).unwrap();
    assert_eq!(revoked["refresh_token"], "");
    assert_eq!(revoked["access_token"], "");
    assert_eq!(revoked["expires_at"], 0);
    assert_eq!(fs::read_to_string(fixture.config()).unwrap(), CONFIG);
    assert!(!fixture.lock().exists());
}

#[path = "kimi_account/binding_tests.rs"]
mod binding_tests;

#[path = "binding_tests.rs"]
mod scoped_binding_tests;
