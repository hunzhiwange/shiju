use crate::app::{App, Message};
use crate::app::message::SettingsMessage;
use desktop_settings_agents::dialogue_flow_message::Message as Local;
use desktop_settings_agents::dialogue_flow_inputs::View;
use iced::Element;
fn map(message: Local) -> Message { match message {
        Local::DialogueFlowExpandEditToolSectionToggled(a) => Message::Settings(SettingsMessage::DialogueFlowExpandEditToolSectionToggled(a)),
        Local::DialogueFlowExpandShellToolSectionToggled(a) => Message::Settings(SettingsMessage::DialogueFlowExpandShellToolSectionToggled(a)),
        Local::DialogueFlowShowReasoningSummaryToggled(a) => Message::Settings(SettingsMessage::DialogueFlowShowReasoningSummaryToggled(a)),
Local::None => Message::None, Local::CopyCode(value) => Message::CopyCode(value),
}}
fn inputs(app: &App) -> View<'_> { View {dialogue_flow_show_reasoning_summary: app.dialogue_flow_show_reasoning_summary,dialogue_flow_expand_shell_tool_section: app.dialogue_flow_expand_shell_tool_section,dialogue_flow_expand_edit_tool_section: app.dialogue_flow_expand_edit_tool_section,dialogue_flow_settings_save_message: &app.dialogue_flow_settings_save_message,dialogue_flow_settings_translation: &app.dialogue_flow_settings_translation, desktop_locale: app.desktop_locale, app_theme: &app.app_theme,  help_text: |i18n, theme, title, label, body| crate::app::components::system_settings_common::settings_hover_help_label(i18n, theme, title, label, body).map(|m| match m { Message::CopyCode(v) => Local::CopyCode(v), _ => Local::None }), }}
pub fn view(app: &App) -> Element<'_, Message> { desktop_settings_agents::system_settings_dialogue_flow::view(&inputs(app)).map(map) }

#[cfg(test)]
#[path = "system_settings_dialogue_flow_ui_refresh_29_tests.rs"]
mod system_settings_dialogue_flow_ui_refresh_29_tests;
