//! 验证信号在触发前后均可可靠唤醒等待方。
//! 手动轮询路径确保注册唤醒器后再触发时不会丢失通知。

use super::signal::Signal;
use std::future::Future;
use std::task::{Context, Poll, Waker};

#[tokio::test]
async fn trigger_releases_waiter() {
    let mut signal = Signal::new();
    signal.trigger();
    signal.wait().await;
}

#[tokio::test]
async fn wait_observes_trigger_after_becoming_pending() {
    let mut signal = Signal::new();
    let sender = signal.test_sender();
    let mut wait = Box::pin(signal.wait());
    let mut context = Context::from_waker(Waker::noop());

    assert!(matches!(wait.as_mut().poll(&mut context), Poll::Pending));
    sender.send(true).expect("signal receiver remains alive");
    wait.await;
}
