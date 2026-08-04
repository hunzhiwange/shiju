use std::path::PathBuf;

use super::{
    ChromeStyleOverrides, DEFAULT_WALLPAPER_PANEL_ALPHA, input_surface_color, panel_surface_color,
    resolve_chrome_style, resolve_chrome_style_from_skin, resolve_chrome_style_from_theme,
    resolve_chrome_style_with_wallpaper,
};
use crate::app::theme_catalog::{ActiveThemeSkin, ThemeChrome};
use iced::{Color, Theme};

fn assert_color(actual: Color, expected: Color) {
    const EPSILON: f32 = 1.0e-5;
    assert!(
        (actual.r - expected.r).abs() < EPSILON
            && (actual.g - expected.g).abs() < EPSILON
            && (actual.b - expected.b).abs() < EPSILON
            && (actual.a - expected.a).abs() < EPSILON,
        "expected {expected:?}, got {actual:?}"
    );
}

#[test]
fn light_panel_surface_softens_pure_white() {
    let surface = panel_surface_color(&Theme::Light, 0.985);
    let white = Color::WHITE;
    assert!(
        surface.r < white.r && surface.g < white.g && surface.b < white.b,
        "light panel should be softer than pure white, got {surface:?}"
    );
    assert!(
        surface.r > 0.95 && surface.g > 0.95 && surface.b > 0.95,
        "light panel should stay very light, got {surface:?}"
    );
    assert!((surface.a - 0.985).abs() < 1.0e-5);
}

#[test]
fn light_input_surface_avoids_muddy_weak_gray() {
    let input = input_surface_color(&Theme::Light, 1.0);
    let weak = Theme::Light.extended_palette().background.weak.color;
    assert_color(input, Color { a: 1.0, ..Theme::Light.palette().background });
    assert!(
        input.r > weak.r && input.g > weak.g && input.b > weak.b,
        "light input should be lighter than background.weak ({weak:?}), got {input:?}"
    );
}

#[test]
fn dark_input_surface_still_uses_weak_elevation() {
    let input = input_surface_color(&Theme::Dark, 0.96);
    let weak = Theme::Dark.extended_palette().background.weak.color;
    assert_color(input, Color { a: 0.96, ..weak });
}

#[test]
fn chrome_style_overrides_input_background() {
    let chrome = ThemeChrome {
        input_bg: Some("#112233".to_string()),
        input_border: Some("#445566".to_string()),
        panel_alpha: Some(200),
        radius_px: Some(14),
        ..ThemeChrome::default()
    };
    let theme = Theme::Dark;
    let overrides = resolve_chrome_style(&chrome, &theme.palette());

    assert_color(
        overrides.input_bg.expect("input_bg"),
        Color { a: 200.0 / 255.0, ..Color::from_rgb8(0x11, 0x22, 0x33) },
    );
    assert_color(overrides.input_border.expect("input_border"), Color::from_rgb8(0x44, 0x55, 0x66));
    // 仅设背景时文字回退 palette.text
    assert_color(overrides.input_text.expect("input_text fallback"), theme.palette().text);
    assert_eq!(overrides.radius_px, Some(14.0));
}

#[test]
fn chrome_style_zone_alphas_override_global_panel_alpha() {
    let chrome = ThemeChrome {
        panel_alpha: Some(100),
        chat_header_alpha: Some(40),
        chat_body_alpha: Some(180),
        chat_input_alpha: Some(220),
        ..ThemeChrome::default()
    };
    let theme = Theme::Dark;
    let overrides = resolve_chrome_style_with_wallpaper(&chrome, &theme.palette(), true);

    assert!((overrides.chat_header_alpha.expect("header") - 40.0 / 255.0).abs() < 1.0e-5);
    assert!((overrides.panel_alpha.expect("body") - 180.0 / 255.0).abs() < 1.0e-5);
    assert!((overrides.chat_input_alpha.expect("input") - 220.0 / 255.0).abs() < 1.0e-5);
    assert!((overrides.input_bg.expect("input_bg").a - 220.0 / 255.0).abs() < 1.0e-5);
    assert!((overrides.panel_bg.expect("panel_bg").a - 180.0 / 255.0).abs() < 1.0e-5);
    assert!((overrides.chat_header_bg.expect("header_bg").a - 40.0 / 255.0).abs() < 1.0e-5);
}

#[test]
fn chrome_style_empty_yields_no_overrides() {
    let overrides = resolve_chrome_style(&ThemeChrome::default(), &Theme::Dark.palette());
    assert_eq!(overrides, ChromeStyleOverrides::default());
    assert_eq!(
        resolve_chrome_style_from_theme(None, &Theme::Light),
        ChromeStyleOverrides::default()
    );
}

#[test]
fn chrome_style_applies_default_alpha_with_wallpaper() {
    let theme = Theme::Dark;
    let with_wallpaper = ActiveThemeSkin {
        theme_id: "wallpaper-theme".to_string(),
        background_path: Some(PathBuf::from("/tmp/vibe-window-wallpaper.png")),
        cover_path: None,
        chat_header_path: None,
        chat_body_path: None,
        chat_input_path: None,
        chrome: ThemeChrome::default(),
    };
    let overrides = resolve_chrome_style_from_skin(Some(&with_wallpaper), &theme);

    let alpha = overrides.panel_alpha.expect("default panel_alpha with wallpaper");
    assert!(
        (alpha - DEFAULT_WALLPAPER_PANEL_ALPHA).abs() < 1.0e-5,
        "expected default wallpaper alpha {DEFAULT_WALLPAPER_PANEL_ALPHA}, got {alpha}"
    );
    assert!(alpha < 1.0);

    let input_bg = overrides.input_bg.expect("input_bg synthesized with wallpaper");
    assert!(
        (input_bg.a - alpha).abs() < 1.0e-5,
        "input_bg alpha should match panel_alpha, got {:?}",
        input_bg
    );
    let panel_bg = overrides.panel_bg.expect("panel_bg synthesized with wallpaper");
    assert!(
        (panel_bg.a - alpha).abs() < 1.0e-5,
        "panel_bg alpha should match panel_alpha, got {:?}",
        panel_bg
    );
    assert_color(
        overrides.input_text.expect("input_text fallback with wallpaper"),
        theme.palette().text,
    );

    let input_rgb = Color { a: 1.0, ..overrides.input_bg.expect("input_bg") };
    assert_color(input_rgb, theme.palette().background);
    let panel_rgb = Color { a: 1.0, ..overrides.panel_bg.expect("panel_bg") };
    assert_color(panel_rgb, theme.palette().background);

    let explicit = ActiveThemeSkin {
        theme_id: "wallpaper-theme".to_string(),
        background_path: Some(PathBuf::from("/tmp/vibe-window-wallpaper.png")),
        cover_path: None,
        chat_header_path: None,
        chat_body_path: None,
        chat_input_path: None,
        chrome: ThemeChrome { panel_alpha: Some(100), ..ThemeChrome::default() },
    };
    let explicit_overrides = resolve_chrome_style_from_skin(Some(&explicit), &theme);
    assert!((explicit_overrides.panel_alpha.expect("explicit") - 100.0 / 255.0).abs() < 1.0e-5);
    assert!((explicit_overrides.input_bg.expect("input_bg").a - 100.0 / 255.0).abs() < 1.0e-5);

    let no_wallpaper = ActiveThemeSkin {
        theme_id: "plain-theme".to_string(),
        background_path: None,
        cover_path: None,
        chat_header_path: None,
        chat_body_path: None,
        chat_input_path: None,
        chrome: ThemeChrome::default(),
    };
    assert_eq!(
        resolve_chrome_style_from_skin(Some(&no_wallpaper), &theme),
        ChromeStyleOverrides::default()
    );
}
