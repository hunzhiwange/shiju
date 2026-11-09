use std::{
    fs,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageSource,
    },
    id::TenantScope,
};

use super::claude::{ClaudeUsage, LoginSlots, query_at};
use crate::{
    auth::{
        claude_keychain::{ClaudeKeychain, KeychainError},
        credential_file::{AccountCredential, CredentialFileStore},
        credential_lease::CredentialLease,
        flow::AuthAccountScope,
    },
    providers::{
        claude::ClaudeFile,
        live::commit::{AppState, ClaudeManagedLogin, ExecutionHostId, ProviderTransactions},
    },
    usage::{
        binding::{
            UnboundUsageAccount, UsageAccountSelection, UsageCredentialRef,
            resolve_usage_credential,
        },
        coding::zhipu::client_builder,
    },
};

const NOW: TimestampMs = TimestampMs(1_893_456_000_000);
const ID: &str = "fixture-query";
const BOUND: &str = "fixture-managed";
const NATIVE: &str = "fixture-native";

struct FixtureKeychain {
    home: PathBuf,
    native: Option<Value>,
}
impl ClaudeKeychain for FixtureKeychain {
    fn authorize_home(&self, home: &Path) -> Result<(), KeychainError> {
        if self.home == home { Ok(()) } else { Err(KeychainError) }
    }
    fn read(&self, directory: Option<&Path>) -> Result<Option<Value>, KeychainError> {
        Ok(if directory.is_none() { self.native.clone() } else { None })
    }
    fn write(
        &self,
        _directory: Option<&Path>,
        _value: Option<&Value>,
    ) -> Result<(), KeychainError> {
        panic!("usage queries must never update credentials");
    }
}

struct Fixture {
    home: tempfile::TempDir,
    tx: ProviderTransactions,
    store: CredentialFileStore,
    tenant: TenantScope,
    keychain: FixtureKeychain,
    managed: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let private = home.path().join("private");
        let store_root = home.path().join("credentials");
        for path in [&home.path().join(".claude"), &private, &store_root] {
            fs::DirBuilder::new().mode(0o700).create(path).unwrap();
        }
        let store = CredentialFileStore::new(&store_root).unwrap();
        let tenant = TenantScope::default();
        let owner = store.for_owner("owner").unwrap();
        owner.register_claude_directory(&tenant, "fixture-host", "bound").unwrap();
        let AccountCredential::ClaudeDirectory(directory) =
            owner.read(&tenant, "fixture-host", AppId::Claude, "bound").unwrap()
        else {
            panic!("expected a managed Claude directory");
        };
        let managed = directory.path().to_owned();
        let tx = ProviderTransactions::new(&private, ExecutionHostId::new("fixture-host").unwrap())
            .unwrap();
        let keychain = FixtureKeychain { home: home.path().into(), native: None };
        let fixture = Self { home, tx, store, tenant, keychain, managed };
        fixture.write_managed(NOW.0 + 1000);
        fixture.write_native(NOW.0 + 1000);
        fixture
    }

    fn write_managed(&self, expires_at: i64) {
        write_login(&self.managed, &self.managed, "managed", BOUND, "max", expires_at);
    }

    fn write_native(&self, expires_at: i64) {
        let home = self.home.path();
        write_login(&home.join(".claude"), home, "native", NATIVE, "pro", expires_at);
    }

    fn selection(&self, bound: bool) -> UsageCredentialRef<'_> {
        let scope = AuthAccountScope {
            tenant: &self.tenant,
            principal: "owner",
            host: "fixture-host",
            service: AuthService::ClaudeOfficial,
        };
        resolve_usage_credential(
            scope,
            scope,
            UsageAccountSelection {
                app: AppId::Claude,
                account_id: None,
                bound_account_id: bound.then_some("bound"),
                unbound: UnboundUsageAccount::NativeLogin { is_owner: true },
            },
            |_| panic!("Claude must not consult the managed default"),
            |_, account| {
                assert_eq!(account, "bound");
                Ok(())
            },
        )
        .unwrap()
    }

    fn lease(
        &self,
        scope: AuthAccountScope<'_>,
        id: &str,
    ) -> Result<(CredentialLease, [u8; 32]), AuthErrorCode> {
        let lease = self
            .store
            .for_owner(scope.principal)
            .and_then(|store| store.lease(scope.tenant, scope.host, AppId::Claude, id, false))
            .map_err(|_| AuthErrorCode::NotFound)?;
        Ok((lease, Sha256::digest(b"managed:org").into()))
    }

    async fn query(&self, bound: bool, origin: &str) -> Result<ClaudeUsage, AuthErrorCode> {
        query_at(
            LoginSlots { home: self.home.path(), transactions: &self.tx, keychain: &self.keychain },
            &self.selection(bound),
            |scope, id| self.lease(scope, id),
            client_builder().no_proxy(),
            origin,
            NOW,
            ID,
        )
        .await
    }
}

fn write_login(
    directory: &Path,
    profile: &Path,
    account: &str,
    token: &str,
    plan: &str,
    expires_at: i64,
) {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(directory.join(".credentials.json"))
        .unwrap();
    serde_json::to_writer(
        file,
        &json!({
            "claudeAiOauth": {
                "accessToken": token,
                "expiresAt": expires_at,
                "refreshToken": "fixture-refresh",
                "subscriptionType": plan,
                "rateLimitTier": format!("tier_{plan}")
            }
        }),
    )
    .unwrap();
    fs::write(
        profile.join(".claude.json"),
        json!({
            "oauthAccount": {"accountUuid": account, "organizationUuid": "org"}
        })
        .to_string(),
    )
    .unwrap();
}

async fn replay(
    fixture: &Fixture,
    bound: bool,
    token: &'static str,
    status: &str,
    body: &str,
) -> ClaudeUsage {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = BufReader::new(stream);
        let mut line = String::new();
        stream.read_line(&mut line).await.unwrap();
        assert_eq!(line, "GET /api/oauth/usage HTTP/1.1\r\n");
        let mut headers = std::collections::HashMap::new();
        loop {
            line.clear();
            assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            let (name, value) = line.split_once(':').unwrap();
            headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
        }
        assert_eq!(headers["authorization"], format!("Bearer {token}"));
        assert_eq!(headers["anthropic-beta"], "oauth-2025-04-20");
        assert_eq!(headers["accept"], "application/json");
        stream.get_mut().write_all(response.as_bytes()).await.unwrap();
    });
    let result = fixture.query(bound, &origin).await.unwrap();
    timeout(Duration::from_secs(1), server).await.unwrap().unwrap();
    result
}

const RESPONSE: &str = r#"{
    "five_hour":{"utilization":25,"resets_at":"2030-01-01T05:00:00Z"},
    "seven_day_opus":{"utilization":90},
    "other_window":{"utilization":12},
    "limits":[
        {"kind":"weekly_scoped","group":"weekly","scope":{"model":{"display_name":"Opus"},"surface":"cli"},"percent":99},
        {"kind":"weekly_scoped","group":"weekly","scope":{"model":{"display_name":"Opus"}},"percent":0,"resets_at":null,"is_active":false}
    ],
    "extra_usage":{"is_enabled":true,"monthly_limit":500,"used_credits":150,"currency":"USD"}
}"#;

#[tokio::test]
async fn usage_subscriptions_claude_returns_expected_result() {
    let fixture = Fixture::new();
    for (bound, token, plan) in [(true, BOUND, "max"), (false, NATIVE, "pro")] {
        let usage = replay(&fixture, bound, token, "200 OK", RESPONSE).await;
        assert_eq!(usage.quota.source, UsageSource::Official("claude".into()));
        assert_eq!(usage.quota.queried_at, Some(NOW));
        assert_eq!(usage.quota.credential_state, UsageCredentialState::Valid);
        let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(data) } =
            usage.quota.outcome
        else {
            panic!("expected official Claude quota")
        };
        assert_eq!(data.subscription_type.as_deref(), Some(plan));
        assert_eq!(data.rate_limit_tier, Some(format!("tier_{plan}")));
        assert_eq!(
            data.windows.iter().map(|window| window.name.as_str()).collect::<Vec<_>>(),
            ["five_hour", "seven_day_opus", "other_window"]
        );
        assert_eq!(data.windows[0].resets_at.as_deref(), Some("2030-01-01T05:00:00Z"));
        assert_eq!(data.windows[1].utilization.map(f64::from), Some(0.0));
        assert_eq!(data.windows[1].resets_at, None);
        let extra = data.extra_usage.unwrap();
        assert!(extra.is_enabled);
        assert_eq!(extra.monthly_limit.map(f64::from), Some(500.0));
        assert_eq!(extra.used_credits.map(f64::from), Some(150.0));
        assert_eq!(extra.currency.as_deref(), Some("USD"));
    }
}

#[tokio::test]
async fn usage_subscriptions_claude_rejects_invalid_input() {
    let fixture = Fixture::new();
    fixture.write_managed(NOW.0);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let usage = fixture.query(true, &origin).await.unwrap();
    assert_eq!(usage.quota.credential_state, UsageCredentialState::RefreshPending);
    assert_eq!(usage.quota.queried_at, None);
    assert_eq!(usage.subscription_type.as_deref(), Some("max"));
    assert!(matches!(usage.quota.outcome,
        UsageQueryOutcome::Failure { kind: UsageQueryKind::OfficialSubscription, error }
            if error.code == UsageQueryErrorCode::CredentialUnavailable && error.correlation_id == ID
    ));

    fs::write(
        fixture.managed.join(".claude.json"),
        br#"{"oauthAccount":{"accountUuid":"wrong","organizationUuid":"org"}}"#,
    )
    .unwrap();
    assert!(matches!(fixture.query(true, &origin).await, Err(AuthErrorCode::IdentityMismatch)));
    assert!(timeout(Duration::from_millis(50), listener.accept()).await.is_err());
}

#[tokio::test]
async fn usage_subscriptions_claude_reports_upstream_rejection_without_fallback() {
    let fixture = Fixture::new();
    for status in ["401 Unauthorized", "403 Forbidden"] {
        let usage = replay(&fixture, true, BOUND, status, "fixture-private-body").await;
        assert_eq!(usage.quota.credential_state, UsageCredentialState::Expired);
        assert_eq!(usage.quota.queried_at, Some(NOW));
        assert_eq!(usage.subscription_type.as_deref(), Some("max"));
        assert!(matches!(usage.quota.outcome,
            UsageQueryOutcome::Failure { kind: UsageQueryKind::OfficialSubscription, error }
                if error.code == UsageQueryErrorCode::UpstreamUnauthorized
                    && !error.message.contains("fixture-private-body")
        ));
    }
}

#[tokio::test]
async fn usage_subscriptions_claude_uses_live_credentials_only_for_their_owner() {
    let fixture = Fixture::new();
    let home = fixture.home.path();
    // Simulate a checkout followed by rotation in the live slot only.
    fs::write(home.join(".claude/settings.json"), b"{}").unwrap();
    ClaudeFile::open(home, false)
        .unwrap()
        .apply_with_managed_login(
            &fixture.tx,
            &fixture.tenant,
            &AppState::default(),
            None,
            &json!({"env":{}}),
            Some("fixture-provider".into()),
            || true,
            Some("fixture-operation".into()),
            Some(ClaudeManagedLogin {
                tenant: fixture.tenant.clone(),
                principal: "owner".into(),
                host: "fixture-host".into(),
                account_id: "bound".into(),
            }),
        )
        .unwrap();
    write_login(&home.join(".claude"), home, "managed", "rotated-fixture", "team", NOW.0 + 1000);
    let usage = replay(&fixture, true, "rotated-fixture", "200 OK", RESPONSE).await;
    assert_eq!(usage.subscription_type.as_deref(), Some("team"));

    // A stale pointer must not cause the bound query to send a different login's token.
    fixture.write_native(NOW.0 + 1000);
    assert!(matches!(
        fixture.query(true, "http://127.0.0.1:0").await,
        Err(AuthErrorCode::IdentityMismatch)
    ));
}

#[tokio::test]
async fn usage_subscriptions_claude_reads_default_plan_with_the_selected_token() {
    let mut fixture = Fixture::new();
    fixture.keychain.native = Some(json!({"mcpOAuth":{}}));
    let file = replay(&fixture, false, NATIVE, "200 OK", RESPONSE).await;
    assert_eq!(file.subscription_type.as_deref(), Some("pro"));

    fixture.keychain.native = Some(json!({
        "claude.ai_oauth": {
            "accessToken": "keychain-fixture",
            "subscriptionType": "team"
        }
    }));
    let keychain = replay(&fixture, false, "keychain-fixture", "200 OK", RESPONSE).await;
    assert_eq!(keychain.subscription_type.as_deref(), Some("team"));

    fixture.keychain.native.as_mut().unwrap()["claude.ai_oauth"]["accessToken"] = json!(null);
    let invalid = fixture.query(false, "http://127.0.0.1:0").await.unwrap();
    assert_eq!(invalid.quota.credential_state, UsageCredentialState::ParseError);
    assert_eq!(invalid.quota.queried_at, None);
    assert_eq!(invalid.subscription_type.as_deref(), Some("team"));
}
