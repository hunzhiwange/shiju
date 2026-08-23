//! 测试消息超时下限、缩放上限与饱和计算。

use super::*;

#[test]
fn effective_timeout_applies_minimum_floor() {
    assert_eq!(effective_message_timeout_secs(0), MIN_CHANNEL_MESSAGE_TIMEOUT_SECS);
    assert_eq!(effective_message_timeout_secs(1), MIN_CHANNEL_MESSAGE_TIMEOUT_SECS);
    assert_eq!(
        effective_message_timeout_secs(MIN_CHANNEL_MESSAGE_TIMEOUT_SECS),
        MIN_CHANNEL_MESSAGE_TIMEOUT_SECS
    );
    assert_eq!(effective_message_timeout_secs(90), 90);
}

#[test]
fn timeout_budget_scales_with_cap_and_saturates() {
    assert_eq!(message_timeout_budget_secs(10, 0), 10);
    assert_eq!(message_timeout_budget_secs(10, 1), 10);
    assert_eq!(message_timeout_budget_secs(10, 2), 20);
    assert_eq!(
        message_timeout_budget_secs(10, CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP as usize),
        10 * CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP
    );
    assert_eq!(message_timeout_budget_secs(10, 99), 10 * CHANNEL_MESSAGE_TIMEOUT_SCALE_CAP);
    assert_eq!(message_timeout_budget_secs(u64::MAX, 2), u64::MAX);
}
