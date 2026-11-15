//! 领域更新效果的宿主持久化适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    use desktop_settings_runtime::messages::SettingsMessage as Local;
    let message = match message {
        SettingsMessage::ReliabilityProviderRetriesChanged(value) => Local::ReliabilityProviderRetriesChanged(value),
        SettingsMessage::ReliabilityProviderBackoffMsChanged(value) => Local::ReliabilityProviderBackoffMsChanged(value),
        SettingsMessage::ReliabilityChannelInitialBackoffSecsChanged(value) => Local::ReliabilityChannelInitialBackoffSecsChanged(value),
        SettingsMessage::ReliabilityChannelMaxBackoffSecsChanged(value) => Local::ReliabilityChannelMaxBackoffSecsChanged(value),
        SettingsMessage::ReliabilitySchedulerPollSecsChanged(value) => Local::ReliabilitySchedulerPollSecsChanged(value),
        SettingsMessage::ReliabilitySchedulerRetriesChanged(value) => Local::ReliabilitySchedulerRetriesChanged(value),
        SettingsMessage::ReliabilitySave => Local::ReliabilitySave,
        SettingsMessage::ReliabilityHelpOpen => Local::ReliabilityHelpOpen,
        SettingsMessage::ReliabilityHelpClose => Local::ReliabilityHelpClose,
        _ => return Task::none(),
    };
    let Some(config) = desktop_settings_runtime::reliability::update(&mut app.reliability_settings, message) else { return Task::none(); };
    crate::app::update_reliability_config_async(move |current| *current = config)
}

#[cfg(test)]
#[path = "reliability_tests.rs"]
mod reliability_tests;
