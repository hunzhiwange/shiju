use config_security::{AutonomyLevel, ShellRedirectPolicy};
#[derive(Debug, Clone)]
pub enum Message {
    AutonomyBlockHighRiskCommandsToggled(bool),
    AutonomyHelpClose,
    AutonomyHelpOpen,
    AutonomyLevelChanged(AutonomyLevel),
    AutonomyListAdd(desktop_settings_security::app_init::state::AutonomyListKind),
    AutonomyListDraftChanged(desktop_settings_security::app_init::state::AutonomyListKind, String),
    AutonomyListRemoveCanceled,
    AutonomyListRemoveConfirmed(desktop_settings_security::app_init::state::AutonomyListKind, usize),
    AutonomyListRemoveRequested(desktop_settings_security::app_init::state::AutonomyListKind, usize),
    AutonomyMaxActionsPerHourChanged(u32),
    AutonomyMaxCostPerDayCentsChanged(u32),
    AutonomyNonCliApprovalApproversChanged(String),
    AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(String),
    AutonomyNonCliNaturalLanguageApprovalModeChanged(
        config_security::NonCliNaturalLanguageApprovalMode,
    ),
    AutonomyRequireApprovalForMediumRiskToggled(bool),
    AutonomyShellRedirectPolicyChanged(ShellRedirectPolicy),
    AutonomyTabSelected(desktop_settings_security::app_init::state::AutonomySettingsTab),
    AutonomyWorkspaceOnlyToggled(bool),
    AutonomySave,
    CopyCode(String), OpenUrlExternal(String), AppendText(String), None,
}
