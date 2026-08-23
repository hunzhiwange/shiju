//! 验证安全动作计数、克隆隔离与过期记录清理。

use super::*;

#[test]
fn tracker_counts_recorded_actions() {
    let tracker = ActionTracker::new();
    tracker.record();
    tracker.record();
    assert_eq!(tracker.count(), 2);
}

#[test]
fn default_and_clone_have_independent_action_lists() {
    let tracker = ActionTracker::default();
    assert_eq!(tracker.record(), 1);

    let cloned = tracker.clone();
    assert_eq!(cloned.count(), 1);
    assert_eq!(cloned.record(), 2);
    assert_eq!(tracker.count(), 1);
}

#[test]
fn expired_actions_are_pruned_from_count_and_record() {
    let tracker = ActionTracker::new();
    {
        let mut actions = tracker.actions.lock();
        actions.push(std::time::Instant::now() - std::time::Duration::from_secs(3700));
        actions.push(std::time::Instant::now());
    }

    assert_eq!(tracker.count(), 1);
    assert_eq!(tracker.record(), 2);
}
