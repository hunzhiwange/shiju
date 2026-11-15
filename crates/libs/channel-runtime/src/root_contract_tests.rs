use super::{ConfiguredChannel, ChannelHealthState};
use channel_types::Channel;
use channel_cli::CliChannel;
use std::sync::Arc;

#[test]
fn configured_channel_new_stores_display_name() {
    let channel: Arc<dyn Channel> = Arc::new(CliChannel::new());
    let configured = ConfiguredChannel::new("My CLI", channel);
    assert_eq!(configured.display_name, "My CLI");
    assert_eq!(configured.channel.name(), "cli");
}

#[test]
fn channel_health_state_variants_compare() {
    assert_eq!(ChannelHealthState::Healthy, ChannelHealthState::Healthy);
    assert_ne!(ChannelHealthState::Healthy, ChannelHealthState::Unhealthy);
    assert_ne!(ChannelHealthState::Unhealthy, ChannelHealthState::Timeout);
}
