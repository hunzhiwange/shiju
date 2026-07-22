use super::*;
use crate::app::{App, PreviewAutoSaveMode, Shell, TerminalTheme, message};
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn system_settings_general_tests_are_wired() {
    assert!(module_path!().contains("system_settings_general_tests"));
}

#[test]
fn setting_row_builds_text_and_control_columns() {
    keep_element(setting_row("标签", "设置说明", text("control")));
}

#[test]
fn selection_helpers_preserve_theme_shell_and_terminal_theme() {
    match app_theme_selected(iced::Theme::Dark) {
        Message::View(message::ViewMessage::AppThemeSelected(theme)) => {
            assert_eq!(theme, iced::Theme::Dark);
        }
        other => panic!("unexpected theme message: {other:?}"),
    }

    for shell in Shell::all() {
        match terminal_shell_selected(shell) {
            Message::Terminal(message::TerminalMessage::ShellSelected(selected)) => {
                assert_eq!(selected, shell);
            }
            other => panic!("unexpected shell message: {other:?}"),
        }
    }

    match terminal_theme_selected(TerminalTheme::System) {
        Message::Terminal(message::TerminalMessage::ThemeSelected(theme)) => {
            assert_eq!(theme, TerminalTheme::System);
        }
        other => panic!("unexpected terminal theme message: {other:?}"),
    }
}

#[test]
fn terminal_font_helper_preserves_empty_and_named_families() {
    for font_family in ["", "JetBrains Mono", "自定义字体"] {
        match terminal_font_selected(font_family.to_string()) {
            Message::Terminal(message::TerminalMessage::FontSelected(selected)) => {
                assert_eq!(selected, font_family);
            }
            other => panic!("unexpected terminal font message: {other:?}"),
        }
    }
}

#[test]
fn auto_save_helper_preserves_every_supported_mode() {
    for mode in PreviewAutoSaveMode::ALL {
        match preview_auto_save_mode_changed(mode) {
            Message::Preview(message::PreviewMessage::AutoSaveModeChanged(selected)) => {
                assert_eq!(selected, mode);
            }
            other => panic!("unexpected auto-save message: {other:?}"),
        }
    }
}

#[test]
fn font_size_helper_preserves_slider_boundaries_and_fraction() {
    for font_size in [10.0, 13.5, 18.0] {
        match terminal_font_size_changed(font_size) {
            Message::Terminal(message::TerminalMessage::FontSizeChanged(selected)) => {
                assert_eq!(selected, font_size);
            }
            other => panic!("unexpected terminal font size message: {other:?}"),
        }
    }
}

#[test]
fn view_builds_supported_general_setting_combinations() {
    let mut app = test_app();
    keep_element(view(&app));

    app.app_theme = iced::Theme::Dark;
    for shell in Shell::all() {
        app.terminal.shell = shell;
        for (font_family, font_size, auto_save_mode) in [
            ("JetBrains Mono", 10.0, PreviewAutoSaveMode::Off),
            ("Fira Code", 13.5, PreviewAutoSaveMode::AfterDelay),
            ("Menlo", 16.0, PreviewAutoSaveMode::OnFocusChange),
            ("Monaco", 18.0, PreviewAutoSaveMode::OnWindowChange),
        ] {
            app.terminal.font_family = font_family.to_string();
            app.terminal.font_size = font_size;
            app.preview_auto_save_mode = auto_save_mode;
            keep_element(view(&app));
        }
    }
}
