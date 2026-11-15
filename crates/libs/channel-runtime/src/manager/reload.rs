//! 通道运行时重载；旧监听器全部退出后才启动下一版，避免重复消费。

use super::*;
use std::sync::Mutex;
use channel_support::scope::ChannelProjectState;
#[cfg(not(target_arch = "wasm32"))]
use agent_config::schema::ChannelsConfigExt;

#[derive(Default)]
pub struct ChannelListeners {
    pub handles: Vec<tokio::task::JoinHandle<()>>,
    project_state: Arc<Mutex<ChannelProjectState>>,
}

impl ChannelListeners {
    pub fn configure_project_state(
        &self,
        config: &Config,
        tenant: vw_api_transport::id::TenantScope,
    ) -> Arc<Mutex<ChannelProjectState>> {
        let mut state = self.project_state.lock().unwrap_or_else(|e| e.into_inner());
        state.tenant = tenant;
        state.config = config.channels_config.clone();
        self.project_state.clone()
    }

    pub async fn stop(&mut self) {
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
pub async fn run_channels_with_source(
    host: &'static dyn crate::startup_host::ChannelStartupHost,
    mut config: Config,
    source: &mut dyn crate::startup_host::ChannelConfigSource,
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
                agent_health::mark_component_ok("channels");
            }
            let runtime = super::start::run_channels(host, config.clone(), &mut listeners);
            tokio::pin!(runtime);
            let mut running = enabled;
            loop {
                tokio::select! {
                    result = &mut runtime, if running => {
                        running = false;
                        if result.is_err() {
                            agent_health::mark_component_error("channels", "channel initialization failed");
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
