//! 调度断言注入真实宿主，保留并发与取消时序。
use crate::dispatch_test_support::{RecordingChannel, context, message};
use crate::manager::dispatch::run_message_dispatch_loop;
use channel_support::runtime_state::ChannelRuntimeContext;
use channel_types::{Channel, self as traits};
use std::{sync::Arc, path::Path, time::Duration};

async fn registered_context(
    workspace_dir: &Path,
    channel_name: &str,
    channel: Option<Arc<dyn Channel>>,
    interrupt_on_new_message: bool,
) -> ChannelRuntimeContext {
    agent_project::from_directory(
        &session_store::ui_store::current_tenant_scope(),
        workspace_dir,
    )
    .await
    .unwrap();
    context(workspace_dir, channel_name, channel, interrupt_on_new_message)
}

pub async fn run_message_dispatch_loop_processes_runtime_commands_until_input_closes(host: &'static dyn crate::host::ChannelHost) {
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(registered_context(tmp.path(), "telegram", Some(channel_dyn), false).await);
    let (tx, rx) = tokio::sync::mpsc::channel(4);

    tx.send(message("msg-1", "telegram", "sender-1", "chat-1")).await.unwrap();
    tx.send(message("msg-2", "telegram", "sender-2", "chat-2")).await.unwrap();
    drop(tx);

    run_message_dispatch_loop(host, rx, ctx, 4).await;

    let sent = channel.sent_contents().await;
    assert_eq!(sent.len(), 2);
    assert!(sent.iter().all(|content| content.contains("Current model")));
}

pub async fn run_message_dispatch_loop_limits_in_flight_workers(host: &'static dyn crate::host::ChannelHost) {
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::blocking("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(registered_context(tmp.path(), "telegram", Some(channel_dyn), false).await);
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let dispatch = tokio::spawn(run_message_dispatch_loop(host, rx, ctx, 1));

    tx.send(message("msg-1", "telegram", "sender-1", "chat-1")).await.unwrap();
    tx.send(message("msg-2", "telegram", "sender-2", "chat-2")).await.unwrap();
    channel.wait_for_active_sends(1).await;
    tokio::time::sleep(Duration::from_millis(25)).await;

    assert_eq!(channel.max_active_sends(), 1);

    channel.release();
    drop(tx);
    tokio::time::timeout(Duration::from_secs(1), dispatch).await.unwrap().unwrap();

    assert_eq!(channel.sent_contents().await.len(), 2);
    assert_eq!(channel.max_active_sends(), 1);
}

pub async fn run_message_dispatch_loop_skips_interrupt_for_non_telegram_channel(host: &'static dyn crate::host::ChannelHost) {
    // interrupt_on_new_message 为 true，但通道不是 telegram，
    // 因此 interrupt_enabled 为 false，两条同发送者消息都应被独立处理。
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new("slack"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(registered_context(tmp.path(), "slack", Some(channel_dyn), true).await);
    let (tx, rx) = tokio::sync::mpsc::channel(4);

    tx.send(message("msg-1", "slack", "sender-1", "chat-1")).await.unwrap();
    tx.send(message("msg-2", "slack", "sender-1", "chat-1")).await.unwrap();
    drop(tx);

    run_message_dispatch_loop(host, rx, ctx, 4).await;

    // 未触发中断，两条消息都得到回复。
    assert_eq!(channel.sent_contents().await.len(), 2);
}

pub async fn run_message_dispatch_loop_keeps_distinct_telegram_senders_independent(host: &'static dyn crate::host::ChannelHost) {
    // 启用中断但两条消息来自不同发送者作用域，互不取消，应都完成。
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(registered_context(tmp.path(), "telegram", Some(channel_dyn), true).await);
    let (tx, rx) = tokio::sync::mpsc::channel(4);

    tx.send(message("msg-1", "telegram", "sender-1", "chat-1")).await.unwrap();
    tx.send(message("msg-2", "telegram", "sender-2", "chat-2")).await.unwrap();
    drop(tx);

    run_message_dispatch_loop(host, rx, ctx, 4).await;

    assert_eq!(channel.sent_contents().await.len(), 2);
}

pub async fn run_message_dispatch_loop_stops_when_semaphore_closed_before_receive(host: &'static dyn crate::host::ChannelHost) {
    // max_in_flight_messages 为 0 时信号量无许可，acquire 会等待；
    // 这里验证关闭接收端后循环优雅退出且不发送任何消息。
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(registered_context(tmp.path(), "telegram", Some(channel_dyn), false).await);
    let (tx, rx) = tokio::sync::mpsc::channel::<traits::ChannelMessage>(1);
    drop(tx);

    tokio::time::timeout(Duration::from_secs(1), run_message_dispatch_loop(host, rx, ctx, 1))
        .await
        .expect("loop should exit promptly when receiver is closed");

    assert!(channel.sent_contents().await.is_empty());
}

pub async fn run_message_dispatch_loop_does_not_interrupt_model_queries(host: &'static dyn crate::host::ChannelHost) {
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::blocking("telegram"));
    let channel_dyn: Arc<dyn Channel> = channel.clone();
    let ctx = Arc::new(registered_context(tmp.path(), "telegram", Some(channel_dyn), true).await);
    let (tx, rx) = tokio::sync::mpsc::channel(4);
    let dispatch = tokio::spawn(run_message_dispatch_loop(host, rx, ctx, 2));

    tx.send(message("msg-1", "telegram", "sender-1", "chat-1")).await.unwrap();
    channel.wait_for_active_sends(1).await;
    tx.send(message("msg-2", "telegram", "sender-1", "chat-1")).await.unwrap();
    channel.wait_for_active_sends(2).await;

    assert_eq!(channel.max_active_sends(), 2);
    assert!(channel.sent_contents().await.is_empty());

    channel.release();
    drop(tx);
    tokio::time::timeout(Duration::from_secs(1), dispatch).await.unwrap().unwrap();

    assert_eq!(channel.sent_contents().await.len(), 2);
    assert_eq!(channel.max_active_sends(), 2);
}
