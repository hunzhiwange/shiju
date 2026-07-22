use super::*;
use crate::app::{App, Message, message};
use iced::Element;
use iced::widget::{slider, text};

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
fn row_helpers_build_controls() {
    keep_element(field_row("标签", "说明", text("control")));
    keep_element(bool_row("启用", "说明", true, "开启", |value| {
        Message::Settings(message::SettingsMessage::CoordinationEnabledToggled(value))
    }));
    keep_element(text_row("主协调 Agent", "说明", "delegate-lead", "lead", |value| {
        Message::Settings(message::SettingsMessage::CoordinationLeadAgentChanged(value))
    }));
    keep_element(slider_row(
        "每 Agent 收件箱",
        "说明",
        slider(1.0..=10.0, 5.0, |_| Message::None),
        5,
    ));
}

#[test]
fn view_builds_default_and_capacity_boundary_states() {
    let mut app = test_app();
    keep_element(view(&app));

    app.coordination_settings.enabled = true;
    app.coordination_settings.lead_agent_input = String::new();
    app.coordination_settings.max_inbox_messages_per_agent = 1;
    app.coordination_settings.max_dead_letters = 1;
    app.coordination_settings.max_context_entries = 1;
    app.coordination_settings.max_seen_message_ids = 1;
    keep_element(view(&app));

    app.coordination_settings.lead_agent_input = "主协调-agent".to_string();
    app.coordination_settings.max_inbox_messages_per_agent = 10_000;
    app.coordination_settings.max_dead_letters = 10_000;
    app.coordination_settings.max_context_entries = 20_000;
    app.coordination_settings.max_seen_message_ids = 100_000;
    keep_element(view(&app));
}

#[test]
fn enabled_control_maps_both_values_into_settings_messages() {
    for expected in [false, true] {
        match settings_message(enabled_toggled(expected)) {
            message::SettingsMessage::CoordinationEnabledToggled(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected coordination enabled message"),
        }
    }
}

#[test]
fn lead_agent_control_preserves_user_input_in_settings_message() {
    let expected = "  主协调-agent  ".to_string();

    match settings_message(lead_agent_changed(expected.clone())) {
        message::SettingsMessage::CoordinationLeadAgentChanged(value) => {
            assert_eq!(value, expected);
        }
        _ => panic!("expected coordination lead agent message"),
    }
}

#[test]
fn inbox_slider_rounds_values_into_settings_messages() {
    for (input, expected) in [(256.49, 256), (256.5, 257), (10_000.0, 10_000)] {
        match settings_message(max_inbox_messages_per_agent_changed(input)) {
            message::SettingsMessage::CoordinationMaxInboxMessagesPerAgentChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected coordination inbox limit message"),
        }
    }
}

#[test]
fn dead_letters_slider_rounds_values_into_settings_messages() {
    for (input, expected) in [(256.49, 256), (256.5, 257), (10_000.0, 10_000)] {
        match settings_message(max_dead_letters_changed(input)) {
            message::SettingsMessage::CoordinationMaxDeadLettersChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected coordination dead letters limit message"),
        }
    }
}

#[test]
fn context_entries_slider_rounds_values_into_settings_messages() {
    for (input, expected) in [(512.49, 512), (512.5, 513), (20_000.0, 20_000)] {
        match settings_message(max_context_entries_changed(input)) {
            message::SettingsMessage::CoordinationMaxContextEntriesChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected coordination context limit message"),
        }
    }
}

#[test]
fn seen_message_ids_slider_rounds_values_into_settings_messages() {
    for (input, expected) in [(4096.49, 4096), (4096.5, 4097), (100_000.0, 100_000)] {
        match settings_message(max_seen_message_ids_changed(input)) {
            message::SettingsMessage::CoordinationMaxSeenMessageIdsChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected coordination seen message IDs limit message"),
        }
    }
}

#[test]
fn overlays_return_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.coordination_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}
