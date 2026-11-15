//! 领域更新效果的宿主持久化适配。
use crate::app::{App, Message};
use iced::Task;
use super::messages::*;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
use desktop_settings_runtime::messages::SettingsMessage as Local;
let message = match message {
SettingsMessage::ObservabilityBackendChanged(value) => Local::ObservabilityBackendChanged(value),
SettingsMessage::ObservabilityOtelEndpointChanged(value) => Local::ObservabilityOtelEndpointChanged(value),
SettingsMessage::ObservabilityOtelServiceNameChanged(value) => Local::ObservabilityOtelServiceNameChanged(value),
SettingsMessage::ObservabilityRuntimeTraceModeChanged(value) => Local::ObservabilityRuntimeTraceModeChanged(value),
SettingsMessage::ObservabilityRuntimeTracePathChanged(value) => Local::ObservabilityRuntimeTracePathChanged(value),
SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(value) => Local::ObservabilityRuntimeTraceMaxEntriesChanged(value),
SettingsMessage::ObservabilityHelpOpen => Local::ObservabilityHelpOpen,
SettingsMessage::ObservabilityHelpClose => Local::ObservabilityHelpClose,
_ => return Task::none(),
};
let Some(config) = desktop_settings_runtime::observability::update(&mut app.observability_settings, message) else { return Task::none(); };
crate::app::update_observability_config_async(move |current| *current = config)
}
#[cfg(test)]
#[path = "observability_tests.rs"]
mod observability_tests;
