//! 通道"正在输入"状态管理模块
//!
//! 本模块提供了管理通道"正在输入"状态的功能，通过定期发送输入状态信号
//! 来提升用户体验。主要用于在代理处理任务期间向用户显示活动指示。
//!
//! # 核心功能
//!
//! - 周期性发送"正在输入"状态到指定通道
//! - 支持通过取消令牌优雅停止
//! - 自动清理：在任务结束时停止输入状态

use super::*;

/// 启动一个作用域化的"正在输入"状态任务
///
/// 创建一个异步任务，定期向指定通道的接收者发送"正在输入"状态信号。
/// 该任务会持续运行直到取消令牌被触发，并在退出时自动清理状态。
///
/// # 参数
///
/// * `channel` - 通道实例的 Arc 智能指针，用于发送输入状态信号
/// * `recipient` - 接收者标识符（例如用户 ID 或频道 ID）
/// * `cancellation_token` - 取消令牌，用于控制任务的停止
///
/// # 返回值
///
/// 返回一个 `JoinHandle<()>`，可用于等待任务完成或强制中止
///
/// # 行为说明
///
/// 1. 任务每隔 `CHANNEL_TYPING_REFRESH_INTERVAL_SECS` 秒发送一次输入状态
/// 2. 使用跳过错过的 tick 策略，避免在系统延迟后连续发送
/// 3. 当取消令牌被触发时，任务会：
///    - 立即退出循环
///    - 尝试停止输入状态（清理工作）
/// 4. 发送失败时仅记录调试日志，不会导致任务失败
///
/// # 示例
///
/// ```ignore
/// use tokio_util::sync::CancellationToken;
/// use std::sync::Arc;
///
/// let channel: Arc<dyn Channel> = /* ... */;
/// let recipient = "user_123".to_string();
/// let token = CancellationToken::new();
///
/// let handle = spawn_scoped_typing_task(channel, recipient, token.clone());
///
/// // 当需要停止时
/// token.cancel();
/// handle.await.ok();
/// ```
pub fn spawn_scoped_typing_task(
    channel: Arc<dyn Channel>,
    recipient: String,
    cancellation_token: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    let stop_signal = cancellation_token;
    let refresh_interval = Duration::from_secs(CHANNEL_TYPING_REFRESH_INTERVAL_SECS);

    let handle = agent_util::spawn(async move {
        let mut interval = tokio::time::interval(refresh_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                () = stop_signal.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(e) = channel.start_typing(&recipient).await {
                        tracing::debug!("Failed to start typing on {}: {e}", channel.name());
                    }
                }
            }
        }

        if let Err(e) = channel.stop_typing(&recipient).await {
            tracing::debug!("Failed to stop typing on {}: {e}", channel.name());
        }
    });

    handle
}

#[cfg(test)]
#[path = "typing_tests.rs"]
mod typing_tests;
