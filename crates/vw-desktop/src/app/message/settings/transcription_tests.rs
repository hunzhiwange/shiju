use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

#[test]
fn transcription_enabled_update_changes_value_and_clears_save_error() {
    let mut app = app();
    app.transcription_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::TranscriptionEnabledToggled(true));

    assert!(app.transcription_settings.enabled);
    assert!(app.transcription_settings.save_error.is_none());
}

#[test]
fn transcription_edits_schedule_persistence_without_running_it() {
    let mut app = app();

    let task = update(&mut app, SettingsMessage::TranscriptionLanguageChanged("  ".to_string()));

    assert_eq!(app.transcription_settings.language, "  ");
    assert!(has_scheduled_action(task));
}

#[test]
fn transcription_text_updates_preserve_user_input_and_clear_save_error() {
    let mut app = app();

    app.transcription_settings.save_error = Some("api error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::TranscriptionApiUrlChanged(
            " https://example.test/transcribe ".to_string(),
        ),
    );
    assert_eq!(app.transcription_settings.api_url, " https://example.test/transcribe ");
    assert!(app.transcription_settings.save_error.is_none());

    app.transcription_settings.save_error = Some("model error".to_string());
    let _ = update(
        &mut app,
        SettingsMessage::TranscriptionModelChanged(" custom-whisper ".to_string()),
    );
    assert_eq!(app.transcription_settings.model, " custom-whisper ");
    assert!(app.transcription_settings.save_error.is_none());

    app.transcription_settings.save_error = Some("language error".to_string());
    let _ = update(&mut app, SettingsMessage::TranscriptionLanguageChanged(" zh-CN ".to_string()));
    assert_eq!(app.transcription_settings.language, " zh-CN ");
    assert!(app.transcription_settings.save_error.is_none());
}

#[test]
fn transcription_max_duration_update_clamps_both_boundaries() {
    let mut app = app();
    app.transcription_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::TranscriptionMaxDurationSecsChanged(0));
    assert_eq!(app.transcription_settings.max_duration_secs, 1);
    assert!(app.transcription_settings.save_error.is_none());

    let _ = update(&mut app, SettingsMessage::TranscriptionMaxDurationSecsChanged(3601));
    assert_eq!(app.transcription_settings.max_duration_secs, 3600);

    let _ = update(&mut app, SettingsMessage::TranscriptionMaxDurationSecsChanged(120));
    assert_eq!(app.transcription_settings.max_duration_secs, 120);
}

#[test]
fn transcription_help_messages_toggle_modal() {
    let mut app = app();

    let _ = update(&mut app, SettingsMessage::TranscriptionHelpOpen);
    assert!(app.transcription_settings.show_help_modal);

    let _ = update(&mut app, SettingsMessage::TranscriptionHelpClose);
    assert!(!app.transcription_settings.show_help_modal);
}

#[test]
fn transcription_update_ignores_unrelated_message() {
    let mut app = app();
    app.transcription_settings.enabled = true;
    app.transcription_settings.api_url = "https://example.test/original".to_string();
    app.transcription_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::HeartbeatHelpOpen);

    assert!(app.transcription_settings.enabled);
    assert_eq!(app.transcription_settings.api_url, "https://example.test/original");
    assert_eq!(app.transcription_settings.save_error.as_deref(), Some("unchanged"));
}
