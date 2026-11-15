//! 验证聊天工具渲染分派。
//! 测试保护不同工具输出选择正确视图组件。

use super::tools::{
    explore_summary_expanded, explore_summary_is_running,
};
use std::collections::HashSet;

#[test]
fn completed_explore_summary_does_not_expand_without_manual_override() {
    let key = 99_u64;

    assert!(!explore_summary_expanded(false, key, &HashSet::new()));
}

#[test]
fn running_explore_summary_requires_manual_expand() {
    let key = 99_u64;
    let mut expanded = HashSet::new();

    assert!(!explore_summary_expanded(true, key, &expanded));

    expanded.insert(key);
    assert!(explore_summary_expanded(true, key, &expanded));
}

#[test]
fn visible_follow_up_block_closes_running_explore_summary() {
    assert!(!explore_summary_is_running(true, false, true));
}

#[test]
fn hidden_think_only_keeps_explore_running_until_visible_boundary() {
    assert!(explore_summary_is_running(false, true, false));
    assert!(!explore_summary_is_running(false, true, true));
}
