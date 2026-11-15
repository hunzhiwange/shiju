use super::app_state::is_recent_copy;
use web_time::{Duration as WebDuration, SystemTime as WebSystemTime};
#[test]
fn is_recent_copy_requires_matching_hash_and_fresh_timestamp() {
    let last_copied_code_hash = Some(42);
    let mut last_copy_time = Some(WebSystemTime::now() - WebDuration::from_millis(500));

    assert!(is_recent_copy(last_copied_code_hash, last_copy_time, 42));
    assert!(!is_recent_copy(last_copied_code_hash, last_copy_time, 7));

    last_copy_time = Some(WebSystemTime::now() - WebDuration::from_millis(2_000));
    assert!(!is_recent_copy(last_copied_code_hash, last_copy_time, 42));
}

#[test]
fn is_recent_copy_rejects_missing_or_unusable_copy_state() {
    let mut last_copied_code_hash = None;
    let mut last_copy_time = None;

    assert!(!is_recent_copy(last_copied_code_hash, last_copy_time, 42));

    last_copied_code_hash = Some(42);
    assert!(!is_recent_copy(last_copied_code_hash, last_copy_time, 42));

    last_copy_time = Some(WebSystemTime::now() + WebDuration::from_millis(60_000));
    assert!(!is_recent_copy(last_copied_code_hash, last_copy_time, 42));
}
