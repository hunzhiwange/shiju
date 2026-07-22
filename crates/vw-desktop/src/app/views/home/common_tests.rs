use std::fs;

use iced::{Color, Element, Theme};
use tempfile::tempdir;

use super::*;

fn assert_color_close(actual: Color, expected: Color) {
    const EPSILON: f32 = 0.000_001;

    assert!((actual.r - expected.r).abs() < EPSILON);
    assert!((actual.g - expected.g).abs() < EPSILON);
    assert!((actual.b - expected.b).abs() < EPSILON);
    assert!((actual.a - expected.a).abs() < EPSILON);
}

fn keep_element(element: Element<'_, Message>) {
    let _ = std::hint::black_box(element);
}

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("common_tests"));
}

#[test]
fn is_dark_theme_distinguishes_builtin_themes() {
    assert!(!is_dark_theme(&Theme::Light));
    assert!(is_dark_theme(&Theme::Dark));
}

#[test]
fn parse_hex_color_accepts_hash_case_and_surrounding_whitespace() {
    assert_color_close(parse_hex_color(" #0a1BfF ").unwrap(), Color::from_rgb8(10, 27, 255));
    assert_color_close(parse_hex_color("00ff80").unwrap(), Color::from_rgb8(0, 255, 128));
}

#[test]
fn parse_hex_color_rejects_wrong_lengths_and_invalid_digits() {
    for input in ["", "#", "12345", "1234567", "#12xz56", "red"] {
        assert!(parse_hex_color(input).is_none(), "input should be rejected: {input:?}");
    }
}

#[test]
fn icon_image_handle_rejects_blank_and_missing_paths() {
    assert!(icon_image_handle("").is_none());
    assert!(icon_image_handle(" \n\t ").is_none());
    assert!(icon_image_handle("/path/that/does/not/exist/vibe-window-icon.png").is_none());
}

#[test]
fn icon_image_handle_accepts_existing_plain_path_with_whitespace() {
    let directory = tempdir().unwrap();
    let icon_path = directory.path().join("icon.png");
    fs::write(&icon_path, b"decoded only by the renderer").unwrap();

    let padded_path = format!("  {}  ", icon_path.display());
    assert!(icon_image_handle(&padded_path).is_some());
}

#[test]
fn icon_image_handle_accepts_existing_file_uri_path() {
    let directory = tempdir().unwrap();
    let icon_path = directory.path().join("icon.png");
    fs::write(&icon_path, b"decoded only by the renderer").unwrap();

    let file_uri = format!("file:///{}", icon_path.display());
    assert!(icon_image_handle(&file_uri).is_some());
}

#[test]
fn project_badge_label_prefers_first_alphanumeric_character() {
    assert_eq!(project_badge_label("  alpha"), "A");
    assert_eq!(project_badge_label("--beta"), "B");
    assert_eq!(project_badge_label("...7zip"), "7");
    assert_eq!(project_badge_label("  项目"), "项");
}

#[test]
fn project_badge_label_falls_back_to_symbol_or_question_mark() {
    assert_eq!(project_badge_label("  -- ++"), "-");
    assert_eq!(project_badge_label(" \n\t "), "?");
    assert_eq!(project_badge_label(""), "?");
}

#[test]
fn stable_hash32_matches_fnv1a_reference_values() {
    assert_eq!(stable_hash32(""), 0x811c_9dc5);
    assert_eq!(stable_hash32("a"), 0xe40c_292c);
    assert_eq!(stable_hash32("hello"), 0x4f9f_2cab);
}

#[test]
fn project_accent_color_is_deterministic_and_seed_sensitive() {
    assert_color_close(project_accent_color(""), Color::from_rgb8(0xFF, 0x4D, 0x7D));
    assert_color_close(project_accent_color("hello"), Color::from_rgb8(0xF2, 0xA9, 0x00));
    assert_color_close(project_accent_color("hello"), project_accent_color("hello"));
}

#[test]
fn contrast_text_color_covers_dark_light_and_threshold_backgrounds() {
    assert_eq!(contrast_text_color(Color::BLACK), Color::WHITE);
    assert_eq!(contrast_text_color(Color::WHITE), Color::from_rgb8(18, 18, 18));
    assert_eq!(contrast_text_color(Color::from_rgb(0.61, 0.61, 0.61)), Color::WHITE);
    assert_eq!(
        contrast_text_color(Color::from_rgb(0.63, 0.63, 0.63)),
        Color::from_rgb8(18, 18, 18)
    );
}

#[test]
fn emphasis_fonts_use_expected_families_and_bold_weight() {
    let ascii = home_emphasis_font();
    assert_eq!(ascii.family, iced::font::Family::Name("JetBrains Mono"));
    assert_eq!(ascii.weight, iced::font::Weight::Bold);

    let cjk = home_cjk_emphasis_font();
    assert_eq!(cjk.family, iced::font::Family::Name("Noto Sans CJK SC"));
    assert_eq!(cjk.weight, iced::font::Weight::Bold);
}

#[test]
fn lighten_color_moves_rgb_toward_white_and_preserves_alpha() {
    assert_color_close(lighten_color(Color::BLACK), Color::from_rgb(0.75, 0.75, 0.75));
    assert_color_close(lighten_color(Color::WHITE), Color::WHITE);
    assert_color_close(
        lighten_color(Color::from_rgba(0.2, 0.4, 0.8, 0.35)),
        Color::from_rgba(0.8, 0.85, 0.95, 0.35),
    );
}

#[test]
fn icon_and_action_builders_accept_regular_and_boundary_sizes() {
    let _ = std::hint::black_box(icon_svg(Icon::Grid1x2, 14.0));
    let _ = std::hint::black_box(icon_svg(Icon::Grid1x2, 0.0));
    keep_element(primary_button("选择文件夹", Message::None));
    keep_element(secondary_button("", Message::None));
}

#[test]
fn project_avatar_builds_default_ascii_and_cjk_labels() {
    keep_element(project_avatar("alpha", "/tmp/alpha", None, None, 48.0));
    keep_element(project_avatar("项目", "/tmp/project", None, None, 48.0));
}

#[test]
fn project_avatar_builds_custom_text_and_missing_image_fallbacks() {
    keep_element(project_avatar(
        "alpha",
        "/tmp/alpha",
        Some(" A "),
        Some(Color::from_rgb8(30, 90, 180)),
        48.0,
    ));
    keep_element(project_avatar(
        "alpha",
        "/tmp/alpha",
        Some("/missing/custom/icon.png"),
        None,
        0.0,
    ));
    keep_element(project_avatar(
        "alpha",
        "/tmp/alpha",
        Some("   "),
        Some(Color::from_rgb8(30, 90, 180)),
        48.0,
    ));
}

#[test]
fn project_avatar_builds_existing_image_icon_branch() {
    let directory = tempdir().unwrap();
    let icon_path = directory.path().join("avatar.png");
    fs::write(&icon_path, b"decoded only by the renderer").unwrap();
    let icon_path = icon_path.to_string_lossy();

    keep_element(project_avatar(
        "alpha",
        "/tmp/alpha",
        Some(icon_path.as_ref()),
        Some(Color::from_rgb8(30, 90, 180)),
        48.0,
    ));
}
