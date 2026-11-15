use super::*;

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
