//! ACP 宿主装配：配置入口、启动编辑标记与 toast 映射。
use crate::app::{App, Message};
use super::messages::{SettingsMessage};
use desktop_settings_agents::{acp_update, acp_update_message::{Message as Local, Toast}};
use iced::Task;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
 let SettingsMessage::Acp(message) = message else { return Task::none(); };
 let mut inputs = acp_update::Update { state: &mut app.acp_settings, acp_agents: &mut app.session_state.acp_agents, acp_agent: &mut app.session_state.acp_agent, access: crate::app::config::agent_config_access(), client: crate::app::config::gateway_client(), snapshot_applied: false, toasts: Vec::new() };
 let task = acp_update::update(&mut inputs, message);
 #[cfg(not(target_arch="wasm32"))]
 let applied = inputs.snapshot_applied;
 let toasts = std::mem::take(&mut inputs.toasts);
 drop(inputs);
 #[cfg(not(target_arch="wasm32"))]
 if applied { app.note_startup_setting_edit("acp_agents"); }
 let task = task.then(|event| match event { Local::Acp(message) => Task::done(Message::Settings(SettingsMessage::Acp(message))), Local::After(duration, message) => crate::app::message::after(duration, Message::Settings(SettingsMessage::Acp(message))) });
 let mut tasks = vec![task];
 for toast in toasts { tasks.push(match toast { Toast::Error(text) => app.show_error_toast(text), Toast::Info(text) => app.show_info_toast(text), Toast::Success(text) => app.show_success_toast(text) }); }
 Task::batch(tasks)
}

#[cfg(test)]
#[path="acp_tests.rs"]
mod acp_tests;
