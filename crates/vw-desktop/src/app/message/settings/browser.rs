//! 浏览器设置领域效果的宿主执行。
use crate::app::{App, Message};
use crate::app::config::{patch_full_agent_config_async, spawn_gateway_task};
use super::messages::{BrowserMessage, SettingsMessage};
use iced::Task;
use desktop_settings_network::message::browser::{self, Effect};
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
 let SettingsMessage::Browser(message) = message else { return Task::none(); };
 match browser::update(&mut app.browser_settings, message) {
  Effect::None => Task::none(),
  Effect::Success(text) => app.show_success_toast(text),
  Effect::Error(text) => app.show_error_toast(text),
  Effect::Persist(browser) => spawn_gateway_task("browser", async move { patch_full_agent_config_async(serde_json::json!({ "browser": browser })).await }),
  Effect::Install(backend) => Task::perform(async move { gateway_desktop_settings::GatewayDesktopSettingsClient::new(crate::app::config::gateway_client()?.transport().clone()).browser_install(&backend).await }, |result| Message::Settings(SettingsMessage::Browser(BrowserMessage::InstallCompleted(result)))),
 }
}

#[cfg(test)]
#[path = "browser_tests.rs"]
mod tests;
