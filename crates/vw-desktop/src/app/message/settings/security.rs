//! 安全设置消息映射与持久化委派。
use crate::app::{App, Message};
use crate::app::config::update_security_config_async;
use super::messages::SettingsMessage;
use desktop_settings_security::security_message::Message as LocalMessage;
use desktop_settings_security::security_update::{update as update_security, apply_security_settings_to_config};
use iced::Task;

pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let message = match message {
        SettingsMessage::SecurityBashSecurityEnabledToggled(value0) => LocalMessage::SecurityBashSecurityEnabledToggled(value0),
        SettingsMessage::SecurityBashPolicyCheckToggled(value0, value1) => LocalMessage::SecurityBashPolicyCheckToggled(value0, value1),
        SettingsMessage::SecurityBashValidatorCheckToggled(value0, value1) => LocalMessage::SecurityBashValidatorCheckToggled(value0, value1),
        SettingsMessage::SecurityTabSelected(value0) => LocalMessage::SecurityTabSelected(value0),
        SettingsMessage::SecurityListDraftChanged(value0, value1) => LocalMessage::SecurityListDraftChanged(value0, value1),
        SettingsMessage::SecurityListAdd(value0) => LocalMessage::SecurityListAdd(value0),
        SettingsMessage::SecurityListRemoveRequested(value0, value1) => LocalMessage::SecurityListRemoveRequested(value0, value1),
        SettingsMessage::SecurityListRemoveConfirmed(value0, value1) => LocalMessage::SecurityListRemoveConfirmed(value0, value1),
        SettingsMessage::SecurityListRemoveCanceled => LocalMessage::SecurityListRemoveCanceled,
        SettingsMessage::SecuritySandboxEnabledChanged(value0) => LocalMessage::SecuritySandboxEnabledChanged(value0),
        SettingsMessage::SecuritySandboxBackendChanged(value0) => LocalMessage::SecuritySandboxBackendChanged(value0),
        SettingsMessage::SecurityResourcesMaxMemoryMbChanged(value0) => LocalMessage::SecurityResourcesMaxMemoryMbChanged(value0),
        SettingsMessage::SecurityResourcesMaxCpuTimeSecondsChanged(value0) => LocalMessage::SecurityResourcesMaxCpuTimeSecondsChanged(value0),
        SettingsMessage::SecurityResourcesMaxSubprocessesChanged(value0) => LocalMessage::SecurityResourcesMaxSubprocessesChanged(value0),
        SettingsMessage::SecurityResourcesMemoryMonitoringToggled(value0) => LocalMessage::SecurityResourcesMemoryMonitoringToggled(value0),
        SettingsMessage::SecurityAuditEnabledToggled(value0) => LocalMessage::SecurityAuditEnabledToggled(value0),
        SettingsMessage::SecurityAuditLogPathChanged(value0) => LocalMessage::SecurityAuditLogPathChanged(value0),
        SettingsMessage::SecurityAuditMaxSizeMbChanged(value0) => LocalMessage::SecurityAuditMaxSizeMbChanged(value0),
        SettingsMessage::SecurityAuditSignEventsToggled(value0) => LocalMessage::SecurityAuditSignEventsToggled(value0),
        SettingsMessage::SecurityOtpEnabledToggled(value0) => LocalMessage::SecurityOtpEnabledToggled(value0),
        SettingsMessage::SecurityOtpMethodChanged(value0) => LocalMessage::SecurityOtpMethodChanged(value0),
        SettingsMessage::SecurityOtpTokenTtlSecsChanged(value0) => LocalMessage::SecurityOtpTokenTtlSecsChanged(value0),
        SettingsMessage::SecurityOtpCacheValidSecsChanged(value0) => LocalMessage::SecurityOtpCacheValidSecsChanged(value0),
        SettingsMessage::SecurityEstopEnabledToggled(value0) => LocalMessage::SecurityEstopEnabledToggled(value0),
        SettingsMessage::SecurityEstopStateFileChanged(value0) => LocalMessage::SecurityEstopStateFileChanged(value0),
        SettingsMessage::SecurityEstopRequireOtpToResumeToggled(value0) => LocalMessage::SecurityEstopRequireOtpToResumeToggled(value0),
        SettingsMessage::SecuritySyscallAnomalyEnabledToggled(value0) => LocalMessage::SecuritySyscallAnomalyEnabledToggled(value0),
        SettingsMessage::SecuritySyscallAnomalyStrictModeToggled(value0) => LocalMessage::SecuritySyscallAnomalyStrictModeToggled(value0),
        SettingsMessage::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(value0) => LocalMessage::SecuritySyscallAnomalyAlertOnUnknownSyscallToggled(value0),
        SettingsMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(value0) => LocalMessage::SecuritySyscallAnomalyMaxDeniedEventsPerMinuteChanged(value0),
        SettingsMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(value0) => LocalMessage::SecuritySyscallAnomalyMaxTotalEventsPerMinuteChanged(value0),
        SettingsMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(value0) => LocalMessage::SecuritySyscallAnomalyMaxAlertsPerMinuteChanged(value0),
        SettingsMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(value0) => LocalMessage::SecuritySyscallAnomalyAlertCooldownSecsChanged(value0),
        SettingsMessage::SecuritySyscallAnomalyLogPathChanged(value0) => LocalMessage::SecuritySyscallAnomalyLogPathChanged(value0),
        SettingsMessage::SecurityCanaryTokensToggled(value0) => LocalMessage::SecurityCanaryTokensToggled(value0),
        SettingsMessage::SecuritySemanticGuardToggled(value0) => LocalMessage::SecuritySemanticGuardToggled(value0),
        SettingsMessage::SecuritySemanticGuardCollectionChanged(value0) => LocalMessage::SecuritySemanticGuardCollectionChanged(value0),
        SettingsMessage::SecuritySemanticGuardThresholdChanged(value0) => LocalMessage::SecuritySemanticGuardThresholdChanged(value0),
        SettingsMessage::SecuritySave => LocalMessage::SecuritySave,
        SettingsMessage::SecurityHelpOpen => LocalMessage::SecurityHelpOpen,
        SettingsMessage::SecurityHelpClose => LocalMessage::SecurityHelpClose,
        _ => return Task::none(),
    };
    if !update_security(&mut app.security_settings, message) {
        return Task::none();
    }
    let state = app.security_settings.clone();
    update_security_config_async(move |security| apply_security_settings_to_config(security, &state))
}
#[cfg(test)]
#[path = "security_tests.rs"]
mod security_tests;
