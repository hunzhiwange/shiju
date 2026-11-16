//! 对话流设置的桌面消息和持久化队列适配。
use crate::app::{App, Message};
use crate::app::config::{gateway_client_endpoint, SystemSettingsSection};
use super::messages::SettingsMessage;
use desktop_settings_agents::dialogue_flow_update as domain;
use iced::Task;
fn event(message: domain::Message) -> Message { Message::Settings(match message {
 domain::Message::DialogueFlowPermissionRefresh => SettingsMessage::DialogueFlowPermissionRefresh,
 domain::Message::DialogueFlowPermissionReset => SettingsMessage::DialogueFlowPermissionReset,
 domain::Message::DialogueFlowPermissionLoaded(v) => SettingsMessage::DialogueFlowPermissionLoaded(v),
 domain::Message::DialogueFlowUiSettingsLoaded(scope, revision, v) => SettingsMessage::DialogueFlowUiSettingsLoaded(scope, revision, v),
 domain::Message::DialogueFlowUiSettingsSave => SettingsMessage::DialogueFlowUiSettingsSave,
 domain::Message::DialogueFlowUiSettingsSaved(v) => SettingsMessage::DialogueFlowUiSettingsSaved(v),
 domain::Message::DialogueFlowShowReasoningSummaryToggled(v) => SettingsMessage::DialogueFlowShowReasoningSummaryToggled(v),
 domain::Message::DialogueFlowExpandShellToolSectionToggled(v) => SettingsMessage::DialogueFlowExpandShellToolSectionToggled(v),
 domain::Message::DialogueFlowExpandEditToolSectionToggled(v) => SettingsMessage::DialogueFlowExpandEditToolSectionToggled(v),
}) }
pub fn dialogue_flow_permission_text_task(path: Option<String>) -> Task<Message> { domain::dialogue_flow_permission_text_task(path).map(event) }
#[cfg(test)]
use crate::app::config::preferences_scope;
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
 let message = match message {
 SettingsMessage::DialogueFlowPermissionRefresh => domain::Message::DialogueFlowPermissionRefresh,
 SettingsMessage::DialogueFlowPermissionReset => domain::Message::DialogueFlowPermissionReset,
 SettingsMessage::DialogueFlowPermissionLoaded(v) => domain::Message::DialogueFlowPermissionLoaded(v),
 SettingsMessage::DialogueFlowUiSettingsLoaded(scope, revision, v) => domain::Message::DialogueFlowUiSettingsLoaded(scope, revision, v),
 SettingsMessage::DialogueFlowUiSettingsSave => domain::Message::DialogueFlowUiSettingsSave,
 SettingsMessage::DialogueFlowUiSettingsSaved(v) => domain::Message::DialogueFlowUiSettingsSaved(v),
 SettingsMessage::DialogueFlowShowReasoningSummaryToggled(v) => domain::Message::DialogueFlowShowReasoningSummaryToggled(v),
 SettingsMessage::DialogueFlowExpandShellToolSectionToggled(v) => domain::Message::DialogueFlowExpandShellToolSectionToggled(v),
 SettingsMessage::DialogueFlowExpandEditToolSectionToggled(v) => domain::Message::DialogueFlowExpandEditToolSectionToggled(v),
 _ => return Task::none(),
 };
 let mut input = domain::Update {
 endpoint: gateway_client_endpoint(), project_path: app.navigation_tabs.project_path.clone(),
 desktop_preferences_persistence: &mut app.desktop_preferences_persistence,
 dialogue_flow_permission_editor: &mut app.dialogue_flow_permission_editor,
 dialogue_flow_show_reasoning_summary: &mut app.dialogue_flow_show_reasoning_summary,
 dialogue_flow_expand_shell_tool_section: &mut app.dialogue_flow_expand_shell_tool_section,
 dialogue_flow_expand_edit_tool_section: &mut app.dialogue_flow_expand_edit_tool_section,
 dialogue_flow_settings_translation: &mut app.dialogue_flow_settings_translation,
 dialogue_flow_settings_save_message: &mut app.dialogue_flow_settings_save_message,
 patches: Vec::new(),
 };
 let task = domain::update(&mut input, message).map(event);
 let patches = std::mem::take(&mut input.patches); drop(input);
 Task::batch(std::iter::once(task).chain(patches.into_iter().map(|fields| app.save_system_settings_fields(SystemSettingsSection::DialogueFlow, fields))))
}
#[cfg(test)]
#[path="dialogue_flow_tests.rs"] mod dialogue_flow_tests;
