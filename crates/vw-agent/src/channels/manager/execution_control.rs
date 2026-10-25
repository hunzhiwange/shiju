//! 当前聊天执行的取消与代次；直到处理器和反馈退出才释放忙碌状态。
use super::*;

pub(crate) fn active_channel_task(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> Option<InFlightSenderTaskState> {
    tracing::trace!(scope = %interruption_scope_key(msg), "Checking active channel task");
    let scope = channel_chat_scope(ctx, msg);
    ctx.project_state.lock().unwrap_or_else(|e| e.into_inner()).active.get(&scope).cloned()
}

pub(crate) async fn stop_channel_task(
    ctx: &ChannelRuntimeContext,
    msg: &traits::ChannelMessage,
) -> String {
    let Some(task) = active_channel_task(ctx, msg) else {
        return "当前聊天没有正在执行的任务。".into();
    };
    task.cancellation.cancel();
    if tokio::time::timeout(Duration::from_secs(2), task.completion.wait()).await.is_err() {
        return "已请求取消，任务或反馈仍在退出；尚未停止，请稍后重试 /stop。".into();
    }
    "当前聊天执行已停止。".into()
}

pub(crate) async fn send_channel_busy(ctx: &ChannelRuntimeContext, msg: &traits::ChannelMessage) {
    if let Some(channel) = channel_for_message(ctx, msg) {
        let _ = channel
            .send(
                &SendMessage::new(
                    "当前聊天繁忙，请先使用 /stop，等待停止确认后重试。",
                    &msg.reply_target,
                )
                .in_thread(msg.thread_ts.clone()),
            )
            .await;
    }
}

pub(crate) async fn process_channel_message(
    ctx: Arc<ChannelRuntimeContext>,
    msg: traits::ChannelMessage,
    cancellation: CancellationToken,
) {
    if cancellation.is_cancelled() {
        return;
    }
    log_inbound_message(&msg);
    let Some(msg) = apply_inbound_hooks(ctx.as_ref(), msg).await else { return };
    if cancellation.is_cancelled() {
        return;
    }
    let command = parse_runtime_command(&msg.channel, &msg.content);
    if let Some(command) = command {
        // 生命周期和项目变更由各自 handler 持有同一写锁；其他设置变更在此互斥。
        let guarded = matches!(
            command,
            ChannelRuntimeCommand::SetModel(_)
                | ChannelRuntimeCommand::SetProvider(_)
                | ChannelRuntimeCommand::TaskMode
        ) || matches!(&command, ChannelRuntimeCommand::Effort(value) if !value.is_empty());
        let gate = channel_chat_gate(&ctx, &msg);
        let _guard = if guarded {
            match gate.try_write() {
                Ok(guard) => Some(guard),
                Err(_) => {
                    send_channel_busy(&ctx, &msg).await;
                    return;
                }
            }
        } else {
            None
        };
        Box::pin(super::message::process_channel_message_inner(ctx, msg, cancellation)).await;
        return;
    }
    if let Some(previous) = active_channel_task(&ctx, &msg) {
        if ctx.interrupt_on_new_message && msg.channel == "telegram" {
            previous.cancellation.cancel();
            if tokio::time::timeout(Duration::from_secs(2), previous.completion.wait())
                .await
                .is_err()
            {
                send_channel_busy(&ctx, &msg).await;
                return;
            }
        } else {
            send_channel_busy(&ctx, &msg).await;
            return;
        }
    }
    let gate = channel_chat_gate(&ctx, &msg);
    let Ok(guard) = gate.try_write_owned() else {
        send_channel_busy(&ctx, &msg).await;
        return;
    };
    let scope = channel_chat_scope(&ctx, &msg);
    static SEQUENCE: AtomicU64 = AtomicU64::new(1);
    let id = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let completion = Arc::new(InFlightTaskCompletion::new());
    ctx.project_state.lock().unwrap_or_else(|e| e.into_inner()).active.insert(
        scope.clone(),
        InFlightSenderTaskState {
            task_id: id,
            cancellation: cancellation.clone(),
            completion: completion.clone(),
        },
    );
    // 调用者被重载/HTTP 超时释放时只请求取消，清理任务继续持有锁直到真实退出。
    let cancel_on_drop = cancellation.clone().drop_guard();
    let operation = async move {
        super::message::process_channel_message_inner(ctx.clone(), msg, cancellation).await;
        drop(guard);
        let mut state = ctx.project_state.lock().unwrap_or_else(|e| e.into_inner());
        if state.active.get(&scope).is_some_and(|task| task.task_id == id) {
            state.active.remove(&scope);
        }
        completion.mark_done();
    };
    #[cfg(not(target_arch = "wasm32"))]
    let worker = tokio::spawn(operation);
    #[cfg(target_arch = "wasm32")]
    let worker = tokio::task::spawn_local(operation);
    log_worker_join_result(worker.await);
    drop(cancel_on_drop);
}
