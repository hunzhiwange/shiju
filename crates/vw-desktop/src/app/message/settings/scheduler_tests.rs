use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn scheduler_enabled_toggle_updates_value_and_clears_save_error() {
    let mut app = app();
    app.scheduler_settings.enabled = false;
    app.scheduler_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::SchedulerEnabledToggled(true));

    assert!(app.scheduler_settings.enabled);
    assert!(app.scheduler_settings.save_error.is_none());

    app.scheduler_settings.save_error = Some("new error".to_string());
    let _ = update(&mut app, SettingsMessage::SchedulerEnabledToggled(false));

    assert!(!app.scheduler_settings.enabled);
    assert!(app.scheduler_settings.save_error.is_none());
}

#[test]
fn scheduler_max_tasks_changed_clamps_bounds_and_preserves_in_range_value() {
    let mut app = app();

    for (input, expected) in [(0, 1), (1, 1), (4_321, 4_321), (10_000, 10_000), (u32::MAX, 10_000)]
    {
        app.scheduler_settings.save_error = Some("save failed".to_string());

        let _ = update(&mut app, SettingsMessage::SchedulerMaxTasksChanged(input));

        assert_eq!(app.scheduler_settings.max_tasks, expected);
        assert!(app.scheduler_settings.save_error.is_none());
    }
}

#[test]
fn scheduler_max_concurrent_changed_clamps_bounds_and_preserves_in_range_value() {
    let mut app = app();

    for (input, expected) in [(0, 1), (1, 1), (42, 42), (100, 100), (u32::MAX, 100)] {
        app.scheduler_settings.save_error = Some("save failed".to_string());

        let _ = update(&mut app, SettingsMessage::SchedulerMaxConcurrentChanged(input));

        assert_eq!(app.scheduler_settings.max_concurrent, expected);
        assert!(app.scheduler_settings.save_error.is_none());
    }
}

#[test]
fn scheduler_save_clears_error_without_changing_current_state() {
    let mut app = app();
    app.scheduler_settings.enabled = false;
    app.scheduler_settings.max_tasks = 0;
    app.scheduler_settings.max_concurrent = u32::MAX;
    app.scheduler_settings.show_help_modal = true;
    app.scheduler_settings.save_error = Some("save failed".to_string());

    let _ = update(&mut app, SettingsMessage::SchedulerSave);

    assert!(!app.scheduler_settings.enabled);
    assert_eq!(app.scheduler_settings.max_tasks, 0);
    assert_eq!(app.scheduler_settings.max_concurrent, u32::MAX);
    assert!(app.scheduler_settings.show_help_modal);
    assert!(app.scheduler_settings.save_error.is_none());
}

#[test]
fn scheduler_help_messages_only_toggle_modal_visibility() {
    let mut app = app();
    app.scheduler_settings.enabled = false;
    app.scheduler_settings.max_tasks = 321;
    app.scheduler_settings.max_concurrent = 12;
    app.scheduler_settings.save_error = Some("keep this error".to_string());

    let _ = update(&mut app, SettingsMessage::SchedulerHelpOpen);

    assert!(app.scheduler_settings.show_help_modal);
    assert!(!app.scheduler_settings.enabled);
    assert_eq!(app.scheduler_settings.max_tasks, 321);
    assert_eq!(app.scheduler_settings.max_concurrent, 12);
    assert_eq!(app.scheduler_settings.save_error.as_deref(), Some("keep this error"));

    let _ = update(&mut app, SettingsMessage::SchedulerHelpClose);

    assert!(!app.scheduler_settings.show_help_modal);
    assert!(!app.scheduler_settings.enabled);
    assert_eq!(app.scheduler_settings.max_tasks, 321);
    assert_eq!(app.scheduler_settings.max_concurrent, 12);
    assert_eq!(app.scheduler_settings.save_error.as_deref(), Some("keep this error"));
}

#[test]
fn scheduler_update_ignores_unrelated_settings_message() {
    let mut app = app();
    app.scheduler_settings.enabled = false;
    app.scheduler_settings.max_tasks = 321;
    app.scheduler_settings.max_concurrent = 12;
    app.scheduler_settings.show_help_modal = true;
    app.scheduler_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(!app.scheduler_settings.enabled);
    assert_eq!(app.scheduler_settings.max_tasks, 321);
    assert_eq!(app.scheduler_settings.max_concurrent, 12);
    assert!(app.scheduler_settings.show_help_modal);
    assert_eq!(app.scheduler_settings.save_error.as_deref(), Some("unchanged"));
}

#[test]
fn scheduler_max_tasks_change_leaves_other_scheduler_fields_unchanged() {
    let mut app = app();
    app.scheduler_settings.enabled = false;
    app.scheduler_settings.max_concurrent = 37;
    app.scheduler_settings.show_help_modal = true;

    let _ = update(&mut app, SettingsMessage::SchedulerMaxTasksChanged(512));

    assert_eq!(app.scheduler_settings.max_tasks, 512);
    assert!(!app.scheduler_settings.enabled);
    assert_eq!(app.scheduler_settings.max_concurrent, 37);
    assert!(app.scheduler_settings.show_help_modal);
}
