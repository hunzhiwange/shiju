use super::*;
use std::time::Duration;

fn instant_before_now(age: Duration) -> std::time::Instant {
    std::time::Instant::now().checked_sub(age).expect("age before now")
}

fn single_timestamp(limiter: &SlidingWindowRateLimiter, key: &str) -> std::time::Instant {
    let guard = limiter.requests.lock();
    guard.0.get(key).and_then(|timestamps| timestamps.first()).copied().expect("timestamp")
}

#[test]
fn sliding_window_rate_limiter_prune_stale_removes_empty_and_cutoff_entries() {
    let cutoff = std::time::Instant::now();
    let mut requests = std::collections::HashMap::new();
    requests.insert("empty".to_string(), Vec::new());
    requests.insert("old".to_string(), vec![cutoff]);
    requests.insert(
        "mixed".to_string(),
        vec![cutoff, cutoff.checked_add(Duration::from_secs(1)).expect("future")],
    );

    SlidingWindowRateLimiter::prune_stale(&mut requests, cutoff);

    assert!(!requests.contains_key("empty"));
    assert!(!requests.contains_key("old"));
    assert_eq!(requests.get("mixed").expect("mixed").len(), 1);
}

#[test]
fn sliding_window_rate_limiter_rejects_after_limit() {
    let limiter = SlidingWindowRateLimiter::new(1, Duration::from_secs(60), 10);

    assert!(limiter.allow("client"));
    assert!(!limiter.allow("client"));
}

#[test]
fn sliding_window_rate_limiter_zero_limit_allows_requests() {
    let limiter = SlidingWindowRateLimiter::new(0, Duration::from_secs(60), 1);

    assert!(limiter.allow("client"));
    assert!(limiter.allow("client"));
}

#[test]
fn sliding_window_rate_limiter_allows_when_window_is_too_large_for_checked_sub() {
    let limiter = SlidingWindowRateLimiter::new(1, Duration::MAX, 10);

    assert!(limiter.allow("client"));
    assert!(limiter.allow("client"));
}

#[test]
fn sliding_window_rate_limiter_tracks_keys_independently() {
    let limiter = SlidingWindowRateLimiter::new(1, Duration::from_secs(60), 10);

    assert!(limiter.allow("client-a"));
    assert!(limiter.allow("client-b"));
    assert!(!limiter.allow("client-a"));
    assert!(!limiter.allow("client-b"));
}

#[test]
fn sliding_window_rate_limiter_reallows_after_window_expires() {
    let limiter = SlidingWindowRateLimiter::new(1, Duration::from_secs(1), 10);
    assert!(limiter.allow("client"));

    {
        let mut guard = limiter.requests.lock();
        guard.0.insert("client".to_string(), vec![instant_before_now(Duration::from_secs(2))]);
    }

    assert!(limiter.allow("client"));
    let guard = limiter.requests.lock();
    assert_eq!(guard.0.get("client").expect("client").len(), 1);
}

#[test]
fn sliding_window_rate_limiter_zero_window_expires_previous_request_immediately() {
    let limiter = SlidingWindowRateLimiter::new(1, Duration::ZERO, 10);

    assert!(limiter.allow("client"));
    assert!(limiter.allow("client"));

    let guard = limiter.requests.lock();
    assert_eq!(guard.0.get("client").expect("client").len(), 1);
}

#[test]
fn sliding_window_rate_limiter_sweeps_stale_keys_on_interval() {
    let limiter = SlidingWindowRateLimiter::new(2, Duration::from_secs(1), 10);

    {
        let mut guard = limiter.requests.lock();
        guard.0.insert("stale".to_string(), vec![instant_before_now(Duration::from_secs(2))]);
        guard.1 = instant_before_now(Duration::from_secs(RATE_LIMITER_SWEEP_INTERVAL_SECS + 1));
    }

    assert!(limiter.allow("fresh"));
    let guard = limiter.requests.lock();
    assert!(!guard.0.contains_key("stale"));
    assert!(guard.0.contains_key("fresh"));
}

#[test]
fn sliding_window_rate_limiter_prunes_stale_key_before_evicting_active_key() {
    let limiter = SlidingWindowRateLimiter::new(2, Duration::from_secs(1), 2);

    {
        let mut guard = limiter.requests.lock();
        guard.0.insert("stale".to_string(), vec![instant_before_now(Duration::from_secs(2))]);
        guard.0.insert("active".to_string(), vec![std::time::Instant::now()]);
    }

    assert!(limiter.allow("new"));
    let guard = limiter.requests.lock();
    assert!(!guard.0.contains_key("stale"));
    assert!(guard.0.contains_key("active"));
    assert!(guard.0.contains_key("new"));
}

#[test]
fn sliding_window_rate_limiter_evicts_least_recent_key_when_full() {
    let limiter = SlidingWindowRateLimiter::new(2, Duration::from_secs(60), 2);

    {
        let mut guard = limiter.requests.lock();
        guard.0.insert("oldest".to_string(), vec![instant_before_now(Duration::from_secs(30))]);
        guard.0.insert("newest".to_string(), vec![instant_before_now(Duration::from_secs(10))]);
    }

    assert!(limiter.allow("incoming"));
    let guard = limiter.requests.lock();
    assert!(!guard.0.contains_key("oldest"));
    assert!(guard.0.contains_key("newest"));
    assert!(guard.0.contains_key("incoming"));
}

#[test]
fn sliding_window_rate_limiter_clamps_zero_max_keys_to_one() {
    let limiter = SlidingWindowRateLimiter::new(2, Duration::from_secs(60), 0);

    assert!(limiter.allow("first"));
    let first_seen_at = single_timestamp(&limiter, "first");
    {
        let mut guard = limiter.requests.lock();
        guard.0.insert("first".to_string(), vec![first_seen_at]);
    }

    assert!(limiter.allow("second"));
    let guard = limiter.requests.lock();
    assert_eq!(guard.0.len(), 1);
    assert!(guard.0.contains_key("second"));
}

#[test]
fn gateway_rate_limiter_uses_separate_pair_and_webhook_limits() {
    let limiter = GatewayRateLimiter::new(1, 2, 10);

    assert!(limiter.allow_pair("client"));
    assert!(!limiter.allow_pair("client"));

    assert!(limiter.allow_webhook("client"));
    assert!(limiter.allow_webhook("client"));
    assert!(!limiter.allow_webhook("client"));
}

#[test]
fn idempotency_store_records_only_new_keys_within_ttl() {
    let store = IdempotencyStore::new(Duration::from_secs(60), 10);

    assert!(store.record_if_new("request"));
    assert!(!store.record_if_new("request"));
    assert!(store.record_if_new("other"));
}

#[test]
fn idempotency_store_reallows_key_after_ttl_expires() {
    let store = IdempotencyStore::new(Duration::from_secs(1), 10);
    assert!(store.record_if_new("request"));

    {
        let mut keys = store.keys.lock();
        keys.insert("request".to_string(), instant_before_now(Duration::from_secs(2)));
    }

    assert!(store.record_if_new("request"));
}

#[test]
fn idempotency_store_zero_ttl_expires_keys_immediately() {
    let store = IdempotencyStore::new(Duration::ZERO, 10);

    assert!(store.record_if_new("request"));
    assert!(store.record_if_new("request"));
}

#[test]
fn idempotency_store_prunes_expired_key_before_capacity_evicts_active_key() {
    let store = IdempotencyStore::new(Duration::from_secs(60), 2);

    {
        let mut keys = store.keys.lock();
        keys.insert("expired".to_string(), instant_before_now(Duration::from_secs(61)));
        keys.insert("active".to_string(), std::time::Instant::now());
    }

    assert!(store.record_if_new("incoming"));
    let keys = store.keys.lock();
    assert!(!keys.contains_key("expired"));
    assert!(keys.contains_key("active"));
    assert!(keys.contains_key("incoming"));
}

#[test]
fn idempotency_store_evicts_oldest_key_when_full() {
    let store = IdempotencyStore::new(Duration::from_secs(60), 2);

    {
        let mut keys = store.keys.lock();
        keys.insert("oldest".to_string(), instant_before_now(Duration::from_secs(30)));
        keys.insert("newest".to_string(), instant_before_now(Duration::from_secs(10)));
    }

    assert!(store.record_if_new("incoming"));
    let keys = store.keys.lock();
    assert!(!keys.contains_key("oldest"));
    assert!(keys.contains_key("newest"));
    assert!(keys.contains_key("incoming"));
}

#[test]
fn idempotency_store_clamps_zero_max_keys_to_one() {
    let store = IdempotencyStore::new(Duration::from_secs(60), 0);

    assert!(store.record_if_new("first"));
    assert!(store.record_if_new("second"));

    let keys = store.keys.lock();
    assert_eq!(keys.len(), 1);
    assert!(keys.contains_key("second"));
}

#[test]
fn idempotency_store_allows_evicted_key_as_new_request() {
    let store = IdempotencyStore::new(Duration::from_secs(60), 1);

    assert!(store.record_if_new("first"));
    assert!(store.record_if_new("second"));
    assert!(store.record_if_new("first"));

    let keys = store.keys.lock();
    assert_eq!(keys.len(), 1);
    assert!(keys.contains_key("first"));
}
