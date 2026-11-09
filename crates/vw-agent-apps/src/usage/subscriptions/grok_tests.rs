use std::{collections::HashMap, fs, os::unix::fs::DirBuilderExt, time::Duration};

use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryOutcome, UsageSource,
        UsageSubscription,
    },
    id::TenantScope,
};

use super::grok::{BillingOrigins, GrokQuota, query_at};
use crate::{
    auth::{
        credential_file::{AccountCredential, CredentialFileStore, XaiCredential},
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
const ID: &str = "grok-fixture-query";
const ACCOUNT: &str = "grok-fixture-account";
const BILLING: &str = r#"{
    "config": {
        "creditUsagePercent": 37.5,
        "currentPeriod": {"type": "USAGE_PERIOD_TYPE_WEEKLY", "end": "2030-01-08T00:00:00Z"},
        "monthlyLimit": {"val": 4000},
        "used": {"val": 1000},
        "billingPeriodEnd": "2030-02-01T00:00:00Z",
        "prepaidBalance": {},
        "onDemandCap": {"val": 2500},
        "onDemandUsed": {"val": 500}
    },
    "on_demand_enabled": true,
    "subscription_tier": "SuperGrok Heavy"
}"#;

fn selection(scope: AuthAccountScope<'_>, managed: bool) -> UsageCredentialRef<'_> {
    resolve_usage_credential(
        scope,
        scope,
        UsageAccountSelection {
            app: AppId::GrokBuild,
            account_id: None,
            bound_account_id: managed.then_some(ACCOUNT),
            unbound: UnboundUsageAccount::NativeLogin { is_owner: true },
        },
        |_| panic!("must not load a managed default"),
        |bound, id| {
            assert!(managed && bound == scope && id == ACCOUNT);
            Ok(())
        },
    )
    .unwrap()
}

fn scope(tenant: &TenantScope) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant,
        principal: "owner",
        host: "fixture-host",
        service: AuthService::XaiOauth,
    }
}

fn write_native(home: &std::path::Path, auth: Value) {
    fs::create_dir_all(home.join(".grok")).unwrap();
    fs::write(home.join(".grok/auth.json"), auth.to_string()).unwrap();
}

fn store(home: &std::path::Path) -> CredentialFileStore {
    let root = home.join("private");
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    CredentialFileStore::new(&root).unwrap()
}

fn frame(flags: u8, payload: &[u8]) -> Vec<u8> {
    let mut data = vec![flags];
    data.extend((payload.len() as u32).to_be_bytes());
    data.extend(payload);
    data
}

fn grpc_usage() -> Vec<u8> {
    // config { credit_usage_percent: 12.5 }
    frame(0, &[0x0a, 0x05, 0x0d, 0, 0, 0x48, 0x41])
}

async fn replay(managed: bool, billing: (&str, &str), grpc: (&str, &[u8])) -> GrokQuota {
    let home = tempfile::tempdir().unwrap();
    let store = store(home.path());
    let tenant = TenantScope::default();
    let scope = scope(&tenant);
    let credential = selection(scope, managed);
    write_native(
        home.path(),
        json!({
            "https://auth.x.ai::fixture": {"key": "fixture-cli"},
            "https://accounts.x.ai/sign-in": {"key": "unused-legacy"}
        }),
    );
    if managed {
        store
            .lease_xai(scope, ACCOUNT, true)
            .unwrap()
            .save(&AccountCredential::XaiOauth(XaiCredential {
                access_token: "fixture-managed".into(),
                refresh_token: "fixture-refresh".into(),
                expires_at: TimestampMs(i64::MAX),
                requires_reauth: false,
            }))
            .unwrap();
    }
    let token = if managed { "fixture-managed" } else { "fixture-cli" };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = async {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            stream.read_line(&mut line).await.unwrap();
            let request = line.trim_end().to_owned();
            let mut headers = HashMap::new();
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
            let (status, body) = match request.as_str() {
                "GET /v1/billing?format=credits HTTP/1.1" => {
                    assert_eq!(headers["x-xai-token-auth"], "xai-grok-cli");
                    assert_eq!(headers["accept"], "application/json");
                    (billing.0, billing.1.as_bytes())
                }
                "POST /grok_api_v2.GrokBuildBilling/GetGrokCreditsConfig HTTP/1.1" => {
                    assert_eq!(headers["origin"], "https://grok.com");
                    assert_eq!(headers["content-type"], "application/grpc-web+proto");
                    assert_eq!(headers["x-grpc-web"], "1");
                    assert_eq!(headers["content-length"], "5");
                    let mut body = [1; 5];
                    stream.read_exact(&mut body).await.unwrap();
                    assert_eq!(body, [0; 5]);
                    grpc
                }
                _ => panic!("unexpected request {request}"),
            };
            requests.push(request);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.get_mut().write_all(response.as_bytes()).await.unwrap();
            stream.get_mut().write_all(body).await.unwrap();
        }
        assert_ne!(requests[0], requests[1]);
    };
    let query = query_at(
        &credential,
        home.path(),
        &store,
        client_builder().no_proxy(),
        BillingOrigins { proxy: &origin, web: &origin },
        NOW,
        ID,
    );
    let ((), result) =
        timeout(Duration::from_secs(3), async { tokio::join!(server, query) }).await.unwrap();
    result.unwrap()
}

fn subscription(result: &GrokQuota) -> &UsageSubscription {
    assert_eq!(result.subscription.credential_state, UsageCredentialState::Valid);
    assert_eq!(result.subscription.queried_at, Some(NOW));
    let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(data) } =
        &result.subscription.outcome
    else {
        panic!("expected subscription");
    };
    data
}

#[tokio::test]
async fn usage_subscriptions_grok_returns_expected_result() {
    for (managed, source) in [(false, "grokbuild"), (true, "xai_oauth")] {
        let result = replay(managed, ("200 OK", BILLING), ("200 OK", &grpc_usage())).await;
        assert_eq!(result.subscription.source, UsageSource::Official(source.into()));
        assert!(result.endpoint_errors.is_empty());
        let data = subscription(&result);
        assert_eq!(data.subscription_type.as_deref(), Some("SuperGrok Heavy"));
        assert_eq!(data.windows.len(), 2);
        assert_eq!(data.windows[0].name, "weekly_limit");
        assert_eq!(data.windows[0].utilization.map(f64::from), Some(37.5));
        assert_eq!(data.windows[0].resets_at.as_deref(), Some("2030-01-08T00:00:00Z"));
        assert_eq!(data.windows[1].name, "monthly");
        assert_eq!(data.windows[1].total.map(f64::from), Some(40.0));
        assert_eq!(data.windows[1].remaining.map(f64::from), Some(30.0));
        let extra = data.extra_usage.as_ref().unwrap();
        assert!(extra.is_enabled);
        assert_eq!(extra.monthly_limit.map(f64::from), Some(25.0));
        assert_eq!(extra.used_credits.map(f64::from), Some(5.0));
        let wallet = result.wallet.unwrap();
        assert_eq!(wallet.currency, "USD");
        assert_eq!(wallet.remaining.map(f64::from), Some(0.0));
    }
}

#[tokio::test]
async fn usage_subscriptions_grok_rejects_invalid_input() {
    let home = tempfile::tempdir().unwrap();
    let store = store(home.path());
    let tenant = TenantScope::default();
    let credential = selection(scope(&tenant), false);
    for (auth, state) in [
        (
            json!({"https://auth.x.ai::test":{"key":"bad\r\nheader"}}),
            UsageCredentialState::ParseError,
        ),
        (
            json!({"https://auth.x.ai::test":{"key":"old-token","expires_at":"2030-01-01T00:00:00Z","refresh_token":"fixture-refresh"}}),
            UsageCredentialState::RefreshPending,
        ),
    ] {
        write_native(home.path(), auth);
        let result = query_at(
            &credential,
            home.path(),
            &store,
            client_builder().no_proxy(),
            BillingOrigins { proxy: "http://127.0.0.1:1", web: "http://127.0.0.1:1" },
            NOW,
            ID,
        )
        .await
        .unwrap();
        assert_eq!(result.subscription.queried_at, None);
        assert_eq!(result.subscription.credential_state, state);
        let UsageQueryOutcome::Failure { error, .. } = result.subscription.outcome else {
            panic!("expected credential rejection");
        };
        assert_eq!(error.code, UsageQueryErrorCode::CredentialUnavailable);
    }
}

#[tokio::test]
async fn usage_subscriptions_grok_preserves_partial_results_and_reasons() {
    let trailer = frame(0x80, b"grpc-status: 7\r\ngrpc-message: fixture-secret\r\n");
    let result = replay(false, ("200 OK", BILLING), ("200 OK", &trailer)).await;
    assert_eq!(subscription(&result).windows.len(), 2);
    assert!(result.wallet.is_some());
    assert_eq!(result.endpoint_errors.len(), 1);
    assert_eq!(result.endpoint_errors[0].code, UsageQueryErrorCode::UpstreamRejected);
    assert!(!result.endpoint_errors[0].message.contains("fixture-secret"));

    let result =
        replay(true, ("503 Service Unavailable", "fixture-secret"), ("200 OK", &grpc_usage()))
            .await;
    assert_eq!(subscription(&result).windows[0].utilization.map(f64::from), Some(12.5));
    assert!(result.wallet.is_none());
    assert_eq!(result.endpoint_errors.len(), 1);
    assert_eq!(result.endpoint_errors[0].code, UsageQueryErrorCode::UpstreamUnavailable);
    assert!(!result.endpoint_errors[0].message.contains("fixture-secret"));
}

#[tokio::test]
async fn usage_subscriptions_grok_classifies_total_auth_failure() {
    let trailer = frame(0x80, b"grpc-status: 16\r\ngrpc-message: fixture-secret\r\n");
    let result = replay(true, ("401 Unauthorized", "fixture-secret"), ("200 OK", &trailer)).await;
    assert_eq!(result.subscription.credential_state, UsageCredentialState::Expired);
    let UsageQueryOutcome::Failure { error, .. } = result.subscription.outcome else {
        panic!("expected authentication failure");
    };
    assert_eq!(error.code, UsageQueryErrorCode::UpstreamUnauthorized);
    assert_eq!(error.correlation_id, ID);
    assert_eq!(result.endpoint_errors.len(), 2);
    assert!(result.endpoint_errors.iter().all(|e| !e.message.contains("fixture-secret")));
}
