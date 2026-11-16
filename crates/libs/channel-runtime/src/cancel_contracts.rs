//! 取消时序契约：借用原生处理器的原子信号，执行原 RuntimeHost。
use crate::{host::ChannelHost, dispatch_test_support::{RecordingChannel, context, message}};
use crate::manager::{dispatch::run_message_dispatch_loop, execution_control::{process_channel_message, active_channel_task, stop_channel_task}};
use channel_support::{runtime_state::ChannelRuntimeContext, scope::ChannelMessageSnapshot, session::sender_session_store};
use channel_types::{Channel, self as traits};
use std::{sync::{Arc, Mutex, atomic::{AtomicBool, Ordering as AtomicOrdering}}, time::Duration, collections::HashMap};
use tokio_util::sync::CancellationToken;
/// 仅借用已注册的真实流信号；契约不创建或模拟流。
pub struct CancelSignals<'a> { pub started: &'a AtomicBool, pub exited: &'a AtomicBool, pub release: &'a AtomicBool }
pub struct CancelFixture<'a> { pub ctx: Arc<ChannelRuntimeContext>, pub channel: Arc<RecordingChannel>, pub msg: traits::ChannelMessage, pub stream: CancelSignals<'a>, pub session: String }
async fn resolve_channel_message_snapshot(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> anyhow::Result<ChannelMessageSnapshot> {
    channel_support::scope::resolve_channel_message_snapshot(&ctx.project_state, ctx.message_snapshot.as_ref(), &ctx.workspace_dir, msg).await
}
fn channel_chat_gate(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) -> Arc<tokio::sync::RwLock<()>> {
    channel_support::scope::channel_chat_gate(&ctx.project_state, msg)
}
async fn cancel_idle(host: &'static dyn ChannelHost, channel_name: &'static str, content: &str) -> Vec<String> {
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new(channel_name));
    let ctx = Arc::new(context(tmp.path(), channel_name, Some(channel.clone()), false));
    let mut msg = message("stop", channel_name, "sender", "room");
    msg.content = content.into();
    let (tx, rx) = tokio::sync::mpsc::channel(2);
    tx.send(msg).await.unwrap();
    drop(tx);
    run_message_dispatch_loop(host, rx, ctx.clone(), 1).await;
    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
    channel.sent_contents().await
}

async fn wait_cancel_flag(flag: &AtomicBool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !flag.load(AtomicOrdering::Acquire) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("model stream flag");
}


pub async fn channel_command_cancel_qq_idle_is_idempotent(host: &'static dyn ChannelHost) {
    assert!(cancel_idle(host, "qq", "/stop").await[0].contains("没有正在执行"));
}

pub async fn channel_command_cancel_lark_idle_is_idempotent(host: &'static dyn ChannelHost) {
    assert!(cancel_idle(host, "lark", "/stop").await[0].contains("没有正在执行"));
}

pub async fn channel_command_cancel_telegram_does_not_require_auto_interrupt(host: &'static dyn ChannelHost) {
    assert!(cancel_idle(host, "telegram", "/stop@bot").await[0].contains("没有正在执行"));
}

pub async fn channel_command_cancel_invalid_arguments_preserve_state(host: &'static dyn ChannelHost) {
    let responses = cancel_idle(host, "qq", "/stop other").await;
    assert!(responses[0].contains("参数错误"));
    assert!(responses[0].contains("/stop"));
}

pub async fn channel_command_cancel_help_describes_scope(host: &'static dyn ChannelHost) {
    let responses = cancel_idle(host, "qq", "/help stop").await;
    assert!(responses[0].contains("/stop"));
    assert!(responses[0].contains("当前聊天"));
}

pub async fn channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session(host: &'static dyn ChannelHost, fixture: CancelFixture<'_>) -> ChannelMessageSnapshot {
    let CancelFixture { ctx, channel, msg, stream, session: old_session } = fixture;
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    let dispatch = tokio::spawn(run_message_dispatch_loop(host, rx, ctx.clone(), 1));
    tx.send(msg.clone()).await.unwrap();
    wait_cancel_flag(&stream.started).await;
    // Another request queues for the only permit; /stop must still reach its target.
    let mut queued = msg.clone();
    queued.sender = "another-sender".into();
    queued.content = "/model".into();
    tx.send(queued).await.unwrap();
    let mut stop = msg.clone();
    stop.content = "/stop".into();
    tx.send(stop).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if channel.sent_contents().await.iter().any(|text| text.contains("执行已停止")) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("stop bypasses semaphore and awaits actual exit");
    assert!(stream.exited.load(AtomicOrdering::Acquire));
    assert!(active_channel_task(&ctx, &msg).is_none());
    let mut new = msg.clone();
    new.content = "/new".into();
    process_channel_message(host, ctx.clone(), new, CancellationToken::new()).await;
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let new_session = sender_session_store().lock().unwrap().get(&snapshot.scope).unwrap().clone();
    assert_ne!(new_session, old_session);
    assert!(!channel.sent_contents().await.iter().any(|text| text.contains("completed naturally")));
    assert!(ctx.conversation_histories.lock().unwrap().get(&snapshot.scope.key()).is_none());
    drop(tx);
    dispatch.await.unwrap();
    assert_eq!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&new_session));
    snapshot
}

pub async fn channel_command_cancel_isolates_instance_room_thread_sender_and_tenant(host: &'static dyn ChannelHost, fixture: CancelFixture<'_>) {
    let CancelFixture { ctx, channel: _channel, msg, stream, session: _session } = fixture;
    let worker =
        tokio::spawn(process_channel_message(host, ctx.clone(), msg.clone(), CancellationToken::new()));
    wait_cancel_flag(&stream.started).await;
    for dimension in 0..4 {
        let mut other = msg.clone();
        match dimension {
            0 => other.instance_id = "other".into(),
            1 => other.reply_target = "other".into(),
            2 => other.thread_ts = Some("other".into()),
            _ => other.sender = "other".into(),
        }
        assert!(stop_channel_task(&ctx, &other).await.contains("没有正在执行"));
        assert!(!active_channel_task(&ctx, &msg).unwrap().cancellation.is_cancelled());
    }
    let mut other_ctx = ctx.as_ref().clone();
    let mut other_state = ctx.project_state.lock().unwrap().clone();
    other_state.tenant =
        vw_api_transport::TenantScope::new("other-cancel-platform", "other-cancel-tenant").unwrap();
    other_ctx.project_state = Arc::new(Mutex::new(other_state));
    assert!(stop_channel_task(&other_ctx, &msg).await.contains("没有正在执行"));
    assert!(!active_channel_task(&ctx, &msg).unwrap().cancellation.is_cancelled());
    assert!(stop_channel_task(&ctx, &msg).await.contains("执行已停止"));
    worker.await.unwrap();
}

pub async fn channel_command_cancel_unconfirmed_exit_keeps_mutations_busy(host: &'static dyn ChannelHost, fixture: CancelFixture<'_>) {
    let CancelFixture { ctx, channel, msg, stream, session } = fixture;
    let worker =
        tokio::spawn(process_channel_message(host, ctx.clone(), msg.clone(), CancellationToken::new()));
    wait_cancel_flag(&stream.started).await;
    let (first, second) =
        tokio::join!(stop_channel_task(&ctx, &msg), stop_channel_task(&ctx, &msg));
    assert!(first.contains("尚未停止"));
    assert!(second.contains("尚未停止"));
    assert!(!stream.exited.load(AtomicOrdering::Acquire));
    let mut new = msg.clone();
    new.content = "/new".into();
    process_channel_message(host, ctx.clone(), new, CancellationToken::new()).await;
    assert!(channel.sent_contents().await.iter().any(|text| text.contains("繁忙")));
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&session));
    stream.release.store(true, AtomicOrdering::Release);
    worker.await.unwrap();
    assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"));
}

pub async fn channel_command_cancel_busy_commands_preserve_state_and_status_responds(host: &'static dyn ChannelHost, fixture: CancelFixture<'_>) {
    let CancelFixture { ctx, channel, msg, stream, session } = fixture;
    let worker =
        tokio::spawn(process_channel_message(host, ctx.clone(), msg.clone(), CancellationToken::new()));
    wait_cancel_flag(&stream.started).await;
    for command in
        ["/new", "/project default", "/resume invalid", "/reset", "/compact", "/stop invalid"]
    {
        let mut control = msg.clone();
        control.content = command.into();
        process_channel_message(host, ctx.clone(), control, CancellationToken::new()).await;
        assert!(!active_channel_task(&ctx, &msg).unwrap().cancellation.is_cancelled());
    }
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&session));
    for command in ["/help stop", "/status"] {
        let mut control = msg.clone();
        control.content = command.into();
        let before = channel.sent.lock().await.len();
        tokio::time::timeout(
            Duration::from_secs(1),
            process_channel_message(host, ctx.clone(), control, CancellationToken::new()),
        )
        .await
        .unwrap();
        assert!(channel.sent.lock().await.len() > before);
    }
    stream.release.store(true, AtomicOrdering::Release);
    let stopping = stop_channel_task(&ctx, &msg).await;
    assert!(stopping.contains("执行已停止") || stopping.contains("没有正在执行"));
    worker.await.unwrap();
    assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"));
}

pub async fn channel_command_cancel_natural_completion_then_stop_is_idempotent(host: &'static dyn ChannelHost, fixture: CancelFixture<'_>) {
    let CancelFixture { ctx, channel, msg, stream, session } = fixture;
    stream.release.store(true, AtomicOrdering::Release);
    process_channel_message(host, ctx.clone(), msg.clone(), CancellationToken::new()).await;
    assert!(stream.exited.load(AtomicOrdering::Acquire));
    assert!(channel.sent_contents().await.iter().any(|text| text.contains("completed naturally")));
    assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"));
    assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"));
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&session));
    assert_eq!(ctx.conversation_histories.lock().unwrap()[&snapshot.scope.key()].len(), 2);
}

pub async fn channel_command_cancel_waits_for_feedback_before_releasing_scope(host: &'static dyn ChannelHost, fixture: CancelFixture<'_>) {
    let CancelFixture { ctx, channel: _channel, msg, stream, session: _session } = fixture;
    let channel = Arc::new(RecordingChannel::feedback_blocking("lark"));
    let mut blocked_ctx = ctx.as_ref().clone();
    blocked_ctx.channels_by_name =
        Arc::new(HashMap::from([("lark".into(), channel.clone() as Arc<dyn Channel>)]));
    let ctx = Arc::new(blocked_ctx);
    let worker =
        tokio::spawn(process_channel_message(host, ctx.clone(), msg.clone(), CancellationToken::new()));
    wait_cancel_flag(&stream.started).await;
    let stopping = tokio::spawn({
        let ctx = ctx.clone();
        let msg = msg.clone();
        async move { stop_channel_task(&ctx, &msg).await }
    });
    channel.wait_for_active_sends(1).await;
    assert!(stream.exited.load(AtomicOrdering::Acquire));
    assert!(channel_chat_gate(&ctx, &msg).try_write().is_err());
    assert!(stopping.await.unwrap().contains("尚未停止"));
    channel.release();
    worker.await.unwrap();
    assert!(active_channel_task(&ctx, &msg).is_none());
    assert!(channel_chat_gate(&ctx, &msg).try_write().is_ok());
}

pub async fn channel_command_cancel_dropped_caller_still_drains_processor(host: &'static dyn ChannelHost, fixture: CancelFixture<'_>) {
    let CancelFixture { ctx, channel: _channel, msg, stream, session } = fixture;
    let worker =
        tokio::spawn(process_channel_message(host, ctx.clone(), msg.clone(), CancellationToken::new()));
    wait_cancel_flag(&stream.started).await;
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    wait_cancel_flag(&stream.exited).await;
    tokio::time::timeout(Duration::from_secs(2), async {
        while active_channel_task(&ctx, &msg).is_some() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let mut new = msg.clone();
    new.content = "/new".into();
    process_channel_message(host, ctx.clone(), new, CancellationToken::new()).await;
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_ne!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&session));
}
