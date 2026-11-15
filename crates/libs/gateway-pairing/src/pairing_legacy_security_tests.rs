use super::{
    constant_time_eq, generate_code, generate_token, hash_token, is_public_bind, is_token_hash,
    PairingGuard, MAX_PAIR_ATTEMPTS, MAX_TRACKED_CLIENTS, PAIR_LOCKOUT_SECS,
    FAILED_ATTEMPT_RETENTION_SECS, FAILED_ATTEMPT_SWEEP_INTERVAL_SECS, FailedAttemptState,
};
use std::time::Instant;

// 测试当没有已存在 token 时，新 Guard 会生成配对码
#[tokio::test]
async fn new_guard_generates_code_when_no_tokens() {
    let guard = PairingGuard::new(true, &[]);
    assert!(guard.pairing_code().is_some());
    assert!(!guard.is_paired());
}

// 测试当已存在 token 时，新 Guard 不会生成配对码
#[tokio::test]
async fn new_guard_no_code_when_tokens_exist() {
    let guard = PairingGuard::new(true, &["zc_existing".into()]);
    assert!(guard.pairing_code().is_none());
    assert!(guard.is_paired());
}

#[tokio::test]
async fn ensure_pairing_code_regenerates_code_when_tokens_exist() {
    let guard = PairingGuard::new(true, &["zc_existing".into()]);

    let code = guard.ensure_pairing_code();

    assert!(code.is_some());
    assert_eq!(guard.pairing_code(), code);
}

// 测试当配对功能禁用时，不会生成配对码
#[tokio::test]
async fn new_guard_no_code_when_pairing_disabled() {
    let guard = PairingGuard::new(false, &[]);
    assert!(guard.pairing_code().is_none());
}

// 测试使用正确的配对码进行配对
#[tokio::test]
async fn try_pair_correct_code() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap().to_string();
    let token = guard.try_pair(&code, "test_client").await.unwrap();
    assert!(token.is_some());
    assert!(token.unwrap().starts_with("zc_"));
    assert!(guard.is_paired());
}

// 测试使用错误的配对码进行配对
#[tokio::test]
async fn try_pair_wrong_code() {
    let guard = PairingGuard::new(true, &[]);
    let result = guard.try_pair("000000", "test_client").await.unwrap();
    let _ = result;
}

// 测试使用空配对码进行配对
#[tokio::test]
async fn try_pair_empty_code() {
    let guard = PairingGuard::new(true, &[]);
    assert!(guard.try_pair("", "test_client").await.unwrap().is_none());
}

// 测试使用有效 token 进行身份验证
#[tokio::test]
async fn is_authenticated_with_valid_token() {
    let guard = PairingGuard::new(true, &["zc_valid".into()]);
    assert!(guard.is_authenticated("zc_valid"));
}

// 测试使用预哈希的 token 进行身份验证
#[tokio::test]
async fn is_authenticated_with_prehashed_token() {
    let hashed = hash_token("zc_valid");
    let guard = PairingGuard::new(true, &[hashed]);
    assert!(guard.is_authenticated("zc_valid"));
}

// 测试使用无效 token 进行身份验证
#[tokio::test]
async fn is_authenticated_with_invalid_token() {
    let guard = PairingGuard::new(true, &["zc_valid".into()]);
    assert!(!guard.is_authenticated("zc_invalid"));
}

// 测试当配对功能禁用时的身份验证
#[tokio::test]
async fn is_authenticated_when_pairing_disabled() {
    let guard = PairingGuard::new(false, &[]);
    assert!(guard.is_authenticated("anything"));
    assert!(guard.is_authenticated(""));
}

// 测试 tokens() 方法返回哈希值而非明文
#[tokio::test]
async fn tokens_returns_hashes() {
    let guard = PairingGuard::new(true, &["zc_a".into(), "zc_b".into()]);
    let tokens = guard.tokens();
    assert_eq!(tokens.len(), 2);
    for t in &tokens {
        assert_eq!(t.len(), 64, "Token should be a SHA-256 hash");
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(!t.starts_with("zc_"), "Token should not be plaintext");
    }
}

// 测试配对后立即进行身份验证
#[tokio::test]
async fn pair_then_authenticate() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap().to_string();
    let token = guard.try_pair(&code, "test_client").await.unwrap().unwrap();
    assert!(guard.is_authenticated(&token));
    assert!(!guard.is_authenticated("wrong"));
}

// 测试哈希函数生成 64 位十六进制字符
#[test]
fn hash_token_produces_64_hex_chars() {
    let hash = hash_token("zc_test_token");
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

// 测试哈希函数的确定性
#[test]
fn hash_token_is_deterministic() {
    assert_eq!(hash_token("zc_abc"), hash_token("zc_abc"));
}

// 测试不同输入产生不同哈希值
#[test]
fn hash_token_differs_for_different_inputs() {
    assert_ne!(hash_token("zc_a"), hash_token("zc_b"));
}

// 测试检测 token 是哈希值还是明文
#[test]
fn is_token_hash_detects_hash_vs_plaintext() {
    assert!(is_token_hash(&hash_token("zc_test")));
    assert!(!is_token_hash("zc_test_token"));
    assert!(!is_token_hash("too_short"));
    assert!(!is_token_hash(""));
}

// 测试本地地址不被视为公共绑定地址
#[test]
fn localhost_variants_not_public() {
    assert!(!is_public_bind("127.0.0.1"));
    assert!(!is_public_bind("localhost"));
    assert!(!is_public_bind("::1"));
    assert!(!is_public_bind("[::1]"));
}

// 测试 0.0.0.0 被视为公共绑定地址
#[test]
fn zero_zero_is_public() {
    assert!(is_public_bind("0.0.0.0"));
}

// 测试真实 IP 地址被视为公共绑定地址
#[test]
fn real_ip_is_public() {
    assert!(is_public_bind("192.168.1.100"));
    assert!(is_public_bind("10.0.0.1"));
}

// 测试常量时间比较函数对相同字符串返回 true
#[test]
fn constant_time_eq_same() {
    assert!(constant_time_eq("abc", "abc"));
    assert!(constant_time_eq("", ""));
}

// 测试常量时间比较函数对不同字符串返回 false
#[test]
fn constant_time_eq_different() {
    assert!(!constant_time_eq("abc", "abd"));
    assert!(!constant_time_eq("abc", "ab"));
    assert!(!constant_time_eq("a", ""));
}

// 测试配对码生成器生成 6 位数字
#[test]
fn generate_code_is_6_digits() {
    let code = generate_code();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));
}

// 测试配对码生成器生成的码不是确定性的
#[test]
fn generate_code_is_not_deterministic() {
    for _ in 0..10 {
        if generate_code() != generate_code() {
            return;
        }
    }
    panic!("Generated 10 pairs of codes and all were collisions — CSPRNG failure");
}

// 测试生成的 token 具有前缀和十六进制载荷
#[test]
fn generate_token_has_prefix_and_hex_payload() {
    let token = generate_token();
    let payload = token.strip_prefix("zc_").expect("Generated token should include zc_ prefix");

    assert_eq!(payload.len(), 64, "Token payload should be 32 bytes in hex");
    assert!(
        payload.chars().all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f')),
        "Token payload should be lowercase hex"
    );
}

// 测试在最大尝试次数后触发暴力破解锁定
#[tokio::test]
async fn brute_force_lockout_after_max_attempts() {
    let guard = PairingGuard::new(true, &[]);
    let client = "attacker_client";
    for i in 0..MAX_PAIR_ATTEMPTS {
        let result = guard.try_pair(&format!("wrong_{i}"), client).await;
        assert!(result.is_ok(), "Attempt {i} should not be locked out yet");
    }
    let result = guard.try_pair("another_wrong", client).await;
    assert!(result.is_err(), "Should be locked out after {MAX_PAIR_ATTEMPTS} attempts");
    let lockout_secs = result.unwrap_err();
    assert!(lockout_secs > 0, "Lockout should have remaining seconds");
    assert!(lockout_secs <= PAIR_LOCKOUT_SECS, "Lockout should not exceed max");
}

// 测试正确配对码会重置失败尝试计数
#[tokio::test]
async fn correct_code_resets_failed_attempts() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap().to_string();
    let client = "test_client";
    for _ in 0..3 {
        let _ = guard.try_pair("wrong", client).await;
    }
    let result = guard.try_pair(&code, client).await.unwrap();
    assert!(result.is_some(), "Correct code should work before lockout");
}

// 测试锁定状态返回剩余秒数
#[tokio::test]
async fn lockout_returns_remaining_seconds() {
    let guard = PairingGuard::new(true, &[]);
    let client = "test_client";
    for _ in 0..MAX_PAIR_ATTEMPTS {
        let _ = guard.try_pair("wrong", client).await;
    }
    let err = guard.try_pair("wrong", client).await.unwrap_err();
    assert!(
        err >= PAIR_LOCKOUT_SECS - 1,
        "Remaining lockout should be ~{PAIR_LOCKOUT_SECS}s, got {err}s"
    );
}

// 测试成功配对只重置请求客户端的状态
#[tokio::test]
async fn successful_pair_resets_only_requesting_client_state() {
    let guard = PairingGuard::new(true, &[]);
    let code = guard.pairing_code().unwrap().to_string();
    let client_a = "client_a";
    let client_b = "client_b";

    for _ in 0..3 {
        let _ = guard.try_pair("wrong", client_a).await;
        let _ = guard.try_pair("wrong", client_b).await;
    }

    let result = guard.try_pair(&code, client_a).await.unwrap();
    assert!(result.is_some(), "client_a should pair successfully");

    let state = guard.failed_attempts.lock();
    let b_state = state.0.get(client_b);
    assert!(b_state.is_some(), "client_b state should still exist");
    assert_eq!(b_state.unwrap().count, 3, "client_b should still have 3 failures");

    assert!(!state.0.contains_key(client_a), "client_a state should be cleared");
}

// 测试失败尝试状态受最大客户端数量限制
#[tokio::test]
async fn failed_attempt_state_is_bounded_by_max_clients() {
    let guard = PairingGuard::new(true, &[]);

    {
        let mut state = guard.failed_attempts.lock();
        let past = Instant::now()
            .checked_sub(std::time::Duration::from_secs(FAILED_ATTEMPT_RETENTION_SECS + 60))
            .unwrap_or_else(Instant::now);
        for i in 0..MAX_TRACKED_CLIENTS {
            state.0.insert(
                format!("stale_client_{i}"),
                FailedAttemptState { count: 1, lockout_until: None, last_attempt: past },
            );
        }
    }

    let result = guard.try_pair("wrong", "new_client").await;
    assert!(result.is_ok(), "New client should not be blocked");

    let state = guard.failed_attempts.lock();
    assert!(
        state.0.len() <= MAX_TRACKED_CLIENTS,
        "Map size should stay within bound, got {}",
        state.0.len()
    );
    assert!(state.0.contains_key("new_client"), "New client should be tracked");
}

// 测试定期清理过期客户端
#[tokio::test]
async fn failed_attempt_sweep_prunes_expired_clients() {
    let guard = PairingGuard::new(true, &[]);

    {
        let mut state = guard.failed_attempts.lock();
        let past = Instant::now()
            .checked_sub(std::time::Duration::from_secs(FAILED_ATTEMPT_RETENTION_SECS + 60))
            .unwrap_or_else(Instant::now);
        state.0.insert(
            "stale_client".to_string(),
            FailedAttemptState { count: 2, lockout_until: None, last_attempt: past },
        );
        state.1 = Instant::now()
            .checked_sub(std::time::Duration::from_secs(FAILED_ATTEMPT_SWEEP_INTERVAL_SECS + 1))
            .unwrap_or_else(Instant::now);
    }

    let _ = guard.try_pair("wrong", "fresh_client").await;

    let state = guard.failed_attempts.lock();
    assert!(
        !state.0.contains_key("stale_client"),
        "Stale client should have been pruned by sweep"
    );
    assert!(state.0.contains_key("fresh_client"), "Fresh client should still be tracked");
}

// 测试锁定是按客户端隔离的
#[tokio::test]
async fn lockout_is_per_client() {
    let guard = PairingGuard::new(true, &[]);
    let attacker = "attacker_ip";
    let legitimate = "legitimate_ip";

    for i in 0..MAX_PAIR_ATTEMPTS {
        let _ = guard.try_pair(&format!("wrong_{i}"), attacker).await;
    }
    assert!(guard.try_pair("wrong", attacker).await.is_err());

    let result = guard.try_pair("wrong", legitimate).await;
    assert!(result.is_ok(), "Legitimate client should not be locked out by attacker");
}
