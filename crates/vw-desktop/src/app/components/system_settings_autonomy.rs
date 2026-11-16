use crate::app::{App, Message};
use crate::app::message::SettingsMessage;
use desktop_settings_agents::autonomy_message::Message as Local;
use desktop_settings_agents::autonomy_inputs::View;
use iced::Element;
fn map(message: Local) -> Message { match message {
 Local::AutonomySave => Message::Settings(SettingsMessage::AutonomySave),
        Local::AutonomyBlockHighRiskCommandsToggled(a) => Message::Settings(SettingsMessage::AutonomyBlockHighRiskCommandsToggled(a)),
        Local::AutonomyHelpClose => Message::Settings(SettingsMessage::AutonomyHelpClose),
        Local::AutonomyHelpOpen => Message::Settings(SettingsMessage::AutonomyHelpOpen),
        Local::AutonomyLevelChanged(a) => Message::Settings(SettingsMessage::AutonomyLevelChanged(a)),
        Local::AutonomyListAdd(a) => Message::Settings(SettingsMessage::AutonomyListAdd(a)),
        Local::AutonomyListDraftChanged(a, b) => Message::Settings(SettingsMessage::AutonomyListDraftChanged(a, b)),
        Local::AutonomyListRemoveCanceled => Message::Settings(SettingsMessage::AutonomyListRemoveCanceled),
        Local::AutonomyListRemoveConfirmed(a, b) => Message::Settings(SettingsMessage::AutonomyListRemoveConfirmed(a, b)),
        Local::AutonomyListRemoveRequested(a, b) => Message::Settings(SettingsMessage::AutonomyListRemoveRequested(a, b)),
        Local::AutonomyMaxActionsPerHourChanged(a) => Message::Settings(SettingsMessage::AutonomyMaxActionsPerHourChanged(a)),
        Local::AutonomyMaxCostPerDayCentsChanged(a) => Message::Settings(SettingsMessage::AutonomyMaxCostPerDayCentsChanged(a)),
        Local::AutonomyNonCliApprovalApproversChanged(a) => Message::Settings(SettingsMessage::AutonomyNonCliApprovalApproversChanged(a)),
        Local::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(a) => Message::Settings(SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(a)),
        Local::AutonomyNonCliNaturalLanguageApprovalModeChanged(a) => Message::Settings(SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(a)),
        Local::AutonomyRequireApprovalForMediumRiskToggled(a) => Message::Settings(SettingsMessage::AutonomyRequireApprovalForMediumRiskToggled(a)),
        Local::AutonomyShellRedirectPolicyChanged(a) => Message::Settings(SettingsMessage::AutonomyShellRedirectPolicyChanged(a)),
        Local::AutonomyTabSelected(a) => Message::Settings(SettingsMessage::AutonomyTabSelected(a)),
        Local::AutonomyWorkspaceOnlyToggled(a) => Message::Settings(SettingsMessage::AutonomyWorkspaceOnlyToggled(a)),
Local::None => Message::None, Local::CopyCode(value) => Message::CopyCode(value),
}}
fn inputs(app: &App) -> View<'_> { View {autonomy_settings: &app.autonomy_settings, available_tools: &app.agents_settings.available_tools, desktop_locale: app.general_settings.desktop_locale, app_theme: &app.appearance.app_theme, last_copied_code_hash: app.clipboard_state.last_copied_code_hash, help_text: |i18n, theme, title, label, body| iced::widget::row![Element::from(crate::app::components::system_settings_common::settings_hover_help_label(i18n, theme, title, label, body)).map(|m| match m { Message::CopyCode(v) => Local::CopyCode(v), _ => Local::None })], }}
pub fn view(app: &App) -> Element<'_, Message> { desktop_settings_agents::system_settings_autonomy::view(&inputs(app)).map(map) }
pub(crate) use desktop_settings_agents::system_settings_autonomy::AUTONOMY_TABS;
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> { desktop_settings_agents::system_settings_autonomy::view_overlays(&inputs(app), dialog, map) }
#[cfg(test)]
#[path = "system_settings_autonomy_tests.rs"]
mod system_settings_autonomy_tests;

#[cfg(test)]
#[path = "system_settings_autonomy_ui_refresh_22_tests.rs"]
mod ui_refresh_22_tests;

#[cfg(test)] use crate::app::state::{AutonomyListKind, AutonomySettingsTab};
