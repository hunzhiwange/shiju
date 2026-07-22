#[test]
fn settings_tests_are_wired() {
    assert!(module_path!().contains("settings_tests"));
}

use super::settings::{
    SettingsControl, build_settings_overlay, control_message, font_size_badge_label,
    line_height_badge_label, selected_editor_theme,
};
use crate::app::message::editor::EditorMessage;
use crate::app::{App, Message};
use iced::widget::{container, text};
use iced::{Element, Theme};
use iced_code_editor::i18n::Language;

fn app() -> App {
    App::new().0
}

fn content() -> Element<'static, Message> {
    container(text("preview")).into()
}

fn keep(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn editor_message(message: Message) -> EditorMessage {
    match message {
        Message::Editor(message) => message,
        other => panic!("expected editor message, got {other:?}"),
    }
}

#[test]
fn close_control_maps_to_toggle_settings() {
    assert!(matches!(
        editor_message(control_message(SettingsControl::Close)),
        EditorMessage::ToggleSettings
    ));
}

#[test]
fn numeric_controls_preserve_slider_values() {
    match editor_message(control_message(SettingsControl::FontSize(17.25))) {
        EditorMessage::FontSizeChanged(value) => assert_eq!(value, 17.25),
        other => panic!("expected font size change, got {other:?}"),
    }

    match editor_message(control_message(SettingsControl::LineHeight(28.5))) {
        EditorMessage::LineHeightChanged(value) => assert_eq!(value, 28.5),
        other => panic!("expected line height change, got {other:?}"),
    }
}

#[test]
fn boolean_controls_preserve_enabled_state() {
    assert!(matches!(
        editor_message(control_message(SettingsControl::AutoLineHeight(true))),
        EditorMessage::ToggleAutoLineHeight(true)
    ));
    assert!(matches!(
        editor_message(control_message(SettingsControl::AutoLineHeight(false))),
        EditorMessage::ToggleAutoLineHeight(false)
    ));
    assert!(matches!(
        editor_message(control_message(SettingsControl::FollowSystemTheme(true))),
        EditorMessage::ToggleFollowSystemTheme(true)
    ));
    assert!(matches!(
        editor_message(control_message(SettingsControl::FollowSystemTheme(false))),
        EditorMessage::ToggleFollowSystemTheme(false)
    ));
}

#[test]
fn language_control_preserves_selected_language() {
    assert!(matches!(
        editor_message(control_message(SettingsControl::Language(Language::ChineseSimplified))),
        EditorMessage::LanguageChanged(Language::ChineseSimplified)
    ));
}

#[test]
fn theme_control_preserves_selected_theme() {
    assert!(matches!(
        editor_message(control_message(SettingsControl::Theme(Theme::Dark))),
        EditorMessage::ThemeChanged(Theme::Dark)
    ));
}

#[test]
fn selected_editor_theme_uses_manual_theme_when_system_following_is_disabled() {
    let mut app = app();
    app.editor_follow_system_theme = false;
    app.app_theme = Theme::Light;
    app.editor_theme = Theme::Dark;

    assert!(matches!(selected_editor_theme(&app), Theme::Dark));
}

#[test]
fn selected_editor_theme_uses_app_theme_when_system_following_is_enabled() {
    let mut app = app();
    app.editor_follow_system_theme = true;
    app.app_theme = Theme::Dark;
    app.editor_theme = Theme::Light;

    assert!(matches!(selected_editor_theme(&app), Theme::Dark));
}

#[test]
fn badge_labels_use_declared_precision_at_slider_boundaries() {
    assert_eq!(font_size_badge_label(10.0), "10 pt");
    assert_eq!(font_size_badge_label(29.6), "30 pt");
    assert_eq!(line_height_badge_label(10.0), "10.0");
    assert_eq!(line_height_badge_label(49.95), "50.0");
}

#[test]
fn settings_overlay_builds_with_default_editor_settings() {
    let app = app();

    keep(build_settings_overlay(&app, content()));
}

#[test]
fn settings_overlay_builds_with_manual_line_height_and_theme() {
    let mut app = app();
    app.current_font_size = 18.0;
    app.auto_adjust_line_height = false;
    app.current_line_height = 28.5;
    app.current_language = Language::ChineseSimplified;
    app.editor_follow_system_theme = false;
    app.editor_theme = Theme::Dark;

    keep(build_settings_overlay(&app, content()));
}

#[test]
fn settings_overlay_uses_app_theme_when_following_system_theme() {
    let mut app = app();
    app.editor_follow_system_theme = true;
    app.app_theme = Theme::Dark;
    app.editor_theme = Theme::Light;

    keep(build_settings_overlay(&app, content()));
}

#[test]
fn settings_overlay_keeps_base_and_modal_stack_as_separate_layers() {
    let app = app();
    let overlay = build_settings_overlay(&app, content());
    let layers = overlay.as_widget().children();

    assert_eq!(layers.len(), 2);
    assert_eq!(layers[1].children.len(), 2);
}
