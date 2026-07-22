use super::*;
use crate::app::App;
use vw_config_types::automation::ResearchTrigger;

fn app() -> App {
    App::new().0
}

fn set_stale_error(app: &mut App) {
    app.research_settings.save_error = Some("old error".to_string());
}

#[test]
fn update_applies_research_fields_and_clears_stale_errors() {
    let mut app = app();

    set_stale_error(&mut app);
    let _ = update(&mut app, SettingsMessage::ResearchEnabledToggled(true));
    assert!(app.research_settings.enabled);
    assert!(app.research_settings.save_error.is_none());

    set_stale_error(&mut app);
    let _ = update(&mut app, SettingsMessage::ResearchTriggerChanged(ResearchTrigger::Question));
    assert_eq!(app.research_settings.trigger, ResearchTrigger::Question);
    assert!(app.research_settings.save_error.is_none());

    set_stale_error(&mut app);
    let _ = update(
        &mut app,
        SettingsMessage::ResearchKeywordsChanged(" rust, ai\nreview ".to_string()),
    );
    assert_eq!(app.research_settings.keywords_input, " rust, ai\nreview ");
    assert!(app.research_settings.save_error.is_none());

    set_stale_error(&mut app);
    let _ = update(&mut app, SettingsMessage::ResearchShowProgressToggled(false));
    assert!(!app.research_settings.show_progress);
    assert!(app.research_settings.save_error.is_none());

    set_stale_error(&mut app);
    let _ = update(
        &mut app,
        SettingsMessage::ResearchSystemPromptPrefixChanged(" prefix with spaces ".to_string()),
    );
    assert_eq!(app.research_settings.system_prompt_prefix, " prefix with spaces ");
    assert!(app.research_settings.save_error.is_none());
}

#[test]
fn min_message_length_is_clamped_at_both_bounds_and_accepts_in_range_values() {
    let mut app = app();

    let cases = [(0, 1), (1, 1), (4_096, 4_096), (10_000, 10_000), (u32::MAX, 10_000)];

    for (input, expected) in cases {
        set_stale_error(&mut app);
        let _ = update(&mut app, SettingsMessage::ResearchMinMessageLengthChanged(input));

        assert_eq!(app.research_settings.min_message_length, expected);
        assert!(app.research_settings.save_error.is_none());
    }
}

#[test]
fn max_iterations_is_clamped_at_both_bounds_and_accepts_in_range_values() {
    let mut app = app();

    let cases = [(0, 1), (1, 1), (42, 42), (100, 100), (u32::MAX, 100)];

    for (input, expected) in cases {
        set_stale_error(&mut app);
        let _ = update(&mut app, SettingsMessage::ResearchMaxIterationsChanged(input));

        assert_eq!(app.research_settings.max_iterations, expected);
        assert!(app.research_settings.save_error.is_none());
    }
}

#[test]
fn research_save_clears_stale_error_without_changing_panel_state() {
    let mut app = app();
    app.research_settings.enabled = true;
    app.research_settings.show_help_modal = true;
    set_stale_error(&mut app);

    let _ = update(&mut app, SettingsMessage::ResearchSave);

    assert!(app.research_settings.enabled);
    assert!(app.research_settings.show_help_modal);
    assert!(app.research_settings.save_error.is_none());
}

#[test]
fn research_help_messages_toggle_modal_without_clearing_save_error() {
    let mut app = app();
    set_stale_error(&mut app);

    let _ = update(&mut app, SettingsMessage::ResearchHelpOpen);
    assert!(app.research_settings.show_help_modal);
    assert_eq!(app.research_settings.save_error.as_deref(), Some("old error"));

    let _ = update(&mut app, SettingsMessage::ResearchHelpClose);
    assert!(!app.research_settings.show_help_modal);
    assert_eq!(app.research_settings.save_error.as_deref(), Some("old error"));
}

#[test]
fn unrelated_message_leaves_research_state_unchanged() {
    let mut app = app();
    app.research_settings.enabled = true;
    app.research_settings.keywords_input = "keep me".to_string();
    app.research_settings.show_help_modal = true;
    set_stale_error(&mut app);

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.research_settings.enabled);
    assert_eq!(app.research_settings.keywords_input, "keep me");
    assert!(app.research_settings.show_help_modal);
    assert_eq!(app.research_settings.save_error.as_deref(), Some("old error"));
}

#[test]
fn research_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        SettingsMessage::ResearchEnabledToggled(true),
        SettingsMessage::ResearchKeywordsChanged("rust, ai".to_string()),
        SettingsMessage::ResearchSave,
    ] {
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
