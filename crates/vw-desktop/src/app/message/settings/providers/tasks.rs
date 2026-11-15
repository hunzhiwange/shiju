//! Inject the gateway and configuration access into provider refresh work.
use crate::app::{Message, config};
use super::super::messages::SettingsMessage;
use desktop_settings_providers::provider_message::{Message as Effect, SettingsMessage as Local};

pub fn refresh_task() -> iced::Task<Message> {
    desktop_settings_providers::provider_refresh::refresh_task(
        config::gateway_client_endpoint(), super::models::access(),
    ).map(|event| match event {
        Effect::Settings(Local::ProvidersRefreshed(result)) => {
            Message::Settings(SettingsMessage::ProvidersRefreshed(result))
        }
        _ => unreachable!("provider refresh result"),
    })
}
