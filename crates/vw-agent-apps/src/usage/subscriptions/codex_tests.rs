use std::{fs, os::unix::fs::DirBuilderExt, time::Duration};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryOutcome,
        UsageQueryResult, UsageSource, UsageSubscription,
    },
    id::TenantScope,
};

use super::codex::query_at;
use crate::{
    auth::{
        credential_file::{AccountCredential, CredentialFileStore},
        flow::AuthAccountScope,
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
const MANAGED: &str = "fixture-managed";
const USAGE: &str = r#"{
    "rate_limit": {
        "primary_window":{"used_percent":12.5,"limit_window_seconds":18000,"reset_at":1893474000},
        "secondary_window":{"used_percent":0,"limit_window_seconds":604800,"reset_at":1894060800}
    },
    "credits":{"has_credits":true,"unlimited":false,"balance":"42.5"}
}"#;
const RESETS: &str = r#"{
    "available_count":12,
    "credits":[
        {"status":"available","expires_at":"2030-01-08T00:00:00Z"},
        {"status":"available","expires_at":"2030-01-01T00:00:00Z"},
        {"status":"redeemed","expires_at":"2030-01-02T00:00:00Z"},
        {"status":"available","expires_at":null},
        {"status":"available","expires_at":"2030-01-03T12:00:00.123+01:00"}
    ]
}"#;

fn selection(scope: AuthAccountScope<'_>, managed: bool) -> UsageCredentialRef<'_> {
    resolve_usage_credential(
        scope,
        scope,
        UsageAccountSelection {
            app: AppId::Codex,
            account_id: None,
            bound_account_id: managed.then_some(MANAGED),
            unbound: UnboundUsageAccount::NativeLogin { is_owner: true },
        },
        |_| panic!("must not load a managed default"),
        |bound, id| {
            assert!(managed && bound == scope && id == MANAGED);
            Ok(())
        },
    )
    .unwrap()
}

fn auth(token: &str, workspace: Option<&str>) -> Value {
    json!({
        "auth_mode":"chatgpt",
        "tokens":{
            "access_token":token,
            "refresh_token":"fixture-refresh",
            "id_token":"fixture-id",
            "account_id":workspace
        }
    })
}

async fn replay(
    managed: bool,
    workspace: Option<&str>,
    usage: (&str, &str),
    reset: (&str, &str),
) -> UsageQueryResult {
    replay_with_usage(managed, workspace, Some(usage), reset).await
}

pub(in crate::usage) async fn replay_with_usage(
    managed: bool,
    workspace: Option<&str>,
    usage: Option<(&str, &str)>,
    reset: (&str, &str),
) -> UsageQueryResult {
    let home = tempfile::tempdir().unwrap();
    let codex_home = home.path().join(".codex");
    fs::create_dir(&codex_home).unwrap();
    let token = if managed { "fixture-managed-token" } else { "fixture-cli-token" };
    fs::write(
        codex_home.join("auth.json"),
        auth(if managed { "wrong-cli-token" } else { token }, workspace).to_string(),
    )
    .unwrap();

    let tenant = TenantScope::default();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "fixture-host",
        service: AuthService::CodexOauth,
    };
    let credential = selection(scope, managed);
    let root = home.path().join("private");
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let store = CredentialFileStore::new(&root).unwrap();
    let owner = store.for_owner(scope.principal).unwrap();
    if managed {
        owner
            .save(
                &tenant,
                scope.host,
                AppId::Codex,
                MANAGED,
                &AccountCredential::CodexAuth(auth(token, workspace)),
            )
            .unwrap();
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = async {
        let mut seen = Vec::new();
        for _ in 0..2 {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            let (response, beta) = match line.as_str() {
                "GET /backend-api/wham/usage HTTP/1.1\r\n" => (usage, None),
                "GET /backend-api/wham/rate-limit-reset-credits HTTP/1.1\r\n" => {
                    (Some(reset), Some("codex-1"))
                }
                _ => panic!("unexpected HTTP request"),
            };
            seen.push(beta);
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
            assert_eq!(headers["user-agent"], "codex-cli");
            assert_eq!(headers["accept"], "application/json");
            assert_eq!(headers.get("chatgpt-account-id").map(String::as_str), workspace);
            assert_eq!(headers.get("openai-beta").map(String::as_str), beta);
            let Some((status, body)) = response else {
                continue;
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.get_mut().write_all(response.as_bytes()).await.unwrap();
        }
        seen.sort_unstable();
        assert_eq!(seen, [None, Some("codex-1")]);
    };
    let query =
        query_at(&credential, home.path(), &store, client_builder().no_proxy(), &origin, NOW, ID);
    let ((), result) =
        timeout(Duration::from_secs(3), async { tokio::join!(server, query) }).await.unwrap();
    result.unwrap()
}

fn subscription(result: UsageQueryResult) -> UsageSubscription {
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    assert_eq!(result.queried_at, Some(NOW));
    let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(data) } =
        result.outcome
    else {
        panic!("expected Codex subscription quota");
    };
    data
}

#[tokio::test]
async fn usage_subscriptions_codex_returns_expected_result() {
    for (managed, workspace, source) in
        [(false, Some("cli-workspace"), "codex"), (true, Some("managed-workspace"), "codex_oauth")]
    {
        let result = replay(managed, workspace, ("200 OK", USAGE), ("200 OK", RESETS)).await;
        assert_eq!(result.source, UsageSource::Official(source.into()));
        let data = subscription(result);
        assert_eq!(data.windows.len(), 2);
        assert_eq!(data.windows[0].name, "five_hour");
        assert_eq!(data.windows[0].utilization.map(f64::from), Some(12.5));
        assert_eq!(data.windows[0].resets_at.as_deref(), Some("2030-01-01T05:00:00Z"));
        assert_eq!(data.windows[1].name, "seven_day");
        assert_eq!(data.windows[1].utilization.map(f64::from), Some(0.0));
        assert_eq!(data.windows[1].resets_at, None);
        assert_eq!(data.credits_balance.map(f64::from), Some(42.5));
        assert_eq!(
            data.reset_credits.unwrap(),
            [Some("2030-01-03T11:00:00.123Z".into()), Some("2030-01-08T00:00:00Z".into()), None]
        );
    }
}

#[tokio::test]
async fn usage_subscriptions_codex_rejects_invalid_input() {
    let home = tempfile::tempdir().unwrap();
    let codex_home = home.path().join(".codex");
    fs::create_dir(&codex_home).unwrap();
    let tenant = TenantScope::default();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "fixture-host",
        service: AuthService::CodexOauth,
    };
    let credential = selection(scope, false);
    let root = home.path().join("private");
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let store = CredentialFileStore::new(&root).unwrap();
    for (material, expected) in [
        (
            json!({"auth_mode":"apikey","OPENAI_API_KEY":"fixture-secret"}),
            UsageCredentialState::NotFound,
        ),
        (auth("bad\r\nBearer", Some("cli-workspace")), UsageCredentialState::ParseError),
        (auth("fixture-cli-token", Some("bad\nworkspace")), UsageCredentialState::ParseError),
    ] {
        fs::write(codex_home.join("auth.json"), material.to_string()).unwrap();
        let result = query_at(
            &credential,
            home.path(),
            &store,
            client_builder().no_proxy(),
            "http://127.0.0.1:0",
            NOW,
            ID,
        )
        .await
        .unwrap();
        assert_eq!(result.queried_at, None);
        assert_eq!(result.credential_state, expected);
        let UsageQueryOutcome::Failure { error, .. } = result.outcome else {
            panic!("expected credential failure");
        };
        assert_eq!(error.code, UsageQueryErrorCode::CredentialUnavailable);
        assert_eq!(error.correlation_id, ID);
        assert!(!error.message.contains("fixture-secret"));
    }
}

#[tokio::test]
async fn usage_subscriptions_codex_does_not_fall_back_from_a_missing_managed_account() {
    let home = tempfile::tempdir().unwrap();
    let codex_home = home.path().join(".codex");
    fs::create_dir(&codex_home).unwrap();
    fs::write(
        codex_home.join("auth.json"),
        auth("fixture-cli-token", Some("cli-workspace")).to_string(),
    )
    .unwrap();
    let tenant = TenantScope::default();
    let scope = AuthAccountScope {
        tenant: &tenant,
        principal: "owner",
        host: "fixture-host",
        service: AuthService::CodexOauth,
    };
    let credential = selection(scope, true);
    let root = home.path().join("private");
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let store = CredentialFileStore::new(&root).unwrap();
    let result = query_at(
        &credential,
        home.path(),
        &store,
        client_builder().no_proxy(),
        "http://127.0.0.1:0",
        NOW,
        ID,
    )
    .await
    .unwrap();
    assert_eq!(result.source, UsageSource::Official("codex_oauth".into()));
    assert_eq!(result.queried_at, None);
    assert_eq!(result.credential_state, UsageCredentialState::NotFound);
    let UsageQueryOutcome::Failure { error, .. } = result.outcome else {
        panic!("expected credential failure");
    };
    assert_eq!(error.code, UsageQueryErrorCode::CredentialUnavailable);
}

#[tokio::test]
async fn usage_subscriptions_codex_keeps_quota_when_resets_are_unavailable() {
    let data = subscription(
        replay(false, None, ("200 OK", USAGE), ("503 Service Unavailable", "private body")).await,
    );
    assert_eq!(data.windows.len(), 2);
    assert_eq!(data.credits_balance.map(f64::from), Some(42.5));
    assert_eq!(data.reset_credits, None);

    let data = subscription(
        replay(
            false,
            None,
            ("200 OK", r#"{"credits":{"has_credits":true,"unlimited":true,"balance":99}}"#),
            ("200 OK", r#"{"credits":[]}"#),
        )
        .await,
    );
    assert_eq!(data.credits_balance, None);
    assert_eq!(data.reset_credits, Some(vec![]));
}

#[tokio::test]
async fn usage_subscriptions_codex_classifies_http_failures_without_payload() {
    for (status, state, code) in [
        (
            "401 Unauthorized",
            UsageCredentialState::Expired,
            UsageQueryErrorCode::UpstreamUnauthorized,
        ),
        ("403 Forbidden", UsageCredentialState::Expired, UsageQueryErrorCode::UpstreamUnauthorized),
        (
            "429 Too Many Requests",
            UsageCredentialState::Valid,
            UsageQueryErrorCode::UpstreamRateLimited,
        ),
        (
            "503 Service Unavailable",
            UsageCredentialState::Valid,
            UsageQueryErrorCode::UpstreamUnavailable,
        ),
        ("200 OK", UsageCredentialState::Valid, UsageQueryErrorCode::InvalidResponse),
    ] {
        let result = replay(false, None, (status, "private body"), ("200 OK", RESETS)).await;
        assert_eq!(result.credential_state, state);
        let UsageQueryOutcome::Failure { error, .. } = result.outcome else {
            panic!("expected query failure");
        };
        assert_eq!(error.code, code);
        assert_eq!(error.correlation_id, ID);
        assert!(!error.message.contains("private body"));
    }
}
