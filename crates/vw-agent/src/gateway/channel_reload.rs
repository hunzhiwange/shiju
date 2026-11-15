//! Gateway channel factory composition (including the host test transport).
use super::state::{AppState, GatewayChannels};
use crate::config::{ChannelsConfig, Config};
use crate::channels::WhatsAppChannel;
use std::sync::Arc;
fn whatsapp(config: &ChannelsConfig) -> Option<Arc<WhatsAppChannel>> {
    let whatsapp_channel: Option<Arc<WhatsAppChannel>> =
        config.whatsapp.as_ref().filter(|wa| wa.is_cloud_config()).map(|wa| {
            Arc::new(WhatsAppChannel::new(
                wa.access_token.clone().unwrap_or_default(),
                wa.phone_number_id.clone().unwrap_or_default(),
                wa.verify_token.clone().unwrap_or_default(),
                wa.allowed_numbers.clone(),
            ))
        });

    whatsapp_channel
}
pub(super) fn build_gateway_channels(config: &ChannelsConfig) -> GatewayChannels {
    agent_gateway::channel_reload::build_gateway_channels_with(config, whatsapp)
}
pub(super) async fn watch_gateway_channels(state: AppState, config: Config) -> anyhow::Result<()> {
    let source = crate::config::schema::ChannelConfigWatcher::new(&config, false).await?;
    watch_gateway_channels_with_source(state, config, source).await
}
pub(super) async fn watch_gateway_channels_with_source(state: AppState, config: Config, source: crate::config::schema::ChannelConfigWatcher) -> anyhow::Result<()> {
    agent_gateway::channel_reload::watch_gateway_channels_with_source(state, config, source, whatsapp).await
}

impl agent_gateway::channel_reload::GatewayChannelSource for crate::config::schema::ChannelConfigWatcher {
    async fn poll(&mut self, config: &Config) -> anyhow::Result<Option<ChannelsConfig>> {
        self.poll(config).await
    }
}
