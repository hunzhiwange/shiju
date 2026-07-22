use super::*;
use crate::app::{App, message};
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
fn field_row_accepts_scheduler_control() {
    keep_element(field_row("最大任务", "单次轮询最多处理的任务数量。", text("64")));
}

#[test]
fn view_builds_default_enabled_and_slider_boundary_states() {
    let mut app = test_app();
    keep_element(view(&app));

    app.scheduler_settings.enabled = true;
    app.scheduler_settings.max_tasks = 1;
    app.scheduler_settings.max_concurrent = 1;
    keep_element(view(&app));

    app.scheduler_settings.max_tasks = 10_000;
    app.scheduler_settings.max_concurrent = 100;
    keep_element(view(&app));
}

#[test]
fn enabled_control_maps_both_values_into_settings_messages() {
    for expected in [false, true] {
        match settings_message(enabled_toggled(expected)) {
            message::SettingsMessage::SchedulerEnabledToggled(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected scheduler enabled message"),
        }
    }
}

#[test]
fn max_tasks_slider_rounds_values_into_settings_messages() {
    for (input, expected) in [(64.49, 64), (64.5, 65), (10_000.0, 10_000)] {
        match settings_message(max_tasks_changed(input)) {
            message::SettingsMessage::SchedulerMaxTasksChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected scheduler max tasks message"),
        }
    }
}

#[test]
fn max_concurrent_slider_rounds_values_into_settings_messages() {
    for (input, expected) in [(4.49, 4), (4.5, 5), (100.0, 100)] {
        match settings_message(max_concurrent_changed(input)) {
            message::SettingsMessage::SchedulerMaxConcurrentChanged(value) => {
                assert_eq!(value, expected);
            }
            _ => panic!("expected scheduler max concurrent message"),
        }
    }
}

#[test]
fn overlays_return_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.scheduler_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}
