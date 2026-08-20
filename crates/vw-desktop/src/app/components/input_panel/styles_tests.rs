use super::styles::*;
use iced::widget::{button, text_editor};
use iced::{Background, Color, Theme};

fn background_color(background: &Option<Background>) -> Color {
    match background {
        Some(Background::Color(color)) => *color,
        other => panic!("expected color background, got {other:?}"),
    }
}

fn assert_color(actual: Color, expected: Color) {
    const EPSILON: f32 = 1.0e-6;
    assert!(
        (actual.r - expected.r).abs() < EPSILON
            && (actual.g - expected.g).abs() < EPSILON
            && (actual.b - expected.b).abs() < EPSILON
            && (actual.a - expected.a).abs() < EPSILON,
        "expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn bottom_bar_constants_keep_compact_control_dimensions() {
    assert_eq!(BOTTOM_BAR_ICON_BUTTON_SIZE, 24.0);
    assert_eq!(BOTTOM_BAR_ICON_SIZE, 12.0);
    assert_eq!(BOTTOM_BAR_LARGE_ICON_SIZE, 14.0);
    assert_eq!(BOTTOM_BAR_CHEVRON_ICON_SIZE, 11.0);
    assert_eq!(BOTTOM_BAR_LABEL_SIZE, 12.0);
}

#[test]
fn quick_message_button_style_keeps_light_and_dark_actions_distinguishable() {
    for theme in [Theme::Light, Theme::Dark] {
        let message = quick_message_button_style(&theme, button::Status::Active, false);
        let action = quick_message_button_style(&theme, button::Status::Active, true);

        assert_ne!(message.background, action.background);
        assert_ne!(message.border.color, action.border.color);
        assert_eq!(message.border.radius.top_left, 999.0);
        assert_eq!(action.border.width, 1.0);
    }
}

#[test]
fn quick_message_group_menu_and_action_badge_support_dark_theme() {
    for theme in [Theme::Light, Theme::Dark] {
        let inactive = quick_message_group_button_style(&theme, button::Status::Active, false);
        let active = quick_message_group_button_style(&theme, button::Status::Active, true);
        assert_ne!(inactive.background, active.background);

        let menu = quick_message_menu_style(&theme);
        assert!(menu.background.is_some());
        assert_eq!(menu.border.width, 1.0);

        let badge = quick_message_action_badge_style(&theme);
        assert!(badge.background.is_some());
        assert!(badge.text_color.is_some());
    }
}

#[test]
fn shiju_menu_styles_support_light_and_dark_themes() {
    for theme in [Theme::Light, Theme::Dark] {
        let idle = shiju_skill_button_style(&theme, button::Status::Active, false);
        let selected = shiju_skill_button_style(&theme, button::Status::Active, true);
        let hovered = shiju_skill_button_style(&theme, button::Status::Hovered, false);

        assert_ne!(idle.background, selected.background);
        assert_ne!(idle.background, hovered.background);
        assert_eq!(selected.border.width, 1.0);
        assert_ne!(selected.border.color, Color::TRANSPARENT);
        assert!(selected.text_color.a >= 0.88);

        let menu = quick_message_menu_style(&theme);
        assert!(menu.background.is_some());
        assert_eq!(menu.border.width, 1.0);

        let upgrade = shiju_upgrade_button_style(&theme, button::Status::Active, true);
        let upgrade_hovered = shiju_upgrade_button_style(&theme, button::Status::Hovered, true);
        let upgrade_disabled = shiju_upgrade_button_style(&theme, button::Status::Disabled, false);
        assert!(upgrade.background.is_some());
        assert_ne!(upgrade.background, upgrade_hovered.background);
        assert_eq!(upgrade.border.width, 1.0);
        assert!(upgrade.text_color.a >= 0.94);
        assert!(upgrade_disabled.background.is_none());
        assert_eq!(upgrade_disabled.border.width, 0.0);
        assert!(upgrade_disabled.text_color.a <= 0.42);
    }
}

#[test]
fn tooltip_dark_style_is_theme_independent() {
    for theme in [Theme::Light, Theme::Dark] {
        let style = tooltip_dark_style(&theme);

        assert_color(background_color(&style.background), Color::from_rgba8(12, 13, 15, 0.97));
        assert_eq!(style.text_color, Some(Color::WHITE));
        assert_eq!(style.border.width, 1.0);
        assert_color(style.border.color, Color::from_rgba8(255, 255, 255, 0.08));
        assert_color(style.shadow.color, Color::BLACK.scale_alpha(0.32));
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 6.0));
        assert_eq!(style.shadow.blur_radius, 20.0);
    }
}

#[test]
fn square_icon_button_style_maps_theme_status_and_enabled_state() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let (idle_bg, hover_bg, pressed_bg, idle_border, active_border) = if is_dark {
            (
                Color::from_rgba8(24, 25, 29, 0.92),
                Color::from_rgba8(31, 33, 38, 0.96),
                Color::from_rgba8(36, 38, 44, 0.98),
                Color::from_rgba8(45, 48, 54, 0.95),
                Color::from_rgba8(63, 67, 75, 0.95),
            )
        } else {
            (
                Color::from_rgba8(247, 248, 250, 1.0),
                Color::from_rgba8(241, 243, 246, 1.0),
                Color::from_rgba8(232, 236, 241, 1.0),
                Color::from_rgba8(226, 231, 237, 1.0),
                Color::from_rgba8(210, 216, 224, 1.0),
            )
        };

        for (status, expected_bg, expected_border) in [
            (button::Status::Active, idle_bg, idle_border),
            (button::Status::Hovered, hover_bg, active_border),
            (button::Status::Pressed, pressed_bg, active_border),
            (button::Status::Disabled, idle_bg, idle_border),
        ] {
            let style = square_icon_button_style(&theme, status, true);
            assert_color(background_color(&style.background), expected_bg);
            assert_color(style.border.color, expected_border);
            assert_eq!(style.border.width, 1.0);
            assert_color(
                style.text_color,
                theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 }),
            );
        }

        let disabled = square_icon_button_style(&theme, button::Status::Active, false);
        assert_color(
            disabled.text_color,
            theme.palette().text.scale_alpha(if is_dark { 0.36 } else { 0.42 }),
        );
    }
}

#[test]
fn round_icon_button_style_maps_status_without_border_or_shadow() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let expected_backgrounds = if is_dark {
            [
                Color::from_rgba8(24, 25, 29, 0.92),
                Color::from_rgba8(31, 33, 38, 0.96),
                Color::from_rgba8(36, 38, 44, 0.98),
            ]
        } else {
            [
                Color::from_rgba8(247, 248, 250, 1.0),
                Color::from_rgba8(241, 243, 246, 1.0),
                Color::from_rgba8(232, 236, 241, 1.0),
            ]
        };

        for (status, expected_bg) in [
            (button::Status::Active, expected_backgrounds[0]),
            (button::Status::Hovered, expected_backgrounds[1]),
            (button::Status::Pressed, expected_backgrounds[2]),
            (button::Status::Disabled, expected_backgrounds[0]),
        ] {
            let style = round_icon_button_style(&theme, status, true);
            assert_color(background_color(&style.background), expected_bg);
            assert_eq!(style.border.width, 0.0);
            assert_eq!(style.border.color, Color::TRANSPARENT);
            assert_eq!(style.shadow.blur_radius, 0.0);
        }

        let enabled = round_icon_button_style(&theme, button::Status::Active, true);
        let disabled = round_icon_button_style(&theme, button::Status::Active, false);
        assert_color(
            enabled.text_color,
            theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 }),
        );
        assert_color(
            disabled.text_color,
            theme.palette().text.scale_alpha(if is_dark { 0.36 } else { 0.42 }),
        );
    }
}

#[test]
fn selector_typography_and_colors_map_theme_and_highlight_state() {
    assert_eq!(selector_label_font().weight, iced::font::Weight::Medium);

    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        assert_color(
            selector_text_color(&theme, true),
            theme.palette().text.scale_alpha(if is_dark { 0.97 } else { 0.95 }),
        );
        assert_color(
            selector_text_color(&theme, false),
            theme.palette().text.scale_alpha(if is_dark { 0.92 } else { 0.88 }),
        );
        assert_color(
            selector_chevron_color(&theme, true),
            theme.palette().text.scale_alpha(if is_dark { 0.72 } else { 0.68 }),
        );
        assert_color(
            selector_chevron_color(&theme, false),
            theme.palette().text.scale_alpha(if is_dark { 0.58 } else { 0.62 }),
        );
    }
}

#[test]
fn selector_pill_button_style_covers_all_surface_branches() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        for highlighted in [false, true] {
            let (idle_bg, hover_bg, pressed_bg, idle_border, active_border) =
                match (is_dark, highlighted) {
                    (true, true) => (
                        Color::from_rgba8(28, 30, 35, 0.97),
                        Color::from_rgba8(34, 36, 42, 0.98),
                        Color::from_rgba8(38, 40, 46, 1.0),
                        Color::from_rgba8(70, 74, 83, 0.96),
                        Color::from_rgba8(84, 88, 98, 0.98),
                    ),
                    (true, false) => (
                        Color::from_rgba8(24, 25, 29, 0.92),
                        Color::from_rgba8(31, 33, 38, 0.96),
                        Color::from_rgba8(36, 38, 44, 0.98),
                        Color::from_rgba8(45, 48, 54, 0.92),
                        Color::from_rgba8(63, 67, 75, 0.95),
                    ),
                    (false, true) => (
                        Color::from_rgba8(244, 246, 249, 1.0),
                        Color::from_rgba8(238, 241, 245, 1.0),
                        Color::from_rgba8(232, 236, 241, 1.0),
                        Color::from_rgba8(206, 213, 223, 1.0),
                        Color::from_rgba8(194, 202, 214, 1.0),
                    ),
                    (false, false) => (
                        Color::from_rgba8(248, 249, 251, 1.0),
                        Color::from_rgba8(241, 243, 246, 1.0),
                        Color::from_rgba8(233, 236, 241, 1.0),
                        Color::from_rgba8(226, 231, 237, 1.0),
                        Color::from_rgba8(210, 216, 224, 1.0),
                    ),
                };

            for (status, expected_bg, expected_border) in [
                (button::Status::Active, idle_bg, idle_border),
                (button::Status::Hovered, hover_bg, active_border),
                (button::Status::Pressed, pressed_bg, active_border),
                (button::Status::Disabled, idle_bg, idle_border),
            ] {
                let style = selector_pill_button_style(&theme, status, highlighted);
                assert_color(background_color(&style.background), expected_bg);
                assert_color(style.border.color, expected_border);
                assert_eq!(style.border.width, 1.0);
                assert_color(style.text_color, selector_text_color(&theme, highlighted));

                if highlighted {
                    assert_color(
                        style.shadow.color,
                        Color::BLACK.scale_alpha(if is_dark { 0.10 } else { 0.035 }),
                    );
                    assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 6.0));
                    assert_eq!(style.shadow.blur_radius, 16.0);
                } else {
                    assert_eq!(style.shadow.blur_radius, 0.0);
                }
            }
        }
    }
}

#[test]
fn selectable_list_button_style_covers_selection_and_pointer_states() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let primary = theme.palette().primary;
        let neutral_hover = if is_dark {
            Color::from_rgba8(31, 33, 38, 0.94)
        } else {
            Color::from_rgba8(241, 243, 246, 1.0)
        };
        let neutral_border = if is_dark {
            Color::from_rgba8(58, 61, 69, 0.92)
        } else {
            Color::from_rgba8(218, 223, 230, 1.0)
        };

        let idle = selectable_list_button_style(&theme, button::Status::Active, false);
        assert!(idle.background.is_none());
        assert_eq!(idle.border.width, 0.0);
        assert_eq!(idle.border.color, Color::TRANSPARENT);
        assert_color(
            idle.text_color,
            theme.palette().text.scale_alpha(if is_dark { 0.90 } else { 0.88 }),
        );

        let disabled = selectable_list_button_style(&theme, button::Status::Disabled, false);
        assert!(disabled.background.is_none());
        assert_eq!(disabled.border.width, 0.0);
        assert_eq!(disabled.border.color, Color::TRANSPARENT);

        for status in [button::Status::Hovered, button::Status::Pressed] {
            let style = selectable_list_button_style(&theme, status, false);
            assert_color(background_color(&style.background), neutral_hover);
            assert_eq!(style.border.width, 1.0);
            assert_color(style.border.color, neutral_border);
        }

        let selected = selectable_list_button_style(&theme, button::Status::Active, true);
        assert_color(
            background_color(&selected.background),
            primary.scale_alpha(if is_dark { 0.15 } else { 0.10 }),
        );
        assert_color(selected.border.color, primary.scale_alpha(if is_dark { 0.42 } else { 0.24 }));
        assert_color(selected.text_color, selector_text_color(&theme, true));

        for status in [button::Status::Hovered, button::Status::Pressed] {
            let style = selectable_list_button_style(&theme, status, true);
            assert_color(
                background_color(&style.background),
                primary.scale_alpha(if is_dark { 0.19 } else { 0.13 }),
            );
            assert_eq!(style.border.width, 1.0);
        }
    }
}

#[test]
fn popover_style_maps_dark_and_light_surfaces() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let style = popover_style(&theme);
        let expected_background = if is_dark {
            Color::from_rgba8(20, 21, 24, 0.985)
        } else {
            Color::from_rgba8(252, 252, 253, 0.985)
        };
        let expected_border = if is_dark {
            Color::from_rgba8(44, 47, 53, 0.96)
        } else {
            Color::from_rgba8(226, 231, 237, 1.0)
        };

        assert_color(background_color(&style.background), expected_background);
        assert_eq!(style.border.width, 1.0);
        assert_color(style.border.color, expected_border);
        assert_color(
            style.shadow.color,
            Color::BLACK.scale_alpha(if is_dark { 0.22 } else { 0.08 }),
        );
        assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 10.0));
        assert_eq!(style.shadow.blur_radius, 28.0);
    }
}

#[test]
fn input_card_style_blends_primary_color_only_during_drop_hover() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let base = crate::app::theme_catalog::input_surface_color(
            &theme,
            if is_dark { 0.96 } else { 1.0 },
        );
        let normal_border = if is_dark {
            theme.extended_palette().background.strong.color.scale_alpha(0.72)
        } else {
            theme.extended_palette().background.strong.color.scale_alpha(0.48)
        };
        let primary = theme.palette().primary;
        let blended = Color::from_rgba(
            base.r * 0.94 + primary.r * 0.06,
            base.g * 0.94 + primary.g * 0.06,
            base.b * 0.94 + primary.b * 0.06,
            base.a,
        );

        let normal = input_card_style(&theme, false, None);
        assert_color(background_color(&normal.background), base);
        assert_color(normal.border.color, normal_border);

        let hovered = input_card_style(&theme, true, None);
        assert_color(background_color(&hovered.background), blended);
        assert_color(hovered.border.color, primary.scale_alpha(0.82));

        for style in [normal, hovered] {
            assert_eq!(style.border.width, 1.0);
            assert_color(
                style.shadow.color,
                Color::BLACK.scale_alpha(if is_dark { 0.10 } else { 0.035 }),
            );
            assert_eq!(style.shadow.offset, iced::Vector::new(0.0, 4.0));
            assert_eq!(style.shadow.blur_radius, 14.0);
        }
    }
}

#[test]
fn input_surface_shadow_is_subtle_and_skin_overrides_survive() {
    for (theme, max_shadow_alpha) in [(Theme::Light, 0.04), (Theme::Dark, 0.12)] {
        let style = input_card_style(&theme, false, None);
        assert!(style.shadow.color.a <= max_shadow_alpha);
        assert!(style.shadow.offset.y <= 6.0);
        assert!(style.shadow.blur_radius <= 18.0);
        assert_eq!(style.border.radius, 16.0.into());
    }

    let chrome = crate::app::theme_catalog::ChromeStyleOverrides {
        input_bg: Some(Color::from_rgb8(0x11, 0x22, 0x33)),
        input_border: Some(Color::from_rgb8(0x44, 0x55, 0x66)),
        radius_px: Some(11.0),
        ..Default::default()
    };
    let skinned = input_card_style(&Theme::Dark, false, Some(&chrome));
    assert_color(background_color(&skinned.background), Color::from_rgb8(0x11, 0x22, 0x33));
    assert_color(skinned.border.color, Color::from_rgb8(0x44, 0x55, 0x66));
    assert_eq!(skinned.border.radius, 11.0.into());

    let hovered = input_card_style(&Theme::Dark, true, Some(&chrome));
    assert_color(hovered.border.color, Theme::Dark.palette().primary.scale_alpha(0.82));
    assert_ne!(hovered.background, skinned.background);
}

#[test]
fn manual_context_card_style_tints_surface_by_theme() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let primary = theme.palette().primary;
        let expected_background = if is_dark {
            Color::from_rgba(
                primary.r * 0.20 + 18.0 / 255.0 * 0.80,
                primary.g * 0.20 + 19.0 / 255.0 * 0.80,
                primary.b * 0.20 + 22.0 / 255.0 * 0.80,
                0.96,
            )
        } else {
            Color::from_rgba(
                primary.r * 0.08 + 250.0 / 255.0 * 0.92,
                primary.g * 0.08 + 251.0 / 255.0 * 0.92,
                primary.b * 0.08 + 253.0 / 255.0 * 0.92,
                1.0,
            )
        };
        let style = manual_context_card_style(&theme);

        assert_color(background_color(&style.background), expected_background);
        assert_eq!(style.border.width, 1.0);
        assert_color(style.border.color, primary.scale_alpha(if is_dark { 0.34 } else { 0.24 }));
    }
}

#[test]
fn manual_context_card_button_style_highlights_only_pointer_states() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        for status in [button::Status::Active, button::Status::Disabled] {
            let style = manual_context_card_button_style(&theme, status);
            assert_color(background_color(&style.background), Color::TRANSPARENT);
            assert_eq!(style.border.width, 0.0);
            assert_eq!(style.border.color, Color::TRANSPARENT);
            assert_color(style.text_color, selector_text_color(&theme, true));
        }

        for status in [button::Status::Hovered, button::Status::Pressed] {
            let style = manual_context_card_button_style(&theme, status);
            assert_color(
                background_color(&style.background),
                theme.palette().primary.scale_alpha(if is_dark { 0.10 } else { 0.06 }),
            );
        }
    }
}

#[test]
fn editor_style_maps_requesting_and_theme_colors() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let idle = editor_style(&theme, text_editor::Status::Active, false, None);
        assert_eq!(idle.background, Background::Color(Color::TRANSPARENT));
        assert_eq!(idle.border.width, 0.0);
        assert_eq!(idle.border.color, Color::TRANSPARENT);
        assert_color(
            idle.value,
            theme.palette().text.scale_alpha(if is_dark { 0.96 } else { 0.94 }),
        );
        assert_color(
            idle.selection,
            theme.palette().primary.scale_alpha(if is_dark { 0.26 } else { 0.18 }),
        );
        let expected_placeholder = if is_dark {
            theme.palette().text.scale_alpha(0.48)
        } else {
            theme.extended_palette().secondary.base.text.scale_alpha(0.56)
        };
        assert_color(idle.placeholder, expected_placeholder);

        let requesting =
            editor_style(&theme, text_editor::Status::Focused { is_hovered: true }, true, None);
        assert_color(requesting.value, theme.palette().text.scale_alpha(0.92));
        assert_color(requesting.selection, idle.selection);
        assert_color(requesting.placeholder, idle.placeholder);
    }
}

#[test]
fn subtask_editor_style_maps_dark_and_light_fields() {
    for (theme, is_dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let active = subtask_editor_style(&theme, text_editor::Status::Active);
        let focused =
            subtask_editor_style(&theme, text_editor::Status::Focused { is_hovered: false });
        let expected_background = if is_dark {
            Color::from_rgba8(22, 23, 27, 0.92)
        } else {
            Color::from_rgba8(248, 249, 251, 1.0)
        };
        let expected_border = if is_dark {
            Color::from_rgba8(43, 46, 52, 0.96)
        } else {
            Color::from_rgba8(226, 231, 237, 1.0)
        };
        let expected_placeholder = if is_dark {
            theme.palette().text.scale_alpha(0.48)
        } else {
            theme.extended_palette().secondary.base.text.scale_alpha(0.56)
        };

        for style in [active, focused] {
            assert_eq!(style.background, Background::Color(expected_background));
            assert_eq!(style.border.width, 1.0);
            assert_color(style.border.color, expected_border);
            assert_color(
                style.value,
                theme.palette().text.scale_alpha(if is_dark { 0.94 } else { 0.92 }),
            );
            assert_color(
                style.selection,
                theme.palette().primary.scale_alpha(if is_dark { 0.26 } else { 0.18 }),
            );
            assert_color(style.placeholder, expected_placeholder);
        }
    }
}

#[test]
fn input_panel_style_falls_back_without_active_chrome() {
    for theme in [Theme::Light, Theme::Dark] {
        let with_none = input_card_style(&theme, false, None);
        let with_empty = input_card_style(
            &theme,
            false,
            Some(&crate::app::theme_catalog::ChromeStyleOverrides::default()),
        );
        assert_color(
            background_color(&with_none.background),
            background_color(&with_empty.background),
        );
        assert_color(with_none.border.color, with_empty.border.color);
        assert_eq!(with_none.border.radius, with_empty.border.radius);

        let editor_none = editor_style(&theme, text_editor::Status::Active, false, None);
        let editor_empty = editor_style(
            &theme,
            text_editor::Status::Active,
            false,
            Some(&crate::app::theme_catalog::ChromeStyleOverrides::default()),
        );
        assert_color(editor_none.value, editor_empty.value);
    }
}

#[test]
fn input_card_style_applies_chrome_input_background() {
    let theme = Theme::Dark;
    let chrome = crate::app::theme_catalog::resolve_chrome_style(
        &crate::app::theme_catalog::ThemeChrome {
            input_bg: Some("#112233".to_string()),
            input_border: Some("#445566".to_string()),
            radius_px: Some(12),
            panel_alpha: Some(255),
            ..crate::app::theme_catalog::ThemeChrome::default()
        },
        &theme.palette(),
    );
    let style = input_card_style(&theme, false, Some(&chrome));
    assert_color(background_color(&style.background), Color::from_rgb8(0x11, 0x22, 0x33));
    assert_color(style.border.color, Color::from_rgb8(0x44, 0x55, 0x66));
    assert_eq!(style.border.radius, 12.0.into());
}

#[test]
fn input_card_style_more_transparent_with_wallpaper_chrome() {
    use std::path::PathBuf;

    use crate::app::theme_catalog::{ActiveThemeSkin, ThemeChrome, resolve_chrome_style_from_skin};

    let theme = Theme::Dark;
    let baseline = input_card_style(&theme, false, None);
    let baseline_alpha = background_color(&baseline.background).a;
    assert!(baseline_alpha > 0.9, "baseline input card should stay near-opaque");

    let with_wallpaper = ActiveThemeSkin {
        theme_id: "wallpaper-theme".to_string(),
        background_path: Some(PathBuf::from("/tmp/vibe-window-wallpaper.png")),
        cover_path: None,
        chat_header_path: None,
        chat_body_path: None,
        chat_input_path: None,
        chrome: ThemeChrome::default(),
    };
    let chrome = resolve_chrome_style_from_skin(Some(&with_wallpaper), &theme);
    let wallpaper_style = input_card_style(&theme, false, Some(&chrome));
    let wallpaper_alpha = background_color(&wallpaper_style.background).a;
    assert!(
        wallpaper_alpha < 1.0 && wallpaper_alpha < baseline_alpha,
        "wallpaper chrome should make input card more transparent: baseline={baseline_alpha}, wallpaper={wallpaper_alpha}"
    );

    let no_wallpaper = ActiveThemeSkin {
        theme_id: "plain-theme".to_string(),
        background_path: None,
        cover_path: None,
        chat_header_path: None,
        chat_body_path: None,
        chat_input_path: None,
        chrome: ThemeChrome::default(),
    };
    let plain_chrome = resolve_chrome_style_from_skin(Some(&no_wallpaper), &theme);
    let plain_style = input_card_style(&theme, false, Some(&plain_chrome));
    assert_color(background_color(&plain_style.background), background_color(&baseline.background));

    let explicit = ActiveThemeSkin {
        theme_id: "wallpaper-theme".to_string(),
        background_path: Some(PathBuf::from("/tmp/vibe-window-wallpaper.png")),
        cover_path: None,
        chat_header_path: None,
        chat_body_path: None,
        chat_input_path: None,
        chrome: ThemeChrome { panel_alpha: Some(90), ..ThemeChrome::default() },
    };
    let explicit_chrome = resolve_chrome_style_from_skin(Some(&explicit), &theme);
    let explicit_style = input_card_style(&theme, false, Some(&explicit_chrome));
    assert!(
        (background_color(&explicit_style.background).a - 90.0 / 255.0).abs() < 1.0e-5,
        "explicit panel_alpha should win"
    );
}
