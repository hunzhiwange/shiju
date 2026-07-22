use super::*;
use crate::app::{App, Message, message};
use iced::Element;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn settings_message(message: Message) -> message::SettingsMessage {
    match message {
        Message::Settings(settings) => settings,
        _ => panic!("expected settings message"),
    }
}

#[test]
fn field_and_text_rows_build_controls() {
    keep_element(field_row("标签", "说明", text("control")));
    keep_element(text_row("模型", "说明", "placeholder", "whisper", |value| {
        Message::Settings(message::SettingsMessage::TranscriptionModelChanged(value))
    }));
}

#[test]
fn controls_map_values_into_transcription_messages() {
    for expected in [false, true] {
        match settings_message(enabled_toggled(expected)) {
            message::SettingsMessage::TranscriptionEnabledToggled(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected transcription enabled message"),
        }
    }

    for (message, expected) in [
        (api_url_changed(String::new()), ""),
        (
            api_url_changed("https://transcribe.example/v1".to_string()),
            "https://transcribe.example/v1",
        ),
    ] {
        match settings_message(message) {
            message::SettingsMessage::TranscriptionApiUrlChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected transcription API URL message"),
        }
    }

    match settings_message(model_changed("whisper-large-v3-turbo".to_string())) {
        message::SettingsMessage::TranscriptionModelChanged(value) => {
            assert_eq!(value, "whisper-large-v3-turbo");
        }
        _ => panic!("expected transcription model message"),
    }

    for expected in ["", "zh"] {
        match settings_message(language_changed(expected.to_string())) {
            message::SettingsMessage::TranscriptionLanguageChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected transcription language message"),
        }
    }
}

#[test]
fn max_duration_slider_rounds_boundary_values_into_settings_messages() {
    for (input, expected) in [(1.0, 1), (119.49, 119), (119.5, 120), (3600.0, 3600)] {
        match settings_message(max_duration_secs_changed(input)) {
            message::SettingsMessage::TranscriptionMaxDurationSecsChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected transcription max duration message"),
        }
    }
}

#[test]
fn view_builds_disabled_enabled_and_duration_boundary_states() {
    let mut app = test_app();
    keep_element(view(&app));

    app.transcription_settings.enabled = true;
    app.transcription_settings.api_url = "https://transcribe.example/v1".to_string();
    app.transcription_settings.model = "whisper-large-v3".to_string();
    app.transcription_settings.language = "zh".to_string();
    app.transcription_settings.max_duration_secs = 3600;
    keep_element(view(&app));

    app.transcription_settings.max_duration_secs = 1;
    keep_element(view(&app));
}

#[test]
fn overlays_return_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.transcription_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}
