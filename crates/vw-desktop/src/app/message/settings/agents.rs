//! 智能体领域更新的桌面装配与跨域效果映射。
use crate::app::{App, Message};
use super::{AgentsMessage, SettingsMessage};
use iced::Task;
use desktop_settings_agents::{agents_update as domain, agents_update_message::{Message as Local, SettingsMessage as LocalSettings, Effect, ToastKind}};
fn event(message: Local) -> Message { match message {
 Local::Settings(LocalSettings::Agents(message)) => Message::Settings(SettingsMessage::Agents(message)),
 Local::ConfigSaved { tag, result } => Message::Settings(SettingsMessage::AgentConfigSaved { tag, result }),
} }
fn effects(app: &mut App, effects: Vec<Effect>, task: Task<Local>) -> Task<Message> {
 let mut tasks = vec![task.map(event)];
 for effect in effects { tasks.push(match effect {
 Effect::Toast(kind, text, args) => app.show_localized_toast(match kind { ToastKind::Error => crate::app::state::ToastKind::Error, ToastKind::Success => crate::app::state::ToastKind::Success, ToastKind::Warning => crate::app::state::ToastKind::Warning }, text, args),
 Effect::StartupEdit => { #[cfg(not(target_arch="wasm32"))] app.note_startup_setting_edit("agents_settings"); Task::none() },
 Effect::EditAcp(backend) => super::acp::update(app, SettingsMessage::Acp(super::AcpMessage::EditAgent(backend))),
 Effect::RefreshAcp => super::acp::update(app, SettingsMessage::Acp(super::AcpMessage::Refresh)),
 Effect::RefreshMcp => super::mcp::update(app, SettingsMessage::Mcp(super::McpMessage::InspectRefresh)),
 Effect::Launch { project, key } => {
 let draft = crate::app::message::project::open_session_draft(app, project.clone(), project);
 let selection = crate::app::message::chat::session::update(app, crate::app::message::chat::ChatMessage::SessionAgentSelected(Some(key)));
 app.navigation_panels.show_system_settings = false; app.navigation_panels.show_settings = false; app.navigation_tabs.screen = crate::app::Screen::Project;
 Task::batch([draft, selection])
 }
 }); }
 Task::batch(tasks)
}
fn inputs(app: &mut App, validation_error: Option<String>) -> domain::Update<'_> {
 domain::Update { state: &mut app.agents_settings, access: crate::app::config::agent_config_access(), client: crate::app::config::gateway_client(), acp_agents: &app.acp_agents, acp_loading: app.acp_settings.loading, new_session_creating: app.new_session_creating, project_path: app.navigation_tabs.project_path.clone(), validation_error, effects: Vec::new() }
}
pub(super) fn config_task(project_path: Option<String>) -> Task<Message> { domain::config_task(crate::app::config::agent_config_access(), crate::app::config::gateway_client(), project_path).map(event) }
pub(in crate::app) use domain::merge_catalog_profiles;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
 let SettingsMessage::Agents(message) = message else { return Task::none(); };
 let validation = if matches!(message, AgentsMessage::ProfileSave) { app.agents_settings.profile_modal.as_ref().and_then(|draft| crate::app::components::system_settings_agents::defaults::validation_error(app, draft)) } else { None };
 let mut input = inputs(app, validation);
 let task = domain::update(&mut input, LocalSettings::Agents(message));
 let pending = std::mem::take(&mut input.effects); drop(input);
 effects(app, pending, task)
}
pub(super) fn handle_agents_config_saved(app: &mut App, result: Result<(), String>) -> Task<Message> {
 let mut input = inputs(app, None); let task = domain::handle_agents_config_saved(&mut input, result);
 let pending = std::mem::take(&mut input.effects); drop(input); effects(app, pending, task)
}
#[cfg(test)] #[path="agents_tests.rs"] mod tests;
