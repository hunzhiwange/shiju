use super::*;
use std::sync::Arc;
use std::sync::atomic::Ordering;

#[test]
fn root_module_reexports_core_channel_types() {
    fn assert_channel<T: Channel>(_channel: &T) {}

    let channel = CliChannel::new();
    let message = SendMessage::with_subject("hello", "stdout", "subject")
        .in_thread(Some("thread-1".to_string()));

    assert_channel(&channel);
    assert_eq!(channel.name(), "cli");
    assert_eq!(message.content, "hello");
    assert_eq!(message.recipient, "stdout");
    assert_eq!(message.subject.as_deref(), Some("subject"));
    assert_eq!(message.thread_ts.as_deref(), Some("thread-1"));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_channel_reexports_are_constructible() {
    let _mattermost = MattermostChannel::new(
        "https://mattermost.example.com/".to_string(),
        "token".to_string(),
        None,
        vec!["*".to_string()],
        true,
        false,
    );
    let _nextcloud = NextcloudTalkChannel::new(
        "https://cloud.example.com/".to_string(),
        "app-token".to_string(),
        vec!["alice".to_string()],
    );
}

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
fn configured_channel_new_stores_display_name() {
    let channel: Arc<dyn Channel> = Arc::new(CliChannel::new());
    let configured = ConfiguredChannel::new("My CLI", channel);
    assert_eq!(configured.display_name, "My CLI");
    assert_eq!(configured.channel.name(), "cli");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_future_runs_outside_runtime() {
    // 不在 Tokio 运行时中调用时，应自建运行时执行 Future。
    let value = block_on_future(async { 7_u32 + 35 });
    assert_eq!(value, 42);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn block_on_future_runs_inside_runtime() {
    // 已处于运行时中时，走 block_in_place 分支。
    let rt =
        tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    let value = rt.block_on(async { block_on_future(async { "ok" }) });
    assert_eq!(value, "ok");
}

#[test]
fn channel_health_state_variants_compare() {
    assert_eq!(ChannelHealthState::Healthy, ChannelHealthState::Healthy);
    assert_ne!(ChannelHealthState::Healthy, ChannelHealthState::Unhealthy);
    assert_ne!(ChannelHealthState::Unhealthy, ChannelHealthState::Timeout);
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
