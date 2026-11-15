//! 聊天键值与分叉目标的领域断言。
use super::orchestration::think_block_key;
use super::events::ForkSessionTarget;

#[test]
fn think_block_key_packs_message_and_think_indexes() {
    assert_eq!(think_block_key(0, 7), 7);
    assert_eq!(think_block_key(2, 3), (2_u64 << 32) | 3);
}

#[test]
fn fork_session_targets_distinguish_destinations() {
    assert_eq!(ForkSessionTarget::Local, ForkSessionTarget::Local);
    assert_ne!(ForkSessionTarget::Local, ForkSessionTarget::NewWorktree);
}
