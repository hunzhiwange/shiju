//! 通道运行时重载；旧监听器全部退出后才启动下一版，避免重复消费。

use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::agent::config::schema::ChannelsConfigExt;

#[derive(Default)]
pub(super) struct ChannelListeners {
    pub(super) handles: Vec<tokio::task::JoinHandle<()>>,
}

impl ChannelListeners {
    pub(super) async fn stop(&mut self) {
        for handle in &self.handles {
            handle.abort();
        }
        for handle in self.handles.drain(..) {
            let _ = handle.await;
        }
    }
}

impl Drop for ChannelListeners {
    fn drop(&mut self) {
        for handle in &self.handles {
            handle.abort();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn start_channels_with_reload(config: Config) -> Result<()> {
    let source =
        crate::app::agent::config::schema::ChannelConfigWatcher::new(&config, true).await?;
    run_channels_with_source(config, source).await
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_channels_with_source(
    mut config: Config,
    mut source: crate::app::agent::config::schema::ChannelConfigWatcher,
) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut listeners = ChannelListeners::default();
    loop {
        let next = {
            // 不配置实时通道时不装配 Provider，但仍等待保存配置。
            let enabled =
                config.channels_config.channels_except_webhook().iter().any(|(_, ok)| *ok);
            if !enabled {
                crate::app::agent::health::mark_component_ok("channels");
            }
            let runtime = super::start::run_channels(config.clone(), &mut listeners);
            tokio::pin!(runtime);
            let mut running = enabled;
            loop {
                tokio::select! {
                    result = &mut runtime, if running => {
                        running = false;
                        if result.is_err() {
                            crate::app::agent::health::mark_component_error("channels", "channel initialization failed");
                            tracing::warn!("Channel initialization failed; waiting for configuration changes");
                        }
                    }
                    _ = interval.tick() => {
                        match source.poll(&config).await {
                            Ok(Some(channels)) => break channels,
                            Ok(None) => {}
                            Err(_) => tracing::warn!("Unable to reload channels; keeping current configuration"),
                        }
                    }
                }
            }
        };
        // runtime future 已释放，分发任务随之取消；再等待所有监听器释放连接。
        listeners.stop().await;
        config.channels_config = next;
        tracing::info!("Reloading channel listeners after configuration change");
    }
}

#[cfg(test)]
#[path = "reload_tests.rs"]
mod tests;
