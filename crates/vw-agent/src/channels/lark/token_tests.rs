use super::*;
use std::time::{Duration, Instant};

fn channel() -> LarkChannel {
    LarkChannel::new(
        "app-id".to_string(),
        "app-secret".to_string(),
        "verify".to_string(),
        None,
        vec!["*".to_string()],
        false,
    )
}

#[test]
fn extract_lark_response_code_reads_integer_code_only() {
    assert_eq!(extract_lark_response_code(&serde_json::json!({"code": 0})), Some(0));
    assert_eq!(extract_lark_response_code(&serde_json::json!({"code": 42})), Some(42));
    assert_eq!(extract_lark_response_code(&serde_json::json!({"code": "42"})), None);
    assert_eq!(extract_lark_response_code(&serde_json::json!({})), None);
}

#[test]
fn should_refresh_lark_tenant_token_handles_status_and_business_code() {
    assert!(should_refresh_lark_tenant_token(
        reqwest::StatusCode::UNAUTHORIZED,
        &serde_json::json!({"code": 0})
    ));
    assert!(should_refresh_lark_tenant_token(
        reqwest::StatusCode::OK,
        &serde_json::json!({"code": LARK_INVALID_ACCESS_TOKEN_CODE})
    ));
    assert!(!should_refresh_lark_tenant_token(
        reqwest::StatusCode::OK,
        &serde_json::json!({"code": 0})
    ));
}

#[test]
fn extract_lark_token_ttl_seconds_accepts_expire_variants_and_defaults() {
    assert_eq!(extract_lark_token_ttl_seconds(&serde_json::json!({"expire": 30})), 30);
    assert_eq!(extract_lark_token_ttl_seconds(&serde_json::json!({"expires_in": 45})), 45);
    assert_eq!(extract_lark_token_ttl_seconds(&serde_json::json!({"expire": -5})), 1);
    assert_eq!(
        extract_lark_token_ttl_seconds(&serde_json::json!({"expires_in": "60"})),
        LARK_DEFAULT_TOKEN_TTL.as_secs()
    );
    assert_eq!(extract_lark_token_ttl_seconds(&serde_json::json!({"expire": 0})), 1);
    assert_eq!(
        extract_lark_token_ttl_seconds(&serde_json::json!({})),
        LARK_DEFAULT_TOKEN_TTL.as_secs()
    );
}

#[test]
fn next_token_refresh_deadline_uses_skew_or_one_second_floor() {
    let now = Instant::now();

    assert_eq!(
        next_token_refresh_deadline(now, 7200).duration_since(now),
        Duration::from_secs(7200).checked_sub(LARK_TOKEN_REFRESH_SKEW).unwrap()
    );
    assert_eq!(next_token_refresh_deadline(now, 0).duration_since(now), Duration::from_secs(1));
}

#[test]
fn ensure_lark_send_success_rejects_status_and_business_errors() {
    assert!(
        ensure_lark_send_success(reqwest::StatusCode::OK, &serde_json::json!({"code": 0}), "ctx")
            .is_ok()
    );
    assert!(
        ensure_lark_send_success(
            reqwest::StatusCode::BAD_REQUEST,
            &serde_json::json!({"msg": "bad"}),
            "ctx"
        )
        .is_err()
    );
    assert!(
        ensure_lark_send_success(reqwest::StatusCode::OK, &serde_json::json!({"code": 1}), "ctx")
            .is_err()
    );
}

#[test]
fn ensure_lark_send_success_defaults_missing_code_to_success() {
    assert!(
        ensure_lark_send_success(reqwest::StatusCode::CREATED, &serde_json::json!({}), "ctx")
            .is_ok()
    );
}

#[test]
fn sanitize_lark_body_returns_safe_string_for_logging() {
    let sanitized = sanitize_lark_body(&serde_json::json!({
        "tenant_access_token": "sk-testSECRET1234567890",
        "msg": "bad"
    }));

    assert!(!sanitized.is_empty());
    assert!(!sanitized.contains("sk-testSECRET1234567890"));
}

#[tokio::test]
async fn resolved_bot_open_id_and_token_cache_can_be_set_and_invalidated() {
    let ch = channel();

    assert_eq!(ch.resolved_bot_open_id(), None);
    ch.set_resolved_bot_open_id(Some("ou_bot".to_string()));
    assert_eq!(ch.resolved_bot_open_id().as_deref(), Some("ou_bot"));
    ch.set_resolved_bot_open_id(None);
    assert_eq!(ch.resolved_bot_open_id(), None);

    {
        let mut cached = ch.tenant_token.write().await;
        *cached = Some(CachedTenantToken {
            value: "cached".to_string(),
            refresh_after: Instant::now() + Duration::from_secs(60),
        });
    }

    ch.invalidate_token().await;

    assert!(ch.tenant_token.read().await.is_none());
}

#[test]
fn resolved_bot_open_id_ignores_poisoned_cache_lock() {
    let ch = channel();
    let lock = std::sync::Arc::clone(&ch.resolved_bot_open_id);

    let panic_result = std::thread::spawn(move || {
        let _guard = lock.write().expect("cache lock should initially be available");
        panic!("poison bot open id cache for test");
    })
    .join();

    assert!(panic_result.is_err());
    assert_eq!(ch.resolved_bot_open_id(), None);
    ch.set_resolved_bot_open_id(Some("ou_ignored".to_string()));
    assert_eq!(ch.resolved_bot_open_id(), None);
}

#[path = "token_bot_tests.rs"]
mod token_bot_tests;
#[path = "token_http_tests.rs"]
mod token_http_tests;
#[path = "token_test_support.rs"]
mod token_test_support;
// 本文件验证 Lark 令牌缓存、过期计算、响应判定与日志脱敏。
