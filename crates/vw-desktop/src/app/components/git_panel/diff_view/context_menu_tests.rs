//! 验证差异上下文菜单的行侧匹配、指针数据、交互状态与主题样式。
//! 同一文件同一行仍需区分新旧侧，避免菜单操作落到另一侧的对应行。

use super::context_menu::{
    diff_context_menu_matches, diff_selection_menu, diff_selection_menu_button_style,
    diff_selection_menu_container_style, is_dark_theme, open_diff_context_menu_message,
    wrap_diff_row_with_context_menu,
};
use crate::app::state::GitDiffContextMenuState;
use crate::app::{App, Message, message};
use iced::widget::{button, text};
use iced::{Background, Color, Point, Theme};

fn app() -> App {
    App::new().0
}

fn context_menu(file: &str, line: usize, is_old: bool) -> GitDiffContextMenuState {
    GitDiffContextMenuState { file: file.to_string(), line, is_old, x: 12.0, y: 24.0 }
}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn assert_color_close(actual: Color, expected: Color) {
    assert!((actual.r - expected.r).abs() < 0.0001, "red: {actual:?} != {expected:?}");
    assert!((actual.g - expected.g).abs() < 0.0001, "green: {actual:?} != {expected:?}");
    assert!((actual.b - expected.b).abs() < 0.0001, "blue: {actual:?} != {expected:?}");
    assert!((actual.a - expected.a).abs() < 0.0001, "alpha: {actual:?} != {expected:?}");
}

#[test]
fn task_701_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("context_menu_tests.rs"));
}

#[test]
fn dark_theme_detection_uses_strict_brightness_threshold() {
    let mut threshold_palette = iced::theme::Palette::DARK;
    threshold_palette.background = Color::from_rgb(0.5, 0.5, 0.5);
    let threshold = Theme::custom("threshold", threshold_palette);

    let mut below_palette = iced::theme::Palette::DARK;
    below_palette.background = Color::from_rgb(0.499, 0.5, 0.5);
    let below = Theme::custom("below-threshold", below_palette);

    assert!(!is_dark_theme(&threshold));
    assert!(is_dark_theme(&below));
    assert!(!is_dark_theme(&Theme::Light));
    assert!(is_dark_theme(&Theme::Dark));
}

#[test]
fn menu_button_style_covers_all_status_and_theme_branches() {
    let light = Theme::Light;
    let dark = Theme::Dark;
    let light_ext = light.extended_palette();
    let dark_ext = dark.extended_palette();

    let active = diff_selection_menu_button_style(&light, button::Status::Active);
    let disabled = diff_selection_menu_button_style(&dark, button::Status::Disabled);
    let light_hovered = diff_selection_menu_button_style(&light, button::Status::Hovered);
    let dark_hovered = diff_selection_menu_button_style(&dark, button::Status::Hovered);
    let light_pressed = diff_selection_menu_button_style(&light, button::Status::Pressed);
    let dark_pressed = diff_selection_menu_button_style(&dark, button::Status::Pressed);

    assert!(active.background.is_none());
    assert!(disabled.background.is_none());
    assert_eq!(active.border.width, 0.0);
    assert_eq!(active.border.color, Color::TRANSPARENT);
    assert_eq!(active.border.radius.top_left, 5.0);
    assert_eq!(active.text_color, light.palette().text);
    assert_color_close(
        background_color(light_hovered.background),
        light_ext.background.weak.color.scale_alpha(0.72),
    );
    assert_color_close(
        background_color(dark_hovered.background),
        dark_ext.background.weak.color.scale_alpha(0.3),
    );
    assert_color_close(
        background_color(light_pressed.background),
        light_ext.background.strong.color.scale_alpha(0.18),
    );
    assert_color_close(
        background_color(dark_pressed.background),
        dark_ext.background.strong.color.scale_alpha(0.34),
    );
}

#[test]
fn menu_container_style_uses_theme_specific_surface_border_and_shadow() {
    for (theme, surface_alpha, border_alpha, shadow_alpha) in
        [(Theme::Light, 0.99, 0.74, 0.08), (Theme::Dark, 0.96, 0.52, 0.20)]
    {
        let ext = theme.extended_palette();
        let style = diff_selection_menu_container_style(&theme);

        assert_color_close(
            background_color(style.background),
            ext.background.base.color.scale_alpha(surface_alpha),
        );
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.border.radius.top_left, 10.0);
        assert_color_close(
            style.border.color,
            ext.background.strong.color.scale_alpha(border_alpha),
        );
        assert_color_close(style.shadow.color, Color::BLACK.scale_alpha(shadow_alpha));
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 4.0));
        assert_eq!(style.shadow.blur_radius, 10.0);
    }
}

#[test]
fn open_context_menu_message_preserves_row_and_pointer_data() {
    let message = open_diff_context_menu_message(
        "src/lib.rs",
        usize::MAX,
        true,
        "-removed",
        Point::new(-3.5, 42.25),
    );

    match message {
        Message::Git(message::GitMessage::OpenDiffContextMenu {
            file,
            line,
            is_old,
            text,
            x,
            y,
        }) => {
            assert_eq!(file, "src/lib.rs");
            assert_eq!(line, usize::MAX);
            assert!(is_old);
            assert_eq!(text, "-removed");
            assert_eq!(x, -3.5);
            assert_eq!(y, 42.25);
        }
        other => panic!("expected OpenDiffContextMenu, got {other:?}"),
    }
}

#[test]
fn open_context_menu_message_preserves_empty_new_side_boundary_data() {
    let message = open_diff_context_menu_message("", 0, false, "", Point::ORIGIN);

    match message {
        Message::Git(message::GitMessage::OpenDiffContextMenu {
            file,
            line,
            is_old,
            text,
            x,
            y,
        }) => {
            assert!(file.is_empty());
            assert_eq!(line, 0);
            assert!(!is_old);
            assert!(text.is_empty());
            assert_eq!((x, y), (0.0, 0.0));
        }
        other => panic!("expected OpenDiffContextMenu, got {other:?}"),
    }
}

#[test]
fn context_menu_match_requires_same_file_line_and_side() {
    let menu = context_menu("src/lib.rs", 7, false);

    assert!(diff_context_menu_matches(&menu, "src/lib.rs", 7, false));
    assert!(!diff_context_menu_matches(&menu, "src/main.rs", 7, false));
    assert!(!diff_context_menu_matches(&menu, "src/lib.rs", 8, false));
    assert!(!diff_context_menu_matches(&menu, "src/lib.rs", 7, true));
}

#[test]
fn selection_menu_and_context_wrapper_build_for_absent_mismatched_and_matching_states() {
    let mut app = app();

    let _menu = diff_selection_menu();
    let _absent = wrap_diff_row_with_context_menu(
        &app,
        "src/lib.rs",
        2,
        false,
        "let x = 1;".to_string(),
        text("row").into(),
    );

    for menu in [
        context_menu("src/main.rs", 2, false),
        context_menu("src/lib.rs", 3, false),
        context_menu("src/lib.rs", 2, true),
    ] {
        app.git_diff_context_menu = Some(menu);
        let _mismatched = wrap_diff_row_with_context_menu(
            &app,
            "src/lib.rs",
            2,
            false,
            "let x = 1;".to_string(),
            text("row").into(),
        );
    }

    app.git_diff_context_menu = Some(context_menu("src/lib.rs", 2, false));
    let _matching = wrap_diff_row_with_context_menu(
        &app,
        "src/lib.rs",
        2,
        false,
        "let x = 1;".to_string(),
        text("row").into(),
    );
}
