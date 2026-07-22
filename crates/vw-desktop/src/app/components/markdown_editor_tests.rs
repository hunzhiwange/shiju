use super::*;
use iced::theme::Palette;
use iced::widget::{button, markdown, text_editor};

struct TestViewer;

impl<'a> markdown::Viewer<'a, ()> for TestViewer {
    fn on_link_click(_url: markdown::Uri) {}
}

fn ignore_mode(_mode: MarkdownViewMode) {}

fn ignore_editor_action(_action: text_editor::Action) {}

fn background_color(background: Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => color,
        other => panic!("expected color background, got {other:?}"),
    }
}

#[test]
fn markdown_view_mode_preserves_copy_equality_and_debug_names() {
    for (mode, name) in [
        (MarkdownViewMode::Edit, "Edit"),
        (MarkdownViewMode::Preview, "Preview"),
        (MarkdownViewMode::Split, "Split"),
    ] {
        let copied = mode;

        assert_eq!(copied, mode);
        assert_eq!(format!("{mode:?}"), name);
    }

    assert_ne!(MarkdownViewMode::Edit, MarkdownViewMode::Preview);
    assert_ne!(MarkdownViewMode::Preview, MarkdownViewMode::Split);
}

#[test]
fn dark_theme_detection_respects_the_brightness_boundary() {
    let mut boundary_palette = Palette::LIGHT;
    boundary_palette.background = Color::from_rgb(0.5, 0.5, 0.5);
    let boundary = Theme::custom("boundary", boundary_palette);

    let mut below_palette = Palette::LIGHT;
    below_palette.background = Color::from_rgb(0.49, 0.5, 0.5);
    let below_boundary = Theme::custom("below-boundary", below_palette);

    assert!(is_dark_theme(&Theme::Dark));
    assert!(!is_dark_theme(&Theme::Light));
    assert!(!is_dark_theme(&boundary));
    assert!(is_dark_theme(&below_boundary));
}

#[test]
fn selected_mode_button_style_tracks_theme_and_pointer_status() {
    for theme in [Theme::Light, Theme::Dark] {
        let dark = is_dark_theme(&theme);
        let primary = theme.palette().primary;
        let active = mode_button_style(&theme, button::Status::Active, true);
        let hovered = mode_button_style(&theme, button::Status::Hovered, true);
        let pressed = mode_button_style(&theme, button::Status::Pressed, true);
        let disabled = mode_button_style(&theme, button::Status::Disabled, true);

        assert_eq!(
            background_color(active.background),
            primary.scale_alpha(if dark { 0.18 } else { 0.12 })
        );
        assert_eq!(
            background_color(hovered.background),
            primary.scale_alpha(if dark { 0.24 } else { 0.16 })
        );
        assert_eq!(
            background_color(pressed.background),
            primary.scale_alpha(if dark { 0.30 } else { 0.20 })
        );
        assert_eq!(disabled.background, active.background);

        for style in [active, hovered, pressed, disabled] {
            assert_eq!(style.text_color, primary);
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius, 999.0.into());
            assert_eq!(style.border.color, primary.scale_alpha(if dark { 0.44 } else { 0.28 }));
            assert_eq!(style.shadow.color, primary.scale_alpha(if dark { 0.16 } else { 0.08 }));
            assert_eq!(style.shadow.offset, Vector::new(0.0, 6.0));
            assert_eq!(style.shadow.blur_radius, 16.0);
        }
    }
}

#[test]
fn unselected_mode_button_style_covers_idle_hover_pressed_and_disabled_states() {
    for theme in [Theme::Light, Theme::Dark] {
        let dark = is_dark_theme(&theme);
        let palette = theme.extended_palette();
        let active = mode_button_style(&theme, button::Status::Active, false);
        let hovered = mode_button_style(&theme, button::Status::Hovered, false);
        let pressed = mode_button_style(&theme, button::Status::Pressed, false);
        let disabled = mode_button_style(&theme, button::Status::Disabled, false);

        assert_eq!(background_color(active.background), Color::TRANSPARENT);
        assert_eq!(
            background_color(hovered.background),
            if dark {
                palette.background.weak.color.scale_alpha(0.82)
            } else {
                Color::WHITE.scale_alpha(0.92)
            }
        );
        assert_eq!(
            background_color(pressed.background),
            if dark {
                palette.background.strong.color.scale_alpha(0.88)
            } else {
                palette.background.weak.color.scale_alpha(0.94)
            }
        );
        assert_eq!(disabled.background, active.background);
        assert_eq!(
            hovered.border.color,
            palette.background.strong.color.scale_alpha(if dark { 0.72 } else { 0.16 })
        );

        for style in [active, pressed, disabled] {
            assert_eq!(style.border.color, Color::TRANSPARENT);
        }

        for style in [active, hovered, pressed, disabled] {
            assert_eq!(style.text_color, theme.palette().text.scale_alpha(0.86));
            assert_eq!(style.border.width, 1.0);
            assert_eq!(style.border.radius, 999.0.into());
            assert_eq!(style.shadow, Shadow::default());
        }
    }
}

#[test]
fn mode_switch_builds_with_each_selected_mode() {
    for mode in [MarkdownViewMode::Edit, MarkdownViewMode::Preview, MarkdownViewMode::Split] {
        let element: Element<'_, ()> = mode_switch(mode, ignore_mode);

        std::hint::black_box(element);
    }
}

#[test]
fn view_builds_edit_preview_and_split_layouts_for_light_and_dark_themes() {
    let editor_content = text_editor::Content::with_text("# Draft\n\nBody");
    let preview_content = markdown::Content::parse("# Preview\n\n[link](https://example.invalid)");
    let viewer = TestViewer;

    for theme in [Theme::Light, Theme::Dark] {
        for mode in [MarkdownViewMode::Edit, MarkdownViewMode::Preview, MarkdownViewMode::Split] {
            let element: Element<'_, ()> = view(
                &editor_content,
                &preview_content,
                &theme,
                &viewer,
                mode,
                ignore_editor_action,
            );

            std::hint::black_box(element);
        }
    }
}

#[test]
fn view_builds_all_modes_with_empty_documents() {
    let editor_content = text_editor::Content::with_text("");
    let preview_content = markdown::Content::parse("");
    let viewer = TestViewer;

    for mode in [MarkdownViewMode::Edit, MarkdownViewMode::Preview, MarkdownViewMode::Split] {
        let element: Element<'_, ()> = view(
            &editor_content,
            &preview_content,
            &Theme::Dark,
            &viewer,
            mode,
            ignore_editor_action,
        );

        std::hint::black_box(element);
    }
}
