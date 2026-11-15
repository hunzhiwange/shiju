//! 验证取消控制器、组合信号与定时取消句柄的生命周期语义。
//! 已取消信号应立即完成，清除定时器则不得误触发后续等待方。

use super::abort::{AbortController, AbortSignal, abort_after, abort_after_any};
use std::time::Duration;

#[tokio::test]
async fn abort_controller_signals_cancellation() {
    let (controller, mut signal) = AbortController::new();
    assert!(!signal.aborted());
    controller.abort();
    signal.cancelled().await;
    assert!(signal.aborted());
}

#[tokio::test]
async fn any_aborts_when_one_signal_aborts() {
    let (controller, signal) = AbortController::new();
    let mut combined = AbortSignal::any([signal]);
    controller.abort();
    combined.cancelled().await;
    assert!(combined.aborted());
}

#[tokio::test]
async fn clear_timeout_prevents_auto_abort() {
    let mut timeout = abort_after(1);
    timeout.clear_timeout();
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(!timeout.signal.aborted());
}

#[test]
fn controller_signal_subscribes_to_shared_abort_state() {
    let (controller, original) = AbortController::new();
    let subscribed = controller.signal();

    controller.abort();

    assert!(original.aborted());
    assert!(subscribed.aborted());
}

#[tokio::test]
async fn cancelled_returns_immediately_for_already_aborted_signal() {
    let (controller, mut signal) = AbortController::new();
    controller.abort();

    signal.cancelled().await;

    assert!(signal.aborted());
}

#[tokio::test]
async fn abort_after_triggers_signal_when_delay_expires() {
    let mut timeout = abort_after(0);

    timeout.signal.cancelled().await;

    assert!(timeout.signal.aborted());
}

#[tokio::test]
async fn abort_after_any_uses_external_signal_after_timeout_is_cleared() {
    let (controller, signal) = AbortController::new();
    let mut combined = abort_after_any(60_000, [signal]);
    combined.clear_timeout();

    controller.abort();
    combined.signal.cancelled().await;

    assert!(combined.signal.aborted());
}
