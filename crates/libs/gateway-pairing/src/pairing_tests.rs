//! 验证配对码、令牌哈希、失败锁定与网关密钥认证。

use super::*;
use vw_config_types::gateway::GatewaySkey;

#[test]
// 认证比较使用恒定时间路径，避免由提前返回泄露匹配前缀。
fn constant_time_eq_matches_equal_strings_only() {
    assert!(constant_time_eq("same", "same"));
    assert!(!constant_time_eq("same", "diff"));
}

#[test]
fn public_bind_detection_is_explicit() {
    assert!(is_public_bind("0.0.0.0"));
    assert!(!is_public_bind("127.0.0.1"));
}

#[test]
fn disabled_pairing_authenticates_without_code_or_token() {
    let guard = PairingGuard::new(false, &[]);

    assert!(!guard.require_pairing());
    assert!(guard.pairing_code().is_none());
    assert!(guard.ensure_pairing_code().is_none());
    assert!(guard.is_authenticated("anything"));
    assert!(!guard.is_paired());
}

#[tokio::test]
async fn successful_pairing_consumes_code_and_stores_token_hash() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().expect("pairing code");

    let token = guard
        .try_pair(&format!(" {code}\n"), " client-a ")
        .await
        .expect("not locked out")
        .expect("token");

    assert!(token.starts_with("zc_"));
    assert_eq!(token.len(), 67);
    assert!(guard.pairing_code().is_none());
    assert!(guard.is_paired());
    assert!(guard.is_authenticated(&token));
    assert!(!guard.is_authenticated("zc_wrong"));

    let tokens = guard.tokens();
    assert_eq!(tokens.len(), 1);
    assert!(tokens[0].chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_eq!(tokens[0].len(), 64);
}

#[tokio::test]
// 失败计数按客户端隔离，攻击者不能借一个来源锁死其他客户端。
async fn failed_attempts_lock_out_client_but_not_other_clients() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().expect("pairing code");

    for _ in 0..5 {
        assert!(guard.try_pair("000000", "client-a").await.unwrap().is_none());
    }

    let lockout = guard.try_pair(&code, "client-a").await.unwrap_err();
    assert!(lockout > 0);
    assert!(guard.try_pair(&code, "client-b").await.unwrap().is_some());
}

#[test]
fn existing_plain_tokens_are_hashed_and_existing_hashes_are_preserved() {
    let plain = "zc_existing_plain";
    let hashed = hash_token("zc_existing_hashed");
    let guard = PairingGuard::new(true, &[plain.to_string(), hashed.clone()]);

    assert!(guard.pairing_code().is_none());
    assert!(guard.is_authenticated(plain));
    assert!(guard.is_authenticated("zc_existing_hashed"));

    let tokens = guard.tokens();
    assert!(tokens.contains(&hash_token(plain)));
    assert!(tokens.contains(&hashed));
}

#[test]
// 配置密钥同时受启用状态和过期时间约束。
fn configured_skeys_respect_enabled_flag_and_expiration() {
    let enabled = GatewaySkey {
        enabled: true,
        skey: Some("enabled-skey".to_string()),
        skey_hash: String::new(),
        masked_skey: String::new(),
        name: "enabled".to_string(),
        expires_at: None,
    };
    let disabled = GatewaySkey {
        enabled: false,
        skey: Some("disabled-skey".to_string()),
        skey_hash: String::new(),
        masked_skey: String::new(),
        name: "disabled".to_string(),
        expires_at: None,
    };
    let expired = GatewaySkey {
        enabled: true,
        skey: Some("expired-skey".to_string()),
        skey_hash: String::new(),
        masked_skey: String::new(),
        name: "expired".to_string(),
        expires_at: Some("2000-01-01T00:00:00Z".to_string()),
    };
    let guard = PairingGuard::from_skeys(true, &[enabled, disabled, expired]);

    assert_eq!(guard.active_skey_count(), 1);
    assert!(guard.is_authenticated("enabled-skey"));
    assert!(!guard.is_authenticated("disabled-skey"));
    assert!(!guard.is_authenticated("expired-skey"));
}

#[test]
fn skey_config_updates_apply_to_existing_guard() {
    let guard = PairingGuard::from_skeys(false, &[]);
    assert!(!guard.auth_enabled());
    assert!(guard.is_authenticated(""));

    let enabled = GatewaySkey {
        enabled: true,
        skey: Some("hot-skey".to_string()),
        skey_hash: String::new(),
        masked_skey: String::new(),
        name: "hot".to_string(),
        expires_at: None,
    };

    guard.update_from_skeys(true, &[enabled]);
    assert!(guard.auth_enabled());
    assert!(guard.is_authenticated("hot-skey"));
    assert!(!guard.is_authenticated("wrong-skey"));

    guard.update_from_skeys(false, &[]);
    assert!(!guard.auth_enabled());
    assert!(guard.is_authenticated(""));
}

#[test]
fn ensure_pairing_code_regenerates_for_paired_guard() {
    let guard = PairingGuard::new(true, &[hash_token("existing")]);

    assert!(guard.pairing_code().is_none());
    let code = guard.ensure_pairing_code().expect("bootstrap code");
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|ch| ch.is_ascii_digit()));
    assert_eq!(guard.ensure_pairing_code(), Some(code));
}

#[test]
fn helper_functions_cover_edge_cases() {
    assert_eq!(normalize_client_key("  "), "unknown");
    assert_eq!(normalize_client_key("  ip-1 "), "ip-1");

    let token = generate_token();
    assert!(token.starts_with("zc_"));
    assert_eq!(token.len(), 67);
    assert!(is_token_hash(&hash_token(&token)));
    assert!(!is_token_hash("not-a-hash"));

    let code = generate_code();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|ch| ch.is_ascii_digit()));

    assert!(!constant_time_eq("same", "same-but-longer"));
    assert!(!is_public_bind("localhost"));
    assert!(!is_public_bind("::1"));
    assert!(is_public_bind("192.168.1.10"));
}

#[test]
fn configured_skeys_reject_invalid_hashes_and_expiration_timestamps() {
    let invalid_hash = GatewaySkey {
        enabled: true,
        skey: None,
        skey_hash: "not-a-sha256-hash".to_string(),
        masked_skey: String::new(),
        name: "invalid-hash".to_string(),
        expires_at: None,
    };
    let invalid_expiration = GatewaySkey {
        enabled: true,
        skey: Some("valid-skey".to_string()),
        skey_hash: String::new(),
        masked_skey: String::new(),
        name: "invalid-expiration".to_string(),
        expires_at: Some("not-rfc3339".to_string()),
    };

    let guard = PairingGuard::from_skeys(true, &[invalid_hash, invalid_expiration]);

    assert_eq!(guard.active_skey_count(), 0);
    assert!(!guard.is_authenticated("valid-skey"));
}

#[test]
fn skey_helpers_trim_hash_input_and_mask_boundary_lengths() {
    assert_eq!(hash_skey("  secret  "), hash_token("secret"));
    assert_eq!(masked_skey_name("   "), "");
    assert_eq!(masked_skey_name("short"), "*****");
    assert_eq!(masked_skey_name("12345678"), "********");
    assert_eq!(masked_skey_name("123456789"), "1234***6789");
    assert_eq!(masked_skey_name(" 密钥-123456789 "), "密钥-1***6789");
}

#[test]
fn failed_attempt_pruning_removes_only_stale_clients() {
    let now = Instant::now();
    let mut attempts = HashMap::from([
        (
            "stale".to_string(),
            FailedAttemptState {
                count: 1,
                lockout_until: None,
                last_attempt: now
                    - std::time::Duration::from_secs(FAILED_ATTEMPT_RETENTION_SECS + 1),
            },
        ),
        (
            "fresh".to_string(),
            FailedAttemptState { count: 2, lockout_until: None, last_attempt: now },
        ),
    ]);

    prune_failed_attempts(&mut attempts, now);

    assert!(!attempts.contains_key("stale"));
    assert!(attempts.contains_key("fresh"));
}
