use crate::app::{App, Message};
use crate::app::message::SettingsMessage;
use desktop_settings_agents::dialogue_flow_message::Message as Local;
use desktop_settings_agents::dialogue_flow_inputs::View;
use iced::Element;
fn map(message: Local) -> Message { match message {
        Local::DialogueFlowExpandEditToolSectionToggled(a) => Message::Settings(SettingsMessage::DialogueFlowExpandEditToolSectionToggled(a)),
        Local::DialogueFlowExpandShellToolSectionToggled(a) => Message::Settings(SettingsMessage::DialogueFlowExpandShellToolSectionToggled(a)),
        Local::DialogueFlowShowReasoningSummaryToggled(a) => Message::Settings(SettingsMessage::DialogueFlowShowReasoningSummaryToggled(a)),
Local::None => Message::None, Local::CopyCode(value) => Message::CopyCode(value), Local::OpenUrlExternal(value) => Message::View(crate::app::message::ViewMessage::OpenUrlExternal(value)), Local::AppendText(value) => Message::Chat(crate::app::message::ChatMessage::AppendText(value)),
}}
fn inputs(app: &App) -> View<'_> { View {dialogue_flow_show_reasoning_summary: app.general_settings.dialogue_flow_show_reasoning_summary,dialogue_flow_expand_shell_tool_section: app.general_settings.dialogue_flow_expand_shell_tool_section,dialogue_flow_expand_edit_tool_section: app.general_settings.dialogue_flow_expand_edit_tool_section,dialogue_flow_settings_save_message: &app.general_settings.dialogue_flow_settings_save_message,dialogue_flow_settings_translation: &app.general_settings.dialogue_flow_settings_translation, desktop_locale: app.general_settings.desktop_locale, app_theme: &app.appearance.app_theme,  help_text: |i18n, theme, title, label, body| iced::widget::row![Element::from(crate::app::components::system_settings_common::settings_hover_help_label(i18n, theme, title, label, body)).map(|m| match m { Message::CopyCode(v) => Local::CopyCode(v), Message::View(crate::app::message::ViewMessage::OpenUrlExternal(value)) => Local::OpenUrlExternal(value), Message::Chat(crate::app::message::ChatMessage::AppendText(value)) => Local::AppendText(value), _ => Local::None })], }}
pub fn view(app: &App) -> Element<'_, Message> { desktop_settings_agents::system_settings_dialogue_flow::view(&inputs(app)).map(map) }

#[cfg(test)]
#[path = "system_settings_dialogue_flow_ui_refresh_29_tests.rs"]
mod system_settings_dialogue_flow_ui_refresh_29_tests;

#[cfg(test)]
#[path = "settings_help_routing_tests.rs"]
mod routing_tests;
