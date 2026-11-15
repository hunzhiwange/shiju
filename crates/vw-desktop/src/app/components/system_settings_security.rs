//! Security settings assembly; implementation belongs to desktop-settings-security.
use crate::app::{App, Message};
use crate::app::message::SettingsMessage;
use desktop_settings_security::{SecurityView, security_message::Message as Local};
use iced::Element;
pub(crate) use desktop_settings_security::system_settings_security::{SECURITY_TABS, security_tab_label};
fn map(message: Local) -> Message {
    match message {
        Local::SecurityBashSecurityEnabledToggled(a) => Message::Settings(SettingsMessage::SecurityBashSecurityEnabledToggled(a)),
        Local::SecurityBashPolicyCheckToggled(a, b) => Message::Settings(SettingsMessage::SecurityBashPolicyCheckToggled(a, b)),
        Local::SecurityBashValidatorCheckToggled(a, b) => Message::Settings(SettingsMessage::SecurityBashValidatorCheckToggled(a, b)),
        Local::SecurityTabSelected(a) => Message::Settings(SettingsMessage::SecurityTabSelected(a)),
        Local::SecurityListDraftChanged(a, b) => Message::Settings(SettingsMessage::SecurityListDraftChanged(a, b)),
        Local::SecurityListAdd(a) => Message::Settings(SettingsMessage::SecurityListAdd(a)),
        Local::SecurityListRemoveRequested(a, b) => Message::Settings(SettingsMessage::SecurityListRemoveRequested(a, b)),
        Local::SecurityListRemoveConfirmed(a, b) => Message::Settings(SettingsMessage::SecurityListRemoveConfirmed(a, b)),
        Local::SecurityListRemoveCanceled => Message::Settings(SettingsMessage::SecurityListRemoveCanceled),
        Local::SecuritySandboxEnabledChanged(a) => Message::Settings(SettingsMessage::SecuritySandboxEnabledChanged(a)),
        Local::SecuritySandboxBackendChanged(a) => Message::Settings(SettingsMessage::SecuritySandboxBackendChanged(a)),
        Local::SecurityResourcesMaxMemoryMbChanged(a) => Message::Settings(SettingsMessage::SecurityResourcesMaxMemoryMbChanged(a)),
        Local::SecurityResourcesMaxCpuTimeSecondsChanged(a) => Message::Settings(SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(a)),
        Local::SecurityResourcesMaxSubprocessesChanged(a) => Message::Settings(SettingsMessage::SecurityResourcesMaxSubprocessesChanged(a)),
        Local::SecurityResourcesMemoryMonitoringToggled(a) => Message::Settings(SettingsMessage::SecurityResourcesMemoryMonitoringToggled(a)),
        Local::SecurityAuditEnabledToggled(a) => Message::Settings(SettingsMessage::SecurityAuditEnabledToggled(a)),
        Local::SecurityAuditLogPathChanged(a) => Message::Settings(SettingsMessage::SecurityAuditLogPathChanged(a)),
        Local::SecurityAuditMaxSizeMbChanged(a) => Message::Settings(SettingsMessage::SecurityAuditMaxSizeMbChanged(a)),
        Local::SecurityAuditSignEventsToggled(a) => Message::Settings(SettingsMessage::SecurityAuditSignEventsToggled(a)),
        Local::SecurityOtpEnabledToggled(a) => Message::Settings(SettingsMessage::SecurityOtpEnabledToggled(a)),
        Local::SecurityOtpMethodChanged(a) => Message::Settings(SettingsMessage::SecurityOtpMethodChanged(a)),
        Local::SecurityOtpTokenTtlSecsChanged(a) => Message::Settings(SettingsMessage::SecurityOtpTokenTtlSecsChanged(a)),
        Local::SecurityOtpCacheValidSecsChanged(a) => Message::Settings(SettingsMessage::SecurityOtpCacheValidSecsChanged(a)),
        Local::SecurityEstopEnabledToggled(a) => Message::Settings(SettingsMessage::SecurityEstopEnabledToggled(a)),
        Local::SecurityEstopStateFileChanged(a) => Message::Settings(SettingsMessage::SecurityEstopStateFileChanged(a)),
        Local::SecurityEstopRequireOtpToResumeToggled(a) => Message::Settings(SettingsMessage::SecurityEstopRequireOtpToResumeToggled(a)),
        Local::SecuritySyscallAnomalyEnabledToggled(a) => Message::Settings(SettingsMessage::SecuritySyscallAnomalyEnabledToggled(a)),
        Local::SecuritySyscallAnomalyStrictModeToggled(a) => Message::Settings(SettingsMessage::SecuritySyscallAnomalyStrictModeToggled(a)),
        Local::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(a) => Message::Settings(SettingsMessage::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(a)),
        Local::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(a) => Message::Settings(SettingsMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(a)),
        Local::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(a) => Message::Settings(SettingsMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(a)),
        Local::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(a) => Message::Settings(SettingsMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(a)),
        Local::SecuritySyscallAnomalyAlertCooldownSecsChanged(a) => Message::Settings(SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(a)),
        Local::SecuritySyscallAnomalyLogPathChanged(a) => Message::Settings(SettingsMessage::SecuritySyscallAnomalyLogPathChanged(a)),
        Local::SecurityCanaryTokensToggled(a) => Message::Settings(SettingsMessage::SecurityCanaryTokensToggled(a)),
        Local::SecuritySemanticGuardToggled(a) => Message::Settings(SettingsMessage::SecuritySemanticGuardToggled(a)),
        Local::SecuritySemanticGuardCollectionChanged(a) => Message::Settings(SettingsMessage::SecuritySemanticGuardCollectionChanged(a)),
        Local::SecuritySemanticGuardThresholdChanged(a) => Message::Settings(SettingsMessage::SecuritySemanticGuardThresholdChanged(a)),
        Local::SecuritySave => Message::Settings(SettingsMessage::SecuritySave),
        Local::SecurityHelpOpen => Message::Settings(SettingsMessage::SecurityHelpOpen),
        Local::SecurityHelpClose => Message::Settings(SettingsMessage::SecurityHelpClose),
        Local::CopyCode(value) => Message::CopyCode(value),
        Local::None => Message::None,
    }
}
fn inputs(app: &App) -> SecurityView<'_> {
    SecurityView { security_settings: &app.security_settings, desktop_locale: app.desktop_locale,
        app_theme: &app.app_theme, last_copied_code_hash: app.last_copied_code_hash,
        help_text: |title, body, color, i18n| crate::app::components::chat_panel::message_view::selectable_titled_text(title, body, color, i18n).map(|message| match message { Message::CopyCode(value) => Local::CopyCode(value), _ => Local::None }),
    }
}
pub fn view(app: &App) -> Element<'_, Message> { desktop_settings_security::system_settings_security::view(&inputs(app)).map(map) }
pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    desktop_settings_security::system_settings_security::host_overlays(&inputs(app), dialog, map)
}
#[cfg(test)]
#[path = "system_settings_security_tests.rs"]
mod tests;
