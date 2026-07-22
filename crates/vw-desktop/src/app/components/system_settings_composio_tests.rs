use super::*;
use crate::app::message::settings::SettingsMessage;
use crate::app::{App, Message};
use iced::Element;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn composio_message_maps_to_settings_message_variants() {
    match SettingsMessage::from(ComposioMessage::EnabledToggled(true)) {
        SettingsMessage::ComposioEnabledToggled(true) => {}
        other => panic!("unexpected message: {other:?}"),
    }

    match SettingsMessage::from(ComposioMessage::ApiKeyChanged("cmp_key".to_string())) {
        SettingsMessage::ComposioApiKeyChanged(value) => assert_eq!(value, "cmp_key"),
        other => panic!("unexpected message: {other:?}"),
    }

    match SettingsMessage::from(ComposioMessage::EntityIdChanged("entity".to_string())) {
        SettingsMessage::ComposioEntityIdChanged(value) => assert_eq!(value, "entity"),
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn field_and_text_rows_build_expected_wrappers() {
    keep_element(field_row("Label", "Description", iced::widget::text("control")));
    keep_element(text_row("API", "Description", "cmp_...", "secret", "hint", true, |value| {
        Message::Settings(ComposioMessage::ApiKeyChanged(value).into())
    }));
    keep_element(text_row("Entity", "Description", "default", "entity", "hint", false, |value| {
        Message::Settings(ComposioMessage::EntityIdChanged(value).into())
    }));
}

#[test]
fn view_builds_default_enabled_and_error_states() {
    let app = test_app();
    keep_element(view(&app));

    let mut app = test_app();
    app.composio_settings.enabled = true;
    app.composio_settings.api_key_input = "cmp_123".to_string();
    app.composio_settings.entity_id_input = "team-a".to_string();
    keep_element(view(&app));

    app.composio_settings.save_error = Some("保存失败".to_string());
    keep_element(view(&app));
}
