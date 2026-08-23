//! 带类型前缀 ID 的生成、解析与错误边界测试。
//!
//! 共享状态相关用例会串行执行，并覆盖同毫秒计数、降序编码和随机源失败。

use super::{
    Error, FORCE_RANDOM_ERROR, Prefix, STATE, ascending, create, descending, random_base62, schema,
    timestamp,
};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

static STATE_TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn reset_state() {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    state.last_timestamp_ms = 0;
    state.counter = 0;
}

fn assert_id_shape(id: &str, prefix: Prefix) {
    let expected_prefix = format!("{}_", prefix.as_str());
    assert!(id.starts_with(&expected_prefix));
    assert_eq!(id.len(), expected_prefix.len() + 26);

    let body = &id[expected_prefix.len()..];
    assert!(body[..12].chars().all(|ch| ch.is_ascii_hexdigit()));
    assert!(body[12..].chars().all(|ch| ch.is_ascii_alphanumeric()));
}

#[test]
fn prefix_as_str_returns_stable_public_keys() {
    let cases = [
        (Prefix::Session, "ses"),
        (Prefix::Message, "msg"),
        (Prefix::Permission, "per"),
        (Prefix::Question, "que"),
        (Prefix::User, "usr"),
        (Prefix::Part, "prt"),
        (Prefix::Pty, "pty"),
        (Prefix::Tool, "tool"),
    ];

    for (prefix, expected) in cases {
        assert_eq!(prefix.as_str(), expected);
    }
}

#[test]
fn error_display_includes_prefix_mismatch_context() {
    let err =
        Error::PrefixMismatch { id: "msg_000000000001abc".to_string(), expected_prefix: "ses" };

    assert_eq!(err.to_string(), "ID msg_000000000001abc does not start with ses");
}

#[test]
fn error_display_returns_random_message() {
    let err = Error::Random("entropy unavailable".to_string());

    assert_eq!(err.to_string(), "entropy unavailable");
}

#[test]
fn schema_checks_the_expected_prefix_start() {
    assert!(schema(Prefix::Session, "ses_000000000001abcdefabcdefab"));
    assert!(schema(Prefix::Session, "sesame"));
    assert!(!schema(Prefix::Message, "ses_000000000001abcdefabcdefab"));
}

#[test]
fn ascending_returns_given_id_when_prefix_matches() {
    let given = "ses_000000000001abcdefabcdefab";

    let id = ascending(Prefix::Session, Some(given)).unwrap();

    assert_eq!(id, given);
}

#[test]
fn descending_returns_given_id_when_prefix_matches() {
    let given = "msg_000000000001abcdefabcdefab";

    let id = descending(Prefix::Message, Some(given)).unwrap();

    assert_eq!(id, given);
}

#[test]
fn ascending_rejects_given_id_when_prefix_mismatches() {
    let err = ascending(Prefix::Session, Some("msg_000000000001abcdefabcdefab")).unwrap_err();

    match err {
        Error::PrefixMismatch { id, expected_prefix } => {
            assert_eq!(id, "msg_000000000001abcdefabcdefab");
            assert_eq!(expected_prefix, "ses");
        }
        Error::Random(_) => panic!("expected prefix mismatch"),
    }
}

#[test]
fn descending_rejects_given_id_when_prefix_mismatches() {
    let err = descending(Prefix::Tool, Some("que_000000000001abcdefabcdefab")).unwrap_err();

    match err {
        Error::PrefixMismatch { id, expected_prefix } => {
            assert_eq!(id, "que_000000000001abcdefabcdefab");
            assert_eq!(expected_prefix, "tool");
        }
        Error::Random(_) => panic!("expected prefix mismatch"),
    }
}

#[test]
fn ascending_creates_id_with_current_timestamp_shape() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();
    reset_state();

    let id = ascending(Prefix::User, None).unwrap();

    assert_id_shape(&id, Prefix::User);
    assert!(timestamp(&id).is_some());
}

#[test]
fn descending_creates_id_with_current_timestamp_shape() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();
    reset_state();

    let id = descending(Prefix::Permission, None).unwrap();

    assert_id_shape(&id, Prefix::Permission);
    assert!(timestamp(&id).is_some());
}

#[test]
fn create_uses_supplied_timestamp_and_resets_counter_for_new_millisecond() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();
    reset_state();

    let first = create(Prefix::Session, false, Some(42)).unwrap();
    let second = create(Prefix::Session, false, Some(43)).unwrap();

    assert_id_shape(&first, Prefix::Session);
    assert_id_shape(&second, Prefix::Session);
    assert_eq!(&first[4..16], "00000002a001");
    assert_eq!(&second[4..16], "00000002b001");
    assert_eq!(timestamp(&first), Some(42));
    assert_eq!(timestamp(&second), Some(43));
}

#[test]
fn create_increments_counter_for_same_millisecond() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();
    reset_state();

    let first = create(Prefix::Message, false, Some(7)).unwrap();
    let second = create(Prefix::Message, false, Some(7)).unwrap();

    assert_eq!(&first[4..16], "000000007001");
    assert_eq!(&second[4..16], "000000007002");
    assert!(first < second);
}

#[test]
fn create_descending_inverts_encoded_timestamp() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();
    reset_state();

    let first = create(Prefix::Question, true, Some(7)).unwrap();
    let second = create(Prefix::Question, true, Some(7)).unwrap();

    assert_eq!(&first[4..16], "ffffffff8ffe");
    assert_eq!(&second[4..16], "ffffffff8ffd");
    assert!(first > second);
}

#[test]
fn create_saturates_timestamp_encoding_at_u64_max() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();
    reset_state();

    let id = create(Prefix::Part, false, Some(u64::MAX)).unwrap();

    assert_id_shape(&id, Prefix::Part);
    assert_eq!(&id[4..16], "ffffffffffff");
    assert_eq!(timestamp(&id), Some(0x000f_ffff_ffff));
}

#[test]
fn random_base62_returns_requested_number_of_alphanumeric_chars() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();

    let value = random_base62(32).unwrap();

    assert_eq!(value.len(), 32);
    assert!(value.chars().all(|ch| ch.is_ascii_alphanumeric()));
}

#[test]
fn random_base62_accepts_zero_length() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();

    let value = random_base62(0).unwrap();

    assert!(value.is_empty());
}

#[test]
fn random_base62_returns_error_when_random_source_fails() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();

    FORCE_RANDOM_ERROR.store(true, Ordering::SeqCst);

    let err = random_base62(4).unwrap_err();

    assert_eq!(err.to_string(), "forced random error");
}

#[test]
fn create_returns_random_error_when_random_suffix_fails() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();
    reset_state();
    FORCE_RANDOM_ERROR.store(true, Ordering::SeqCst);

    let err = create(Prefix::Session, false, Some(12)).unwrap_err();

    assert_eq!(err.to_string(), "forced random error");
}

#[test]
fn timestamp_parses_prefix_relative_hex_segment() {
    let id = "tool_00000002a001abcdefghijklmn";

    assert_eq!(timestamp(id), Some(42));
}

#[test]
fn timestamp_returns_none_for_missing_or_invalid_hex_segment() {
    assert_eq!(timestamp("ses"), None);
    assert_eq!(timestamp("ses_short"), None);
    assert_eq!(timestamp("ses_not-valid-hexabcdef"), None);
}

#[test]
fn create_recovers_from_poisoned_state_lock() {
    let _guard = STATE_TEST_LOCK.lock().unwrap();
    reset_state();

    let _ = std::thread::spawn(|| {
        let _state = STATE.lock().unwrap();
        panic!("poison id state lock");
    })
    .join();

    let id = create(Prefix::Pty, false, Some(9)).unwrap();

    assert_id_shape(&id, Prefix::Pty);
    assert_eq!(timestamp(&id), Some(9));
}
