//! Shared native Kimi fixture for authentication and usage unit tests.
use super::{credential_file::{AccountCredential, CredentialFileStore, KimiCredential}, flow::AuthAccountScope, kimi_account::{KimiAccountTarget, KimiNativeTarget}};
use crate::providers::live::commit::{ExecutionHostId, ProviderTransactions};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, sync::Arc};
use tempfile::TempDir;
use vw_api_transport::{TimestampMs, id::TenantScope};
use vw_api_app_auth::{AuthRegion, AuthService};
pub const CONFIG: &str = r#"# Kimi configuration
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
pub const DEVICE: &str = "d8c8a37b-175c-4e59-9501-07351b0da278";
pub const HOST: &str = "fixture-host";
pub const SLOT: &str = "kimi-code-fixture";
pub const ACCOUNT: &str = "managed-account";
pub const FUTURE: i64 = 4_102_444_800;

pub struct Fixture {
    pub(crate) home: TempDir,
    pub credentials: CredentialFileStore,
    pub(crate) transactions: Arc<ProviderTransactions>,
    pub(crate) tenant: TenantScope,
}

impl Fixture {
    pub fn new() -> Self {
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

    pub fn target(&self, region: AuthRegion) -> KimiAccountTarget<'_> {
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

    pub fn native(&self) -> KimiNativeTarget<'_> {
        KimiNativeTarget {
            home: self.home.path(),
            transactions: &self.transactions,
            provider_id: "subscription",
        }
    }

    pub fn config(&self) -> PathBuf {
        self.home.path().join(".kimi-code/config.toml")
    }

    pub fn credential(&self) -> PathBuf {
        self.home.path().join(format!(".kimi-code/credentials/{SLOT}.json"))
    }

    pub fn lock(&self) -> PathBuf {
        self.home.path().join(format!(".kimi-code/oauth/{SLOT}.lock"))
    }

    pub fn write_native(&self, tokens: &Value) {
        fs::write(self.credential(), tokens.to_string()).unwrap();
        fs::set_permissions(self.credential(), fs::Permissions::from_mode(0o600)).unwrap();
    }
}

pub fn native_tokens(prefix: &str, expires_at: i64) -> Value {
    json!({
        "access_token":format!("{prefix}-access"), "refresh_token":format!("{prefix}-refresh"),
        "expires_at":expires_at, "expires_in":900, "token_type":"Bearer", "scope":""
    })
}
