use crate::scope::InFlightTaskCompletion;
use crate::routing::ChannelRouteSelection;
use std::sync::{Arc, atomic::Ordering};

#[test]
fn in_flight_completion_starts_not_done() {
    let completion = InFlightTaskCompletion::new();
    assert!(!completion.done.load(Ordering::Acquire));
}

#[test]
fn in_flight_completion_mark_done_sets_flag() {
    let completion = InFlightTaskCompletion::new();
    completion.mark_done();
    assert!(completion.done.load(Ordering::Acquire));
}

#[test]
fn in_flight_completion_wait_returns_immediately_when_done() {
    // mark_done 后走 wait 的快速路径，应立即返回而不阻塞。
    let completion = Arc::new(InFlightTaskCompletion::new());
    completion.mark_done();

    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    rt.block_on(async {
        completion.wait().await;
    });
    assert!(completion.done.load(Ordering::Acquire));
}

#[test]
fn in_flight_completion_wait_wakes_on_mark_done() {
    // 先进入慢速路径等待，再由另一任务标记完成并唤醒等待者。
    let completion = Arc::new(InFlightTaskCompletion::new());
    let rt =
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    rt.block_on(async {
        let waiter = {
            let completion = Arc::clone(&completion);
            tokio::spawn(async move {
                completion.wait().await;
                completion.done.load(Ordering::Acquire)
            })
        };
        // 给等待者时间进入 notified() 等待。
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        completion.mark_done();
        assert!(waiter.await.unwrap());
    });
}

#[test]
fn channel_route_selection_equality() {
    let a = ChannelRouteSelection {
        provider: "anthropic".to_string(),
        model: "claude".to_string(),
        reasoning_effort: None,
        task_mode_enabled: true,
    };
    let b = a.clone();
    assert_eq!(a, b);
    let c = ChannelRouteSelection { task_mode_enabled: false, ..a.clone() };
    assert_ne!(a, c);
}
