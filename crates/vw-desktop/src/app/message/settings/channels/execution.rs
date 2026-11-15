use crate::app::{App, Message};
use iced::Task;
#[cfg(test)]
use config_agent_profile::AgentModelReference;
#[cfg(test)]
use crate::app::state::ChannelExecutionChoices;
#[cfg(test)]
fn tenant() -> Result<String, String> {
 serde_json::to_string(crate::app::config::gateway_client()?.endpoint().tenant_scope()).map_err(|e| e.to_string())
}
pub(super) fn load_active(app: &mut App) -> Task<Message> {
 let services = super::updates::services(app);
 desktop_settings_channels::update::execution::load_active(&mut app.channels_settings, &services).map(super::updates::map_effect)
}
#[cfg(test)]
pub(super) fn select_provider(app: &mut App, instance: String, provider: Option<String>) -> Task<Message> {
 let services = super::updates::services(app);
 desktop_settings_channels::update::execution::select_provider(&mut app.channels_settings, &services, instance, provider).map(super::updates::map_effect)
}
#[cfg(test)]
pub(super) fn select_model(app: &mut App, instance: String, model: Option<AgentModelReference>) -> Task<Message> {
 let services = super::updates::services(app);
 desktop_settings_channels::update::execution::select_model(&mut app.channels_settings, &services, instance, model).map(super::updates::map_effect)
}
#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
