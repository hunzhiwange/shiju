//! 自主性领域更新的桌面持久化适配。
use crate::app::{App, Message};
use super::SettingsMessage;
use iced::Task;
use desktop_settings_agents::{autonomy_message::Message as Local, autonomy_update};
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
 let message = match message {
SettingsMessage::AutonomyBlockHighRiskCommandsToggled(v0) => Local::AutonomyBlockHighRiskCommandsToggled(v0),
SettingsMessage::AutonomyHelpClose => Local::AutonomyHelpClose,
SettingsMessage::AutonomyHelpOpen => Local::AutonomyHelpOpen,
SettingsMessage::AutonomyLevelChanged(v0) => Local::AutonomyLevelChanged(v0),
SettingsMessage::AutonomyListAdd(v0) => Local::AutonomyListAdd(v0),
SettingsMessage::AutonomyListDraftChanged(v0, v1) => Local::AutonomyListDraftChanged(v0, v1),
SettingsMessage::AutonomyListRemoveCanceled => Local::AutonomyListRemoveCanceled,
SettingsMessage::AutonomyListRemoveConfirmed(v0, v1) => Local::AutonomyListRemoveConfirmed(v0, v1),
SettingsMessage::AutonomyListRemoveRequested(v0, v1) => Local::AutonomyListRemoveRequested(v0, v1),
SettingsMessage::AutonomyMaxActionsPerHourChanged(v0) => Local::AutonomyMaxActionsPerHourChanged(v0),
SettingsMessage::AutonomyMaxCostPerDayCentsChanged(v0) => Local::AutonomyMaxCostPerDayCentsChanged(v0),
SettingsMessage::AutonomyNonCliApprovalApproversChanged(v0) => Local::AutonomyNonCliApprovalApproversChanged(v0),
SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(v0) => Local::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(v0),
SettingsMessage::AutonomyRequireApprovalForMediumRiskToggled(v0) => Local::AutonomyRequireApprovalForMediumRiskToggled(v0),
SettingsMessage::AutonomyShellRedirectPolicyChanged(v0) => Local::AutonomyShellRedirectPolicyChanged(v0),
SettingsMessage::AutonomyTabSelected(v0) => Local::AutonomyTabSelected(v0),
SettingsMessage::AutonomyWorkspaceOnlyToggled(v0) => Local::AutonomyWorkspaceOnlyToggled(v0),
SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(v) => Local::AutonomyNonCliNaturalLanguageApprovalModeChanged(v),
 SettingsMessage::AutonomySave => Local::AutonomySave,
 _ => return Task::none(),
 };
 if autonomy_update::update(&mut app.autonomy_settings, message) { persist(app) } else { Task::none() }
}
fn persist(app: &mut App) -> Task<Message> {
 let settings = app.autonomy_settings.clone();
 crate::app::update_autonomy_config_async(move |config| autonomy_update::apply_autonomy_settings_to_config(&settings, config))
}
