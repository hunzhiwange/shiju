//! 通道监听任务的监督循环，负责健康心跳与失败后的退避重启。
//!
//! 监听返回或失败后不会结束监督任务，而是按上限受控的指数退避重新进入监听。

use super::*;

/// 使用生产健康心跳间隔启动受监督的通道监听任务。
pub(crate) fn spawn_supervised_listener(
    ch: ConfiguredChannel,
    tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    initial_backoff_secs: u64,
    max_backoff_secs: u64,
) -> tokio::task::JoinHandle<()> {
    spawn_supervised_listener_with_health_interval(
        ch,
        tx,
        initial_backoff_secs,
        max_backoff_secs,
        Duration::from_secs(CHANNEL_HEALTH_HEARTBEAT_SECS),
    )
}

/// 使用显式心跳间隔启动监听任务，主要供可控时钟测试复用。
pub(crate) fn spawn_supervised_listener_with_health_interval(
    ch: ConfiguredChannel,
    tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    initial_backoff_secs: u64,
    max_backoff_secs: u64,
    health_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    let health_interval =
        if health_interval.is_zero() { Duration::from_secs(1) } else { health_interval };

    crate::app::agent::util::spawn(async move {
        let component = format!("channel:{}", ch.channel.name());
        let mut backoff = initial_backoff_secs.max(1);
        let max_backoff = max_backoff_secs.max(backoff);

        loop {
            crate::app::agent::health::mark_component_ok(&component);
            let mut health = tokio::time::interval(health_interval);
            health.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let result = {
                let listen_future = ch.listen(tx.clone());
                tokio::pin!(listen_future);

                loop {
                    tokio::select! {
                        _ = health.tick() => {
                            crate::app::agent::health::mark_component_ok(&component);
                        }
                        result = &mut listen_future => break result,
                    }
                }
            };

            if tx.is_closed() {
                break;
            }

            match result {
                Ok(()) => {
                    tracing::warn!("Channel {} exited unexpectedly; restarting", ch.channel.name());
                    crate::app::agent::health::mark_component_error(
                        &component,
                        "listener exited unexpectedly",
                    );
                    // Clean exit — reset backoff since the listener ran successfully
                    backoff = initial_backoff_secs.max(1);
                }
                Err(e) => {
                    tracing::error!("Channel {} error: {e}; restarting", ch.channel.name());
                    crate::app::agent::health::mark_component_error(&component, e.to_string());
                }
            }

            crate::app::agent::health::bump_component_restart(&component);
            tokio::time::sleep(Duration::from_secs(backoff)).await;
            // Double backoff AFTER sleeping so first error uses initial_backoff
            backoff = backoff.saturating_mul(2).min(max_backoff);
        }
    })
}

#[cfg(test)]
#[path = "listener_tests.rs"]
mod listener_tests;
