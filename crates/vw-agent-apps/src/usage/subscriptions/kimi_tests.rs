use super::kimi::query_at;
use crate::auth::{
    copilot_tests::oauth_server,
    credential_file::AccountCredential,
    kimi::KimiDeviceAuth,
    kimi_account_tests::{ACCOUNT, CONFIG, Fixture},
};
use reqwest::{Request, Url, header};
use serde_json::json;
use std::fs;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthRegion},
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryErrorCode, UsageQueryOutcome, UsageSource,
    },
};

const NOW: TimestampMs = TimestampMs(1_700_000_000_000);

fn route(request: &mut Request, origin: &Url) {
    let mut target = origin.clone();
    target.set_path(request.url().path());
    *request.url_mut() = target;
}

#[tokio::test]
async fn usage_subscriptions_kimi_returns_expected_result() {
    let fixture = Fixture::new();
    let target = fixture.target(AuthRegion::Global);
    let auth = KimiDeviceAuth::new().unwrap();
    let (refresh_url, refresh_sent) = oauth_server(
        "200 OK",
        "",
        json!({
            "access_token": "fixture-new-access",
            "refresh_token": "fixture-new-refresh",
            "expires_in": 900
        }),
    )
    .await;
    let (me_url, me_sent) = oauth_server("200 OK", "", json!({"user_id": 41})).await;
    let (quota_url, quota_sent) = oauth_server(
        "200 OK",
        "",
        json!({
            "limits": [{"detail": {"limit": "80", "remaining": 60}}],
            "usage": {"limit": 100, "remaining": "40", "resetTime": "2027-02-01T00:00:00Z"}
        }),
    )
    .await;

    let result = query_at(
        &auth,
        &fixture.credentials,
        fixture.target(AuthRegion::Global),
        Some(fixture.native()),
        NOW,
        "fixture",
        |request| match request.url().as_str() {
            "https://auth.kimi.ai/api/oauth/token" => {
                assert!(fixture.lock().is_dir());
                route(request, &refresh_url);
            }
            "https://api.kimi.ai/coding/v1/me" => {
                assert!(fixture.lock().is_dir());
                route(request, &me_url);
            }
            "https://api.kimi.ai/coding/v1/usages" => {
                assert!(request.headers()[header::AUTHORIZATION].is_sensitive());
                route(request, &quota_url);
            }
            _ => panic!("unexpected Kimi endpoint"),
        },
    )
    .await
    .unwrap();
    let sent = refresh_sent.await.unwrap();
    assert!(sent.starts_with("POST /api/oauth/token HTTP/1.1\r\n"));
    assert!(sent.contains("grant_type=refresh_token&refresh_token=fixture-expired-refresh"));
    let sent = me_sent.await.unwrap();
    assert!(sent.starts_with("GET /coding/v1/me HTTP/1.1\r\n"));
    assert!(sent.contains("\r\nauthorization: Bearer fixture-new-access\r\n"));
    let sent = quota_sent.await.unwrap();
    assert!(sent.starts_with("GET /coding/v1/usages HTTP/1.1\r\n"));
    assert!(sent.contains("\r\nauthorization: Bearer fixture-new-access\r\n"));
    assert!(sent.contains("\r\naccept: application/json\r\n"));
    assert_eq!(result.source, UsageSource::Official("kimi_oauth".into()));
    assert_eq!(result.credential_state, UsageCredentialState::Valid);
    assert_eq!(result.queried_at, Some(NOW));
    let UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(quota) } =
        result.outcome
    else {
        panic!("expected official quota");
    };
    assert_eq!(quota.windows.len(), 2);
    assert_eq!(quota.windows[0].name, "five_hour");
    assert_eq!(quota.windows[0].utilization.map(f64::from), Some(25.0));
    assert_eq!(quota.windows[1].name, "weekly_limit");
    assert_eq!(quota.windows[1].remaining.map(f64::from), Some(40.0));
    assert_eq!(quota.windows[1].resets_at.as_deref(), Some("2027-02-01T00:00:00Z"));
    assert_eq!(fs::read_to_string(fixture.config()).unwrap(), CONFIG);
    assert!(!fixture.lock().exists());
    let native: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.credential()).unwrap()).unwrap();
    assert_eq!(native["refresh_token"], "fixture-new-refresh");
    let Some(AccountCredential::KimiOauth(saved)) =
        fixture.credentials.read_managed_account(target.scope, ACCOUNT).unwrap()
    else {
        panic!("expected managed Kimi credential");
    };
    assert_eq!(saved.refresh_token, "fixture-new-refresh");
}

#[tokio::test]
async fn usage_subscriptions_kimi_rejects_invalid_input() {
    let fixture = Fixture::new();
    let auth = KimiDeviceAuth::new().unwrap();

    let wrong_region = query_at(
        &auth,
        &fixture.credentials,
        fixture.target(AuthRegion::MainlandCn),
        Some(fixture.native()),
        NOW,
        "fixture",
        |_| panic!("mismatched region must fail before egress"),
    )
    .await;
    assert!(matches!(wrong_region, Err(AuthErrorCode::IdentityMismatch)));

    fs::create_dir(fixture.lock()).unwrap();
    let locked = query_at(
        &auth,
        &fixture.credentials,
        fixture.target(AuthRegion::Global),
        Some(fixture.native()),
        NOW,
        "fixture",
        |_| panic!("a held native lock must prevent rotation"),
    )
    .await
    .unwrap();
    assert_eq!(locked.credential_state, UsageCredentialState::RefreshPending);
    assert!(fixture.lock().is_dir());
}

#[tokio::test]
async fn usage_subscriptions_kimi_queries_valid_managed_account_directly() {
    let fixture = Fixture::new();
    let target = fixture.target(AuthRegion::Global);
    let auth = KimiDeviceAuth::new().unwrap();
    let lease = fixture.credentials.lease_kimi(target.scope, ACCOUNT, false).unwrap();
    let AccountCredential::KimiOauth(mut saved) = lease.read().unwrap() else {
        panic!("expected managed Kimi credential");
    };
    saved.access_token = "fixture-current-access".into();
    saved.expires_at = TimestampMs(NOW.0 + 1);
    lease.save(&AccountCredential::KimiOauth(saved)).unwrap();
    drop(lease);

    let (quota_url, quota_sent) =
        oauth_server("401 Unauthorized", "", json!({"error": "fixture-current-access"})).await;
    let result = query_at(&auth, &fixture.credentials, target, None, NOW, "fixture", |request| {
        assert_eq!(request.url().as_str(), "https://api.kimi.ai/coding/v1/usages");
        route(request, &quota_url);
    })
    .await
    .unwrap();
    assert!(
        quota_sent.await.unwrap().contains("\r\nauthorization: Bearer fixture-current-access\r\n")
    );
    assert_eq!(result.credential_state, UsageCredentialState::Expired);
    let UsageQueryOutcome::Failure { error, .. } = &result.outcome else {
        panic!("expected credential failure");
    };
    assert_eq!(error.code, UsageQueryErrorCode::UpstreamUnauthorized);
    assert!(!serde_json::to_string(&result).unwrap().contains("fixture-current-access"));
}
