//! 宿主输入装配与领域消息映射。
use crate::app::{App, Message};
use iced::Element;
use desktop_settings_runtime::system_settings_hooks as domain;
fn inputs(app: &App) -> domain::View<'_> {
 domain::View { hooks_settings: &app.hooks_settings,
 desktop_locale: app.desktop_locale, app_theme: &app.app_theme,

 render_help: super::system_settings_hooks::render_help,
 }
}
pub fn view(app: &App) -> Element<'_, Message> {
 domain::view(inputs(app)).map(super::system_settings_hooks::map_message)
}
#[cfg(test)]
#[path = "system_settings_hooks_ui_refresh_28_tests.rs"]
mod ui_refresh_28_tests;

pub(super) fn map_message(message: desktop_settings_runtime::Message) -> Message {
 use desktop_settings_runtime::{Message as Local, messages::SettingsMessage as S};
 use crate::app::message::settings::SettingsMessage as Global;
 match message {
 Local::None => Message::None, Local::CopyCode(text) => Message::CopyCode(text),
 Local::Settings(message) => Message::Settings(match message {
S::Hooks(value) => Global::Hooks(value),
S::Runtime(value) => Global::Runtime(value),
S::Multimodal(value) => Global::Multimodal(value),
S::QueryClassification(value) => Global::QueryClassification(value),
S::ReliabilityProviderRetriesChanged(value) => Global::ReliabilityProviderRetriesChanged(value),
S::ReliabilityProviderBackoffMsChanged(value) => Global::ReliabilityProviderBackoffMsChanged(value),
S::ReliabilityChannelInitialBackoffSecsChanged(value) => Global::ReliabilityChannelInitialBackoffSecsChanged(value),
S::ReliabilityChannelMaxBackoffSecsChanged(value) => Global::ReliabilityChannelMaxBackoffSecsChanged(value),
S::ReliabilitySchedulerPollSecsChanged(value) => Global::ReliabilitySchedulerPollSecsChanged(value),
S::ReliabilitySchedulerRetriesChanged(value) => Global::ReliabilitySchedulerRetriesChanged(value),
S::ReliabilitySave => Global::ReliabilitySave,
S::ReliabilityHelpOpen => Global::ReliabilityHelpOpen,
S::ReliabilityHelpClose => Global::ReliabilityHelpClose,
S::ObservabilityBackendChanged(value) => Global::ObservabilityBackendChanged(value),
S::ObservabilityOtelEndpointChanged(value) => Global::ObservabilityOtelEndpointChanged(value),
S::ObservabilityOtelServiceNameChanged(value) => Global::ObservabilityOtelServiceNameChanged(value),
S::ObservabilityRuntimeTraceModeChanged(value) => Global::ObservabilityRuntimeTraceModeChanged(value),
S::ObservabilityRuntimeTracePathChanged(value) => Global::ObservabilityRuntimeTracePathChanged(value),
S::ObservabilityRuntimeTraceMaxEntriesChanged(value) => Global::ObservabilityRuntimeTraceMaxEntriesChanged(value),
S::ObservabilityHelpOpen => Global::ObservabilityHelpOpen,
S::ObservabilityHelpClose => Global::ObservabilityHelpClose,
S::NavigateModelRoutes => Global::ModelConfigTab(crate::app::message::settings::ModelConfigTabMessage::Selected(crate::app::state::ModelConfigTab::ModelRoutes)),
 }),
 }
}
pub(super) fn render_help<'a>(title: &'a str, description: std::borrow::Cow<'a, str>, color: Option<iced::Color>, i18n: desktop_i18n::I18n) -> Element<'a, desktop_settings_runtime::Message> {
 super::chat_panel::message_view::selectable_titled_text(title, description, color, i18n).map(|message| match message {
 Message::None => desktop_settings_runtime::Message::None,
 Message::CopyCode(text) => desktop_settings_runtime::Message::CopyCode(text),
 _ => unreachable!("plain help selection published a chat-only message"),
 })
}
