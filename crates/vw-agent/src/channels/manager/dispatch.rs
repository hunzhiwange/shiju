//! 通道消息调度；控制命令不等待模型并发许可。
use super::*;

pub(crate) fn compute_max_in_flight_messages(channel_count: usize) -> usize {
    channel_count
        .saturating_mul(CHANNEL_PARALLELISM_PER_CHANNEL)
        .clamp(CHANNEL_MIN_IN_FLIGHT_MESSAGES, CHANNEL_MAX_IN_FLIGHT_MESSAGES)
}

pub(crate) fn log_worker_join_result(result: Result<(), tokio::task::JoinError>) {
    if let Err(error) = result {
        tracing::error!("Channel message worker crashed: {error}");
    }
}

pub(crate) async fn run_message_dispatch_loop(
    mut rx: tokio::sync::mpsc::Receiver<traits::ChannelMessage>,
    ctx: Arc<ChannelRuntimeContext>,
    max_in_flight_messages: usize,
) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_in_flight_messages));
    let mut workers = crate::app::agent::util::task_set::TaskSet::new();
    let mut pending = std::collections::VecDeque::<traits::ChannelMessage>::new();
    let mut closed = false;
    let mut worker_count = 0;
    while !closed || !pending.is_empty() || worker_count > 0 {
        tokio::select! {
            result = workers.join_next(), if worker_count > 0 => {
                if let Some(result) = result { log_worker_join_result(result); }
                worker_count -= 1;
            }
            permit = semaphore.clone().acquire_owned(), if !pending.is_empty() => {
                let Ok(permit) = permit else { break };
                let msg = pending.pop_front().expect("pending queue is not empty");
                let ctx = ctx.clone();
                worker_count += 1;
                workers.spawn(async move {
                    let _permit = permit;
                    process_channel_message(ctx, msg, CancellationToken::new()).await;
                });
            }
            message = rx.recv(), if !closed => {
                let Some(msg) = message else { closed = true; continue };
                let command = parse_runtime_command(&msg.channel, &msg.content);
                // 只读模型列表仍沿用并发限制，其余控制消息随时处理。
                let control = command.is_some() && !matches!(command, Some(ChannelRuntimeCommand::ShowModel | ChannelRuntimeCommand::ShowProviders));
                if matches!(command, Some(ChannelRuntimeCommand::Stop)) {
                    let scope = channel_chat_scope(&ctx, &msg);
                    pending.retain(|queued| channel_chat_scope(&ctx, queued) != scope);
                }
                if !control && pending.len() < 100 {
                    pending.push_back(msg);
                    continue;
                }
                let ctx = ctx.clone();
                worker_count += 1;
                workers.spawn(async move {
                    if control {
                        process_channel_message(ctx, msg, CancellationToken::new()).await;
                    } else {
                        send_channel_busy(&ctx, &msg).await;
                    }
                });
            }
        }
    }
    while let Some(result) = workers.join_next().await {
        log_worker_join_result(result);
    }
}
