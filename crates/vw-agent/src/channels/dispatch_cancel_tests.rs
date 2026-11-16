use super::dispatch_tests::{RecordingChannel, context, message};
use super::*;
use channel_runtime::cancel_contracts::{CancelFixture, CancelSignals};

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


#[tokio::test]
async fn channel_command_cancel_qq_idle_is_idempotent() { channel_runtime::cancel_contracts::channel_command_cancel_qq_idle_is_idempotent(crate::channels::manager::contract_host()).await; }

#[tokio::test]
async fn channel_command_cancel_lark_idle_is_idempotent() { channel_runtime::cancel_contracts::channel_command_cancel_lark_idle_is_idempotent(crate::channels::manager::contract_host()).await; }

#[tokio::test]
async fn channel_command_cancel_telegram_does_not_require_auto_interrupt() { channel_runtime::cancel_contracts::channel_command_cancel_telegram_does_not_require_auto_interrupt(crate::channels::manager::contract_host()).await; }

#[tokio::test]
async fn channel_command_cancel_invalid_arguments_preserve_state() { channel_runtime::cancel_contracts::channel_command_cancel_invalid_arguments_preserve_state(crate::channels::manager::contract_host()).await; }

#[tokio::test]
async fn channel_command_cancel_help_describes_scope() { channel_runtime::cancel_contracts::channel_command_cancel_help_describes_scope(crate::channels::manager::contract_host()).await; }

#[tokio::test]
async fn channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session() {
    if isolated_cancel_test(concat!(module_path!(), "::channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session")).await { return; }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(false).await;
    let snapshot = channel_runtime::cancel_contracts::channel_command_cancel_bypasses_full_slots_and_cleans_before_new_session(crate::channels::manager::contract_host(), CancelFixture { ctx, channel, msg, stream: CancelSignals { started: &stream.started, exited: &stream.exited, release: &stream.release }, session: session.clone() }).await;
    let recorded = crate::session::processor::take_test_llm_messages(&session);
    assert_eq!(recorded.len(), 1);
    assert_eq!(std::fs::canonicalize(&recorded[0].directory).unwrap(), snapshot.directory);
    assert_eq!(recorded[0].tenant, snapshot.scope.chat.tenant);
}

#[tokio::test]
async fn channel_command_cancel_isolates_instance_room_thread_sender_and_tenant() {
    if isolated_cancel_test(concat!(module_path!(), "::channel_command_cancel_isolates_instance_room_thread_sender_and_tenant")).await { return; }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(false).await;
    channel_runtime::cancel_contracts::channel_command_cancel_isolates_instance_room_thread_sender_and_tenant(crate::channels::manager::contract_host(), CancelFixture { ctx, channel, msg, stream: CancelSignals { started: &stream.started, exited: &stream.exited, release: &stream.release }, session: session.clone() }).await;
}

#[tokio::test]
async fn channel_command_cancel_unconfirmed_exit_keeps_mutations_busy() {
    if isolated_cancel_test(concat!(module_path!(), "::channel_command_cancel_unconfirmed_exit_keeps_mutations_busy")).await { return; }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(true).await;
    channel_runtime::cancel_contracts::channel_command_cancel_unconfirmed_exit_keeps_mutations_busy(crate::channels::manager::contract_host(), CancelFixture { ctx, channel, msg, stream: CancelSignals { started: &stream.started, exited: &stream.exited, release: &stream.release }, session: session.clone() }).await;
}

#[tokio::test]
async fn channel_command_cancel_busy_commands_preserve_state_and_status_responds() {
    if isolated_cancel_test(concat!(module_path!(), "::channel_command_cancel_busy_commands_preserve_state_and_status_responds")).await { return; }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(false).await;
    channel_runtime::cancel_contracts::channel_command_cancel_busy_commands_preserve_state_and_status_responds(crate::channels::manager::contract_host(), CancelFixture { ctx, channel, msg, stream: CancelSignals { started: &stream.started, exited: &stream.exited, release: &stream.release }, session: session.clone() }).await;
}

#[tokio::test]
async fn channel_command_cancel_natural_completion_then_stop_is_idempotent() {
    if isolated_cancel_test(concat!(module_path!(), "::channel_command_cancel_natural_completion_then_stop_is_idempotent")).await { return; }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(false).await;
    channel_runtime::cancel_contracts::channel_command_cancel_natural_completion_then_stop_is_idempotent(crate::channels::manager::contract_host(), CancelFixture { ctx, channel, msg, stream: CancelSignals { started: &stream.started, exited: &stream.exited, release: &stream.release }, session: session.clone() }).await;
}

#[tokio::test]
async fn channel_command_cancel_waits_for_feedback_before_releasing_scope() {
    if isolated_cancel_test(concat!(module_path!(), "::channel_command_cancel_waits_for_feedback_before_releasing_scope")).await { return; }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(false).await;
    channel_runtime::cancel_contracts::channel_command_cancel_waits_for_feedback_before_releasing_scope(crate::channels::manager::contract_host(), CancelFixture { ctx, channel, msg, stream: CancelSignals { started: &stream.started, exited: &stream.exited, release: &stream.release }, session: session.clone() }).await;
}

#[tokio::test]
async fn channel_command_cancel_dropped_caller_still_drains_processor() {
    if isolated_cancel_test(concat!(module_path!(), "::channel_command_cancel_dropped_caller_still_drains_processor")).await { return; }
    let (_tmp, ctx, channel, msg, stream, session) = cancel_fixture(false).await;
    channel_runtime::cancel_contracts::channel_command_cancel_dropped_caller_still_drains_processor(crate::channels::manager::contract_host(), CancelFixture { ctx, channel, msg, stream: CancelSignals { started: &stream.started, exited: &stream.exited, release: &stream.release }, session: session.clone() }).await;
}
