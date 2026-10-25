//! 通道消息分发模块
//!
//! 本模块提供通道消息的分发和调度功能，包括：
//! - 并发消息处理的流量控制
//! - 任务生命周期管理
//! - 基于 Telegram 的消息中断机制
//!
//! 主要组件：
//! - [`compute_max_in_flight_messages`]: 根据通道数量计算最大并发消息数
//! - [`log_worker_join_result`]: 记录工作线程的执行结果
//! - [`run_message_dispatch_loop`]: 消息分发主循环

use super::*;

/// 根据通道数量计算最大并发处理消息数
///
/// 该函数基于通道数量和每个通道的并行度配置，计算系统可以同时处理的最大消息数。
/// 计算结果会被限制在预定义的最小值和最大值范围内，以防止资源过度消耗。
///
/// # 参数
///
/// - `channel_count`: 当前活跃的通道数量
///
/// # 返回值
///
/// 返回计算后的最大并发消息数，范围在 [`CHANNEL_MIN_IN_FLIGHT_MESSAGES`] 和
/// [`CHANNEL_MAX_IN_FLIGHT_MESSAGES`] 之间
///
/// # 计算逻辑
///
/// 1. 使用饱和乘法计算 `channel_count * CHANNEL_PARALLELISM_PER_CHANNEL`
/// 2. 将结果限制在最小值和最大值之间（clamp 操作）
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::channels::manager::dispatch::compute_max_in_flight_messages;
///
/// let max_messages = compute_max_in_flight_messages(5);
/// // 返回值将在最小和最大限制之间
/// ```
pub(crate) fn compute_max_in_flight_messages(channel_count: usize) -> usize {
    channel_count
        .saturating_mul(CHANNEL_PARALLELISM_PER_CHANNEL)
        .clamp(CHANNEL_MIN_IN_FLIGHT_MESSAGES, CHANNEL_MAX_IN_FLIGHT_MESSAGES)
}

/// 记录工作线程的执行结果
///
/// 当工作线程完成任务后，检查其执行结果。如果任务发生崩溃或错误，
/// 通过日志系统记录错误信息，便于问题排查和监控。
///
/// # 参数
///
/// - `result`: Tokio 任务的 `JoinHandle` 的执行结果
///   - `Ok(())`: 任务正常完成
///   - `Err(JoinError)`: 任务崩溃或被取消
///
/// # 日志输出
///
/// 仅在任务失败时输出错误级别日志，包含具体的错误信息。
/// 正常完成的任务不会产生日志输出，避免日志噪音。
///
/// # 使用场景
///
/// 在消息分发循环中，用于处理已完成的 worker 任务结果：
/// - 立即尝试回收已完成任务时
/// - 等待所有任务完成时
pub(crate) fn log_worker_join_result(result: Result<(), tokio::task::JoinError>) {
    if let Err(error) = result {
        tracing::error!("Channel message worker crashed: {error}");
    }
}

/// 运行消息分发主循环
///
/// 该函数是通道消息处理的核心调度循环，负责：
/// 1. 接收来自各通道的消息
/// 2. 控制并发处理数量，防止系统过载
/// 3. 为每个消息分配独立的工作线程
/// 4. 实现 Telegram 通道的消息中断机制
/// 5. 管理任务生命周期和清理
///
/// # 参数
///
/// - `rx`: 消息接收器，用于接收来自各通道的 [`traits::ChannelMessage`]
/// - `ctx`: 通道运行时上下文，包含配置和共享状态
/// - `max_in_flight_messages`: 最大并发处理消息数，通过 [`compute_max_in_flight_messages`] 计算
///
/// # 并发控制
///
/// 使用 `tokio::sync::Semaphore` 实现信号量机制，确保同时处理的消息数不超过限制。
/// 每个消息处理任务在开始前需要获取信号量许可，完成后自动释放。
///
/// # 中断机制
///
/// 对于 Telegram 通道且启用了中断功能时，如果同一发送者有新消息到达：
/// - 取消正在处理的旧消息
/// - 等待旧消息完全停止
/// - 开始处理新消息
///
/// 这样可以确保用户获得最新的响应，而不是过时的结果。
///
/// # 任务管理
///
/// 使用 [`crate::app::agent::util::task_set::TaskSet`] 管理所有工作线程：
/// - 在主循环中尽可能回收已完成任务
/// - 循环结束后等待所有剩余任务完成
///
/// # 优雅关闭
///
/// 当接收器关闭或信号量被关闭时，循环会：
/// 1. 停止接收新消息
/// 2. 等待所有正在进行的任务完成
/// 3. 记录所有任务的执行结果
///
/// # 示例
///
/// ```ignore
/// use std::sync::Arc;
/// use tokio::sync::mpsc;
///
/// let (tx, rx) = mpsc::channel(100);
/// let ctx = Arc::new(ChannelRuntimeContext::new(/* ... */));
/// let max_in_flight = compute_max_in_flight_messages(5);
///
/// tokio::spawn(run_message_dispatch_loop(rx, ctx, max_in_flight));
/// ```
pub(crate) async fn run_message_dispatch_loop(
    mut rx: tokio::sync::mpsc::Receiver<traits::ChannelMessage>,
    ctx: Arc<ChannelRuntimeContext>,
    max_in_flight_messages: usize,
) {
    // 创建信号量用于控制并发数量，限制同时处理的消息数
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_in_flight_messages));
    // 任务集合，用于管理和追踪所有工作线程
    let mut workers = crate::app::agent::util::task_set::TaskSet::new();
    // 按发送者维护正在执行的任务状态，用于实现中断机制
    let in_flight_by_sender =
        Arc::new(tokio::sync::Mutex::new(HashMap::<String, InFlightSenderTaskState>::new()));
    // 任务序列号生成器，用于唯一标识每个任务
    let task_sequence = Arc::new(AtomicU64::new(1));

    // 主循环：持续接收消息直到接收器关闭
    while let Some(msg) = rx.recv().await {
        // 尝试获取信号量许可，如果信号量已关闭则退出循环
        let permit = match Arc::clone(&semaphore).acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => break,
        };

        // 克隆必要的上下文和状态用于工作线程
        let worker_ctx = Arc::clone(&ctx);
        let in_flight = Arc::clone(&in_flight_by_sender);
        let task_sequence = Arc::clone(&task_sequence);

        // 生成工作线程处理该消息
        workers.spawn(async move {
            // 持有信号量许可，任务完成时自动释放
            let _permit = permit;
            // 检查是否启用中断机制（仅对 Telegram 通道启用）
            let interrupt_enabled = worker_ctx.interrupt_on_new_message
                && msg.channel == "telegram"
                && !matches!(
                    parse_runtime_command(&msg.channel, &msg.content),
                    Some(
                        ChannelRuntimeCommand::ListProjects
                            | ChannelRuntimeCommand::ShowProject
                            | ChannelRuntimeCommand::SelectProject(_)
                    )
                );
            // 生成中断作用域键，用于标识同一发送者的任务
            let sender_scope_key = interruption_scope_key(&msg);
            // 创建取消令牌，用于中断正在执行的任务
            let cancellation_token = CancellationToken::new();
            // 创建完成标记，用于等待任务真正结束
            let completion = Arc::new(InFlightTaskCompletion::new());
            // 分配唯一的任务 ID
            let task_id = task_sequence.fetch_add(1, Ordering::Relaxed);

            // 如果启用中断机制，注册当前任务并检查是否有需要中断的旧任务
            if interrupt_enabled {
                // 将当前任务注册到活动任务映射中，同时取出之前可能存在的任务
                let previous = {
                    let mut active = in_flight.lock().await;
                    active.insert(
                        sender_scope_key.clone(),
                        InFlightSenderTaskState {
                            task_id,
                            cancellation: cancellation_token.clone(),
                            completion: Arc::clone(&completion),
                        },
                    )
                };

                // 如果存在该发送者的旧任务，则取消并等待其完成
                if let Some(previous) = previous {
                    tracing::info!(
                        channel = %msg.channel,
                        sender = %msg.sender,
                        "Interrupting previous in-flight request for sender"
                    );
                    // 触发旧任务的取消
                    previous.cancellation.cancel();
                    // 等待旧任务真正停止执行
                    previous.completion.wait().await;
                }
            }

            // 实际处理通道消息
            process_channel_message(worker_ctx, msg, cancellation_token).await;

            // 如果启用了中断机制，从活动映射中移除当前任务
            if interrupt_enabled {
                let mut active = in_flight.lock().await;
                // 只有当映射中的任务 ID 匹配时才移除，避免误删新任务
                if active.get(&sender_scope_key).is_some_and(|state| state.task_id == task_id) {
                    active.remove(&sender_scope_key);
                }
            }

            // 标记任务已完成，允许等待者继续执行
            completion.mark_done();
        });

        // 非阻塞地回收已完成的工作线程，避免任务集合无限增长
        while let Some(result) = workers.try_join_next() {
            log_worker_join_result(result);
        }
    }

    // 接收器已关闭，等待所有剩余工作线程完成
    while let Some(result) = workers.join_next().await {
        log_worker_join_result(result);
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod dispatch_tests;
