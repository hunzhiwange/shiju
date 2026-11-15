//! 配置监听器的宿主适配；重载循环与监听器状态归运行库。
use super::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use channel_runtime::manager::reload::ChannelListeners;
#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn start_channels_with_reload(config: Config) -> Result<()> {
    let source = crate::config::schema::ChannelConfigWatcher::new(&config, true).await?;
    run_channels_with_source(config, source).await
}
#[cfg(not(target_arch = "wasm32"))]
async fn run_channels_with_source(config: Config, mut source: crate::config::schema::ChannelConfigWatcher) -> Result<()> {
    channel_runtime::manager::reload::run_channels_with_source(&super::runtime_host::HOST, config, &mut source).await
}
#[cfg(not(target_arch = "wasm32"))]
#[async_trait::async_trait]
impl channel_runtime::startup_host::ChannelConfigSource for crate::config::schema::ChannelConfigWatcher {
    async fn poll(&mut self, config: &Config) -> Result<Option<crate::config::ChannelsConfig>> {
        self.poll(config).await
    }
}
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/reload_tests.rs"]
mod tests;
