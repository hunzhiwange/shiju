use super::dispatch_tests::{RecordingChannel, context, message};
use super::*;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

async fn cancel_idle(channel_name: &'static str, content: &str) -> Vec<String> {
    let tmp = tempfile::tempdir().unwrap();
    let channel = Arc::new(RecordingChannel::new(channel_name));
    let ctx = Arc::new(context(tmp.path(), channel_name, Some(channel.clone()), false));
    let mut msg = message("stop", channel_name, "sender", "room");
    msg.content = content.into();
    let (tx, rx) = tokio::sync::mpsc::channel(2);
    tx.send(msg).await.unwrap();
    drop(tx);
    run_message_dispatch_loop(rx, ctx.clone(), 1).await;
    assert!(ctx.conversation_histories.lock().unwrap().is_empty());
    channel.sent_contents().await
}

#[tokio::test]
async fn channel_command_cancel_qq_idle_is_idempotent() {
    assert!(cancel_idle("qq", "/stop").await[0].contains("没有正在执行"));
}

#[tokio::test]
async fn channel_command_cancel_lark_idle_is_idempotent() {
    assert!(cancel_idle("lark", "/stop").await[0].contains("没有正在执行"));
}

#[tokio::test]
async fn channel_command_cancel_telegram_does_not_require_auto_interrupt() {
    assert!(cancel_idle("telegram", "/stop@bot").await[0].contains("没有正在执行"));
}

#[tokio::test]
async fn channel_command_cancel_invalid_arguments_preserve_state() {
    let responses = cancel_idle("qq", "/stop other").await;
    assert!(responses[0].contains("参数错误"));
    assert!(responses[0].contains("/stop"));
}

#[tokio::test]
async fn channel_command_cancel_help_describes_scope() {
    let responses = cancel_idle("qq", "/help stop").await;
    assert!(responses[0].contains("/stop"));
    assert!(responses[0].contains("当前聊天"));
}

async fn isolated_cancel_test(name: &str) -> bool {
    let name = name.strip_prefix("vw_agent::").unwrap_or(name);
    if std::env::var("VW_CHANNEL_CANCEL_TEST").as_deref() == Ok(name) {
        crate::config::update(serde_json::json!({
            "providers": {"channel-cancel-fixture": {"models": {"cancel-model": {"status": "active"}}}}
        })).await.unwrap();
        return false;
    }
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("VW_CHANNEL_CANCEL_TEST", name)
        .env("HOME", home.path())
        .env("VIBEWINDOW_CONFIG_DIR", home.path().join("config"))
        .env("VIBEWINDOW_WORKSPACE", home.path().join("workspace"))
        .output()
        .unwrap();
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}

async fn cancel_fixture(
    hold_exit: bool,
) -> (
    tempfile::TempDir,
    Arc<ChannelRuntimeContext>,
    Arc<RecordingChannel>,
    traits::ChannelMessage,
    Arc<crate::session::processor::ChannelCancelStream>,
    String,
) {
    let tmp = tempfile::tempdir().unwrap();
    crate::project::from_directory(&crate::session::ui_store::current_tenant_scope(), tmp.path())
        .await
        .unwrap();
    let channel = Arc::new(RecordingChannel::new("lark"));
    let mut ctx = context(tmp.path(), "lark", Some(channel.clone()), false);
    ctx.default_provider = Arc::new("channel-cancel-fixture".into());
    ctx.model = Arc::new("cancel-model".into());
    ctx.provider_cache
        .lock()
        .unwrap()
        .insert("channel-cancel-fixture".into(), ctx.provider.clone());
    let ctx = Arc::new(ctx);
    let mut msg = message("new", "lark", "sender", "room");
    msg.content = "/new".into();
    process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()).await;
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let session = sender_session_store().lock().unwrap().get(&snapshot.scope).unwrap().clone();
    channel.sent.lock().await.clear();
    let stream = Arc::new(crate::session::processor::ChannelCancelStream {
        hold_exit,
        ..Default::default()
    });
    crate::session::processor::register_channel_cancel_stream(&session, stream.clone());
    msg.id = "ordinary".into();
    msg.content = "Tell me a short story".into();
    (tmp, ctx, channel, msg, stream, session)
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

#[tokio::test]
async fn channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session() {
    let name = concat!(
        module_path!(),
        "::channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session"
    );
    if isolated_cancel_test(name).await {
        return;
    }
    let (_tmp, ctx, channel, msg, stream, old_session) = cancel_fixture(false).await;
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    let dispatch = tokio::spawn(run_message_dispatch_loop(rx, ctx.clone(), 1));
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
    process_channel_message(ctx.clone(), new, CancellationToken::new()).await;
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    let new_session = sender_session_store().lock().unwrap().get(&snapshot.scope).unwrap().clone();
    assert_ne!(new_session, old_session);
    assert!(!channel.sent_contents().await.iter().any(|text| text.contains("completed naturally")));
    assert!(ctx.conversation_histories.lock().unwrap().get(&snapshot.scope.key()).is_none());
    drop(tx);
    dispatch.await.unwrap();
    assert_eq!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&new_session));
    let recorded = crate::session::processor::take_test_llm_messages(&old_session);
    assert_eq!(recorded.len(), 1);
    assert_eq!(std::fs::canonicalize(&recorded[0].directory).unwrap(), snapshot.directory);
    assert_eq!(recorded[0].tenant, snapshot.scope.chat.tenant);
}

#[tokio::test]
async fn channel_command_cancel_isolates_instance_room_thread_sender_and_tenant() {
    let name = concat!(
        module_path!(),
        "::channel_command_cancel_isolates_instance_room_thread_sender_and_tenant"
    );
    if isolated_cancel_test(name).await {
        return;
    }
    let (_tmp, ctx, _channel, msg, stream, _session) = cancel_fixture(false).await;
    let worker =
        tokio::spawn(process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()));
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
        vw_api_types::id::TenantScope::new("other-cancel-platform", "other-cancel-tenant").unwrap();
    other_ctx.project_state = Arc::new(Mutex::new(other_state));
    assert!(stop_channel_task(&other_ctx, &msg).await.contains("没有正在执行"));
    assert!(!active_channel_task(&ctx, &msg).unwrap().cancellation.is_cancelled());
    assert!(stop_channel_task(&ctx, &msg).await.contains("执行已停止"));
    worker.await.unwrap();
}

#[tokio::test]
async fn channel_command_cancel_unconfirmed_exit_keeps_mutations_busy() {
    let name =
        concat!(module_path!(), "::channel_command_cancel_unconfirmed_exit_keeps_mutations_busy");
    if isolated_cancel_test(name).await {
        return;
    }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(true).await;
    let worker =
        tokio::spawn(process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()));
    wait_cancel_flag(&stream.started).await;
    let (first, second) =
        tokio::join!(stop_channel_task(&ctx, &msg), stop_channel_task(&ctx, &msg));
    assert!(first.contains("尚未停止"));
    assert!(second.contains("尚未停止"));
    assert!(!stream.exited.load(AtomicOrdering::Acquire));
    let mut new = msg.clone();
    new.content = "/new".into();
    process_channel_message(ctx.clone(), new, CancellationToken::new()).await;
    assert!(channel.sent_contents().await.iter().any(|text| text.contains("繁忙")));
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&session));
    stream.release.store(true, AtomicOrdering::Release);
    worker.await.unwrap();
    assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"));
}

#[tokio::test]
async fn channel_command_cancel_busy_commands_preserve_state_and_status_responds() {
    let name = concat!(
        module_path!(),
        "::channel_command_cancel_busy_commands_preserve_state_and_status_responds"
    );
    if isolated_cancel_test(name).await {
        return;
    }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(false).await;
    let worker =
        tokio::spawn(process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()));
    wait_cancel_flag(&stream.started).await;
    for command in
        ["/new", "/project default", "/resume invalid", "/reset", "/compact", "/stop invalid"]
    {
        let mut control = msg.clone();
        control.content = command.into();
        process_channel_message(ctx.clone(), control, CancellationToken::new()).await;
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
            process_channel_message(ctx.clone(), control, CancellationToken::new()),
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

#[tokio::test]
async fn channel_command_cancel_natural_completion_then_stop_is_idempotent() {
    let name = concat!(
        module_path!(),
        "::channel_command_cancel_natural_completion_then_stop_is_idempotent"
    );
    if isolated_cancel_test(name).await {
        return;
    }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(false).await;
    stream.release.store(true, AtomicOrdering::Release);
    process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()).await;
    assert!(stream.exited.load(AtomicOrdering::Acquire));
    assert!(channel.sent_contents().await.iter().any(|text| text.contains("completed naturally")));
    assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"));
    assert!(stop_channel_task(&ctx, &msg).await.contains("没有正在执行"));
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_eq!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&session));
    assert_eq!(ctx.conversation_histories.lock().unwrap()[&snapshot.scope.key()].len(), 2);
}

#[tokio::test]
async fn channel_command_cancel_waits_for_feedback_before_releasing_scope() {
    let name = concat!(
        module_path!(),
        "::channel_command_cancel_waits_for_feedback_before_releasing_scope"
    );
    if isolated_cancel_test(name).await {
        return;
    }
    let (_tmp, ctx, _channel, msg, stream, _session) = cancel_fixture(false).await;
    let channel = Arc::new(RecordingChannel::feedback_blocking("lark"));
    let mut blocked_ctx = ctx.as_ref().clone();
    blocked_ctx.channels_by_name =
        Arc::new(HashMap::from([("lark".into(), channel.clone() as Arc<dyn Channel>)]));
    let ctx = Arc::new(blocked_ctx);
    let worker =
        tokio::spawn(process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()));
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

#[tokio::test]
async fn channel_command_cancel_dropped_caller_still_drains_processor() {
    let name =
        concat!(module_path!(), "::channel_command_cancel_dropped_caller_still_drains_processor");
    if isolated_cancel_test(name).await {
        return;
    }
    let (_tmp, ctx, _channel, msg, stream, session) = cancel_fixture(false).await;
    let worker =
        tokio::spawn(process_channel_message(ctx.clone(), msg.clone(), CancellationToken::new()));
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
    process_channel_message(ctx.clone(), new, CancellationToken::new()).await;
    let snapshot = resolve_channel_message_snapshot(&ctx, &msg).await.unwrap();
    assert_ne!(sender_session_store().lock().unwrap().get(&snapshot.scope), Some(&session));
}
