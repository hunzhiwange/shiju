use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn normalize_execution_mode_accepts_autonomous_case_insensitively() {
    assert_eq!(normalize_execution_mode("  AuToNoMoUs  "), "autonomous");
}

#[test]
fn normalize_execution_mode_falls_back_to_supervised() {
    assert_eq!(normalize_execution_mode(" supervised "), "supervised");
    assert_eq!(normalize_execution_mode("unsupported"), "supervised");
    assert_eq!(normalize_execution_mode("   "), "supervised");
}

#[test]
fn trim_optional_trims_values_and_rejects_blank_input() {
    assert_eq!(trim_optional("  workspace/sops  "), Some("workspace/sops".to_string()));
    assert_eq!(trim_optional(" \n\t "), None);
}

#[test]
fn update_ignores_non_sop_messages() {
    let mut app = app();
    app.sop_settings.sops_dir_input = "unchanged".to_string();
    app.sop_settings.default_execution_mode = "autonomous".to_string();
    app.sop_settings.max_finished_runs = 12;
    app.sop_settings.max_concurrent_total = 3;
    app.sop_settings.approval_timeout_secs = 90;
    app.sop_settings.save_error = Some("old error".to_string());

    let _task = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.sop_settings.sops_dir_input, "unchanged");
    assert_eq!(app.sop_settings.default_execution_mode, "autonomous");
    assert_eq!(app.sop_settings.max_finished_runs, 12);
    assert_eq!(app.sop_settings.max_concurrent_total, 3);
    assert_eq!(app.sop_settings.approval_timeout_secs, 90);
    assert_eq!(app.sop_settings.save_error.as_deref(), Some("old error"));
}

#[test]
fn sops_dir_changed_keeps_editor_input_and_clears_save_error() {
    let mut app = app();
    app.sop_settings.save_error = Some("old error".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Sop(SopMessage::SopsDirChanged("  workspace/sops  ".to_string())),
    );

    assert_eq!(app.sop_settings.sops_dir_input, "  workspace/sops  ");
    assert!(app.sop_settings.save_error.is_none());
}

#[test]
fn sops_dir_pick_request_does_not_mutate_settings_before_completion() {
    let mut app = app();
    app.sop_settings.sops_dir_input = "existing".to_string();
    app.sop_settings.save_error = Some("old error".to_string());

    let _task = update(&mut app, SettingsMessage::Sop(SopMessage::SopsDirPickFolder));

    assert_eq!(app.sop_settings.sops_dir_input, "existing");
    assert_eq!(app.sop_settings.save_error.as_deref(), Some("old error"));
}

#[test]
fn cancelled_sops_dir_pick_preserves_settings() {
    let mut app = app();
    app.sop_settings.sops_dir_input = "existing".to_string();
    app.sop_settings.save_error = Some("old error".to_string());

    let _task = update(&mut app, SettingsMessage::Sop(SopMessage::SopsDirPicked(None)));

    assert_eq!(app.sop_settings.sops_dir_input, "existing");
    assert_eq!(app.sop_settings.save_error.as_deref(), Some("old error"));
}

#[test]
fn completed_sops_dir_pick_updates_path_and_clears_save_error() {
    let mut app = app();
    app.sop_settings.save_error = Some("old error".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Sop(SopMessage::SopsDirPicked(Some("picked/sops".to_string()))),
    );

    assert_eq!(app.sop_settings.sops_dir_input, "picked/sops");
    assert!(app.sop_settings.save_error.is_none());
}

#[test]
fn execution_mode_change_normalizes_supported_and_unknown_values() {
    let mut app = app();
    app.sop_settings.save_error = Some("old error".to_string());

    let _task = update(
        &mut app,
        SettingsMessage::Sop(SopMessage::DefaultExecutionModeChanged("  AUTONOMOUS  ".to_string())),
    );
    assert_eq!(app.sop_settings.default_execution_mode, "autonomous");
    assert!(app.sop_settings.save_error.is_none());

    let _task = update(
        &mut app,
        SettingsMessage::Sop(SopMessage::DefaultExecutionModeChanged("unknown".to_string())),
    );
    assert_eq!(app.sop_settings.default_execution_mode, "supervised");
}

#[test]
fn max_finished_runs_change_caps_only_the_upper_bound() {
    let mut app = app();

    let _task =
        update(&mut app, SettingsMessage::Sop(SopMessage::MaxFinishedRunsChanged(u32::MAX)));
    assert_eq!(app.sop_settings.max_finished_runs, 100_000);

    let _task = update(&mut app, SettingsMessage::Sop(SopMessage::MaxFinishedRunsChanged(0)));
    assert_eq!(app.sop_settings.max_finished_runs, 0);
}

#[test]
fn max_concurrent_total_change_clamps_both_bounds() {
    let mut app = app();
    app.sop_settings.save_error = Some("old error".to_string());

    let _task = update(&mut app, SettingsMessage::Sop(SopMessage::MaxConcurrentTotalChanged(0)));
    assert_eq!(app.sop_settings.max_concurrent_total, 1);
    assert!(app.sop_settings.save_error.is_none());

    let _task =
        update(&mut app, SettingsMessage::Sop(SopMessage::MaxConcurrentTotalChanged(u32::MAX)));
    assert_eq!(app.sop_settings.max_concurrent_total, 1_000);
}

#[test]
fn approval_timeout_change_caps_upper_bound_and_allows_disabled_value() {
    let mut app = app();

    let _task =
        update(&mut app, SettingsMessage::Sop(SopMessage::ApprovalTimeoutSecsChanged(u32::MAX)));
    assert_eq!(app.sop_settings.approval_timeout_secs, 86_400);

    let _task = update(&mut app, SettingsMessage::Sop(SopMessage::ApprovalTimeoutSecsChanged(0)));
    assert_eq!(app.sop_settings.approval_timeout_secs, 0);
}

#[test]
fn numeric_sop_changes_preserve_values_inside_allowed_ranges() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::Sop(SopMessage::MaxFinishedRunsChanged(2_500)));
    let _ = update(&mut app, SettingsMessage::Sop(SopMessage::MaxConcurrentTotalChanged(24)));
    let _ = update(&mut app, SettingsMessage::Sop(SopMessage::ApprovalTimeoutSecsChanged(600)));

    assert_eq!(app.sop_settings.max_finished_runs, 2_500);
    assert_eq!(app.sop_settings.max_concurrent_total, 24);
    assert_eq!(app.sop_settings.approval_timeout_secs, 600);
}
