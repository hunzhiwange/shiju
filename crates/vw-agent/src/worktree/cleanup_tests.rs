//! Real runtime status conversion at the worktree host boundary.
#[test]
fn worktree_host_busy_snapshot_includes_busy_and_retry_only() {
    use crate::session::status::Info;
    use std::collections::{HashMap, HashSet};
    let busy = super::super::host_adapter::busy_sessions(HashMap::from([
        ("busy".into(), Info::Busy),
        ("retry".into(), Info::Retry { attempt: 1, message: "retry".into(), next: 100 }),
        ("idle".into(), Info::Idle),
    ]));
    assert_eq!(busy, HashSet::from(["busy".into(), "retry".into()]));
}
