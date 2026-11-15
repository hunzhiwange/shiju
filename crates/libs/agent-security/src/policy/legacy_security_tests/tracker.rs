use super::*;

// 测试动作计数器初始值为零
#[test]
fn action_tracker_starts_at_zero() {
    let tracker = ActionTracker::new();
    assert_eq!(tracker.count(), 0);
}

// 测试动作计数器正确记录动作
#[test]
fn action_tracker_records_actions() {
    let tracker = ActionTracker::new();
    assert_eq!(tracker.record(), 1);
    assert_eq!(tracker.record(), 2);
    assert_eq!(tracker.record(), 3);
    assert_eq!(tracker.count(), 3);
}

// 测试在限制内记录动作被允许
#[test]
fn record_action_allows_within_limit() {
    let p = SecurityPolicy { max_actions_per_hour: 5, ..SecurityPolicy::default() };
    for _ in 0..5 {
        assert!(p.record_action(), "should allow actions within limit");
    }
}

// 测试超过限制时记录动作被阻止
#[test]
fn record_action_blocks_over_limit() {
    let p = SecurityPolicy { max_actions_per_hour: 3, ..SecurityPolicy::default() };
    assert!(p.record_action());
    assert!(p.record_action());
    assert!(p.record_action());
    assert!(!p.record_action());
}

// 测试速率限制状态反映当前计数
#[test]
fn is_rate_limited_reflects_count() {
    let p = SecurityPolicy { max_actions_per_hour: 2, ..SecurityPolicy::default() };
    assert!(!p.is_rate_limited());
    p.record_action();
    assert!(!p.is_rate_limited());
    p.record_action();
    assert!(p.is_rate_limited());
}

// 测试动作计数器克隆是独立的
#[test]
fn action_tracker_clone_is_independent() {
    let tracker = ActionTracker::new();
    tracker.record();
    tracker.record();
    let cloned = tracker.clone();
    assert_eq!(cloned.count(), 2);
    tracker.record();
    assert_eq!(tracker.count(), 3);
    assert_eq!(cloned.count(), 2);
}

// 测试速率限制边界值
#[test]
fn rate_limit_exactly_at_boundary() {
    let p = SecurityPolicy { max_actions_per_hour: 1, ..SecurityPolicy::default() };
    assert!(p.record_action());
    assert!(!p.record_action());
    assert!(!p.record_action());
}

// 测试零速率限制阻止所有操作
#[test]
fn rate_limit_zero_blocks_everything() {
    let p = SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() };
    assert!(!p.record_action());
}

// 测试高速率限制允许大量操作
#[test]
fn rate_limit_high_allows_many() {
    let p = SecurityPolicy { max_actions_per_hour: 10000, ..SecurityPolicy::default() };
    for _ in 0..100 {
        assert!(p.record_action());
    }
}
