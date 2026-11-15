//! 通道启动的宿主组合入口。
use super::*;
pub(crate) use channel_runtime::manager::start::*;
pub async fn start_channels(config: Config) -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    return super::reload::start_channels_with_reload(config).await;
    #[cfg(target_arch = "wasm32")]
    channel_runtime::manager::start::run_channels(&super::runtime_host::HOST, config, &mut super::reload::ChannelListeners::default()).await
}
#[cfg(test)]
fn build_channel_hooks(config: &Config) -> Result<Option<Arc<crate::hooks::HookRunner>>> {
    channel_runtime::startup_host::ChannelStartupHost::hooks(&super::runtime_host::HOST, config)
}
#[cfg(test)]
#[path = "../../../../libs/channel-runtime/src/manager/start_tests.rs"]
mod start_tests;
