use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::AuthService,
    apps_usage::{UsageQueryErrorCode, UsageQueryKind, UsageQueryOutcome, UsageQueryResult},
    id::{ProjectId, ProjectScope},
};

use super::{
    binding::UsageAccount,
    cache::{UsageCache, UsageCacheKey, UsageCacheResult},
    subscriptions::codex_tests::replay_with_usage,
};

fn key() -> UsageCacheKey {
    UsageCacheKey {
        project: ProjectScope::for_default_tenant(ProjectId("project-a".into())).unwrap(),
        principal: "owner".into(),
        host: "fixture-host".into(),
        app: AppId::Codex,
        provider: "provider-a".into(),
        kind: UsageQueryKind::OfficialSubscription,
        account: Some((AuthService::CodexOauth, UsageAccount::Managed("fixture-managed".into()))),
        credential_version: 1,
        configuration_version: 1,
    }
}

async fn query(usage: Option<(&str, &str)>) -> UsageQueryResult {
    replay_with_usage(true, Some("fixture-workspace"), usage, ("200 OK", r#"{"credits":[]}"#)).await
}

async fn success() -> UsageQueryResult {
    let result =
        query(Some(("200 OK", r#"{"rate_limit":{"primary_window":{"used_percent":17}}}"#))).await;
    assert!(matches!(result.outcome, UsageQueryOutcome::Success { .. }));
    result
}

async fn failure(status: Option<&str>, expected: UsageQueryErrorCode) -> UsageQueryResult {
    let result = query(status.map(|status| (status, "invalid JSON"))).await;
    let UsageQueryOutcome::Failure { error, .. } = &result.outcome else {
        panic!("expected upstream failure");
    };
    assert_eq!(error.code, expected);
    result
}

#[tokio::test]
async fn usage_cache_returns_expected_result() {
    use UsageQueryErrorCode::*;

    let good = success().await;
    let limited = failure(Some("429 Too Many Requests"), UpstreamRateLimited).await;
    let unavailable = failure(Some("503 Service Unavailable"), UpstreamUnavailable).await;
    let transport = failure(None, Transport).await;

    let key = key();
    let start = Instant::now();
    let mut cache = UsageCache::default();
    assert_eq!(
        cache.record(&key, unavailable.clone(), start),
        UsageCacheResult::Current(unavailable.clone())
    );
    assert_eq!(cache.record(&key, good.clone(), start), UsageCacheResult::Current(good.clone()));

    for (failure, elapsed_ms) in
        [(transport, 1), (limited, 300_000), (unavailable.clone(), 599_999)]
    {
        assert_eq!(
            cache.record(&key, failure.clone(), start + Duration::from_millis(elapsed_ms)),
            UsageCacheResult::Stale { last_success: Arc::new(good.clone()), failure }
        );
    }
    assert_eq!(
        cache.snapshot(&key, start + Duration::from_millis(599_999)),
        Some(UsageCacheResult::Stale {
            last_success: Arc::new(good),
            failure: unavailable.clone(),
        })
    );
    assert_eq!(
        cache.snapshot(&key, start + Duration::from_secs(600)),
        Some(UsageCacheResult::Current(unavailable.clone()))
    );
    assert_eq!(
        cache.record(&key, unavailable.clone(), start + Duration::from_secs(600)),
        UsageCacheResult::Current(unavailable)
    );
}

#[tokio::test]
async fn usage_cache_rejects_invalid_input() {
    use UsageQueryErrorCode::*;

    let good = success().await;
    let unavailable = failure(Some("503 Service Unavailable"), UpstreamUnavailable).await;
    let unauthorized = failure(Some("401 Unauthorized"), UpstreamUnauthorized).await;
    let invalid_response = failure(Some("200 OK"), InvalidResponse).await;
    let key = key();
    let now = Instant::now();
    let mut cache = UsageCache::default();
    for failure in [unauthorized, invalid_response] {
        cache.record(&key, good.clone(), now);
        assert_eq!(cache.record(&key, failure.clone(), now), UsageCacheResult::Current(failure));
        assert_eq!(
            cache.record(&key, unavailable.clone(), now),
            UsageCacheResult::Current(unavailable.clone())
        );
    }
}

#[tokio::test]
async fn usage_cache_isolates_accounts_and_versions() {
    let good = success().await;
    let unavailable =
        failure(Some("503 Service Unavailable"), UsageQueryErrorCode::UpstreamUnavailable).await;
    let key = key();
    let now = Instant::now();
    let mut cache = UsageCache::default();
    cache.record(&key, good.clone(), now);

    for other in [
        UsageCacheKey {
            account: Some((AuthService::CodexOauth, UsageAccount::Managed("other".into()))),
            ..key.clone()
        },
        UsageCacheKey { credential_version: 2, ..key.clone() },
        UsageCacheKey { configuration_version: 2, ..key.clone() },
    ] {
        assert_eq!(
            cache.record(&other, unavailable.clone(), now),
            UsageCacheResult::Current(unavailable.clone())
        );
    }
    assert_eq!(
        cache.record(&key, unavailable.clone(), now),
        UsageCacheResult::Stale { last_success: Arc::new(good), failure: unavailable.clone() }
    );
    cache.invalidate(&key);
    assert_eq!(
        cache.record(&key, unavailable.clone(), now),
        UsageCacheResult::Current(unavailable)
    );
}
