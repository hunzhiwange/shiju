use super::*;
use crate::app::App;
use vw_config_types::security::{
    AutonomyLevel, NonCliNaturalLanguageApprovalMode, ShellRedirectPolicy,
};

fn app() -> App {
    App::new().0
}

fn apply(app: &mut App, message: SettingsMessage) {
    drop(update(app, message));
}

#[test]
fn parses_non_cli_mode_by_channel_with_aliases() {
    let parsed = parse_non_cli_mode_by_channel(
        " Telegram : direct, slack:request-confirm\nDiscord:disabled\nbad-entry\nqq:unknown",
    );

    assert_eq!(parsed.get("telegram"), Some(&NonCliNaturalLanguageApprovalMode::Direct));
    assert_eq!(parsed.get("slack"), Some(&NonCliNaturalLanguageApprovalMode::RequestConfirm));
    assert_eq!(parsed.get("discord"), Some(&NonCliNaturalLanguageApprovalMode::Disabled));
    assert!(!parsed.contains_key("qq"));
}

#[test]
fn parses_non_cli_mode_by_channel_normalizes_aliases_and_overwrites_duplicates() {
    let parsed = parse_non_cli_mode_by_channel(
        "slack:request_confirm, teams:requestconfirm, :direct, SLACK:disabled, matrix:direct:extra",
    );

    assert_eq!(parsed.get("teams"), Some(&NonCliNaturalLanguageApprovalMode::RequestConfirm));
    assert_eq!(parsed.get("slack"), Some(&NonCliNaturalLanguageApprovalMode::Disabled));
    assert_eq!(parsed.len(), 2);
}

#[test]
fn parses_non_cli_mode_by_channel_returns_empty_for_blank_and_invalid_entries() {
    assert!(parse_non_cli_mode_by_channel("  ,\n\t ").is_empty());
    assert!(parse_non_cli_mode_by_channel("missing-mode, :direct, slack:prompt").is_empty());
}

#[test]
fn update_tracks_field_changes_and_help_modal() {
    let mut app = app();
    app.autonomy_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::AutonomyLevelChanged(AutonomyLevel::Full));
    let _ = update(&mut app, SettingsMessage::AutonomyWorkspaceOnlyToggled(true));
    let _ = update(
        &mut app,
        SettingsMessage::AutonomyAllowedCommandsChanged("git status\ncargo test".to_string()),
    );
    let _ = update(
        &mut app,
        SettingsMessage::AutonomyForbiddenPathsChanged(" /tmp , /var ".to_string()),
    );
    let _ = update(&mut app, SettingsMessage::AutonomyMaxActionsPerHourChanged(0));
    let _ = update(&mut app, SettingsMessage::AutonomyMaxCostPerDayCentsChanged(2_000_000));
    let _ = update(
        &mut app,
        SettingsMessage::AutonomyShellRedirectPolicyChanged(ShellRedirectPolicy::Strip),
    );
    let _ = update(
        &mut app,
        SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeChanged(
            NonCliNaturalLanguageApprovalMode::RequestConfirm,
        ),
    );
    let _ = update(
        &mut app,
        SettingsMessage::AutonomyNonCliNaturalLanguageApprovalModeByChannelChanged(
            "slack:direct".to_string(),
        ),
    );

    assert_eq!(app.autonomy_settings.level, AutonomyLevel::Full);
    assert!(app.autonomy_settings.workspace_only);
    assert_eq!(app.autonomy_settings.allowed_commands_input, "git status\ncargo test");
    assert_eq!(app.autonomy_settings.forbidden_paths_input, " /tmp , /var ");
    assert_eq!(app.autonomy_settings.max_actions_per_hour, 1);
    assert_eq!(app.autonomy_settings.max_cost_per_day_cents, 1_000_000);
    assert_eq!(app.autonomy_settings.shell_redirect_policy, ShellRedirectPolicy::Strip);
    assert_eq!(
        app.autonomy_settings.non_cli_natural_language_approval_mode,
        NonCliNaturalLanguageApprovalMode::RequestConfirm
    );
    assert_eq!(
        app.autonomy_settings.non_cli_natural_language_approval_mode_by_channel_input,
        "slack:direct"
    );
    assert!(app.autonomy_settings.save_error.is_none());

    let _ = update(&mut app, SettingsMessage::AutonomyHelpOpen);
    assert!(app.autonomy_settings.show_help_modal);
    let _ = update(&mut app, SettingsMessage::AutonomyHelpClose);
    assert!(!app.autonomy_settings.show_help_modal);
}

#[test]
fn update_tracks_risk_and_list_field_changes() {
    let mut app = app();
    app.autonomy_settings.save_error = Some("old".to_string());

    apply(&mut app, SettingsMessage::AutonomyRequireApprovalForMediumRiskToggled(false));
    apply(&mut app, SettingsMessage::AutonomyBlockHighRiskCommandsToggled(false));
    apply(&mut app, SettingsMessage::AutonomyShellEnvPassthroughChanged("PATH\nHOME".to_string()));
    apply(
        &mut app,
        SettingsMessage::AutonomyAutoApproveChanged("file_read, memory_recall".to_string()),
    );
    apply(&mut app, SettingsMessage::AutonomyAlwaysAskChanged("shell\nfile_write".to_string()));
    apply(
        &mut app,
        SettingsMessage::AutonomyAllowedRootsChanged("/workspace, /tmp/safe".to_string()),
    );
    apply(
        &mut app,
        SettingsMessage::AutonomyNonCliExcludedToolsChanged("shell, browser".to_string()),
    );
    apply(
        &mut app,
        SettingsMessage::AutonomyNonCliApprovalApproversChanged("alice\nbob".to_string()),
    );

    assert!(!app.autonomy_settings.require_approval_for_medium_risk);
    assert!(!app.autonomy_settings.block_high_risk_commands);
    assert_eq!(app.autonomy_settings.shell_env_passthrough_input, "PATH\nHOME");
    assert_eq!(app.autonomy_settings.auto_approve_input, "file_read, memory_recall");
    assert_eq!(app.autonomy_settings.always_ask_input, "shell\nfile_write");
    assert_eq!(app.autonomy_settings.allowed_roots_input, "/workspace, /tmp/safe");
    assert_eq!(app.autonomy_settings.non_cli_excluded_tools_input, "shell, browser");
    assert_eq!(app.autonomy_settings.non_cli_approval_approvers_input, "alice\nbob");
    assert!(app.autonomy_settings.save_error.is_none());
}

#[test]
fn update_clamps_both_resource_bounds_and_save_clears_error() {
    let mut app = app();

    apply(&mut app, SettingsMessage::AutonomyMaxActionsPerHourChanged(u32::MAX));
    assert_eq!(app.autonomy_settings.max_actions_per_hour, 10_000);

    apply(&mut app, SettingsMessage::AutonomyMaxCostPerDayCentsChanged(0));
    assert_eq!(app.autonomy_settings.max_cost_per_day_cents, 1);

    app.autonomy_settings.save_error = Some("retry".to_string());
    apply(&mut app, SettingsMessage::AutonomySave);
    assert!(app.autonomy_settings.save_error.is_none());
}

#[test]
fn update_ignores_non_autonomy_messages() {
    let mut app = app();
    app.autonomy_settings.save_error = Some("unchanged".to_string());
    let before = app.autonomy_settings.clone();

    apply(&mut app, SettingsMessage::ObservabilityHelpOpen);

    assert_eq!(app.autonomy_settings.level, before.level);
    assert_eq!(app.autonomy_settings.workspace_only, before.workspace_only);
    assert_eq!(app.autonomy_settings.save_error, before.save_error);
    assert_eq!(app.autonomy_settings.show_help_modal, before.show_help_modal);
}
