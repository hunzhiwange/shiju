use iced::Color;

use super::{
    contrast_text_color, lighten_color, mix_color, project_accent_color, project_badge_label,
    session_title_max_chars, stable_hash32, truncate_display_width, workspace_path_for_tooltip,
};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("utils_tests"));
}

#[test]
fn project_badge_label_uses_first_ascii_alphanumeric_uppercase() {
    assert_eq!(project_badge_label("  alpha project"), "A");
}

#[test]
fn project_badge_label_uses_digit_without_case_conversion() {
    assert_eq!(project_badge_label("  7 rivers"), "7");
}

#[test]
fn project_badge_label_uses_unicode_alphanumeric() {
    assert_eq!(project_badge_label("  窗口"), "窗");
}

#[test]
fn project_badge_label_prefers_later_alphanumeric_over_punctuation() {
    assert_eq!(project_badge_label("  - beta"), "B");
}

#[test]
fn project_badge_label_falls_back_to_first_non_whitespace() {
    assert_eq!(project_badge_label("  /-"), "/");
}

#[test]
fn project_badge_label_returns_question_mark_for_blank_title() {
    assert_eq!(project_badge_label(" \n\t"), "?");
}

#[test]
fn stable_hash32_returns_fnv_offset_for_empty_input() {
    assert_eq!(stable_hash32(""), 2_166_136_261);
}

#[test]
fn stable_hash32_returns_known_fnv1a_value() {
    assert_eq!(stable_hash32("hello"), 0x4f9f_2cab);
}

#[test]
fn project_accent_color_is_stable_for_same_seed() {
    assert_eq!(project_accent_color("workspace-a"), project_accent_color("workspace-a"));
}

#[test]
fn project_accent_color_selects_palette_entry_from_hash() {
    assert_eq!(project_accent_color(""), Color::from_rgb8(0xFF, 0x4D, 0x7D));
}

#[test]
fn contrast_text_color_uses_white_for_dark_background() {
    assert_eq!(contrast_text_color(Color::from_rgb8(0x20, 0x20, 0x20)), Color::WHITE);
}

#[test]
fn contrast_text_color_uses_dark_text_for_light_background() {
    assert_eq!(
        contrast_text_color(Color::from_rgb8(0xF0, 0xF0, 0xF0)),
        Color::from_rgb8(18, 18, 18)
    );
}

#[test]
fn contrast_text_color_uses_white_at_threshold() {
    assert_eq!(contrast_text_color(Color::from_rgb(0.62, 0.62, 0.62)), Color::WHITE);
}

#[test]
fn contrast_text_color_weights_green_more_than_red() {
    assert_eq!(contrast_text_color(Color::from_rgb(0.0, 1.0, 0.0)), Color::from_rgb8(18, 18, 18));
    assert_eq!(contrast_text_color(Color::from_rgb(1.0, 0.0, 0.0)), Color::WHITE);
}

#[test]
fn lighten_color_blends_toward_white() {
    let color = lighten_color(Color::from_rgb(0.2, 0.4, 0.6));

    assert_color_close(color, Color::from_rgb(0.8, 0.85, 0.9));
}

#[test]
fn lighten_color_turns_black_into_dark_gray() {
    assert_color_close(lighten_color(Color::BLACK), Color::from_rgb(0.75, 0.75, 0.75));
}

#[test]
fn lighten_color_leaves_white_unchanged() {
    assert_color_close(lighten_color(Color::WHITE), Color::WHITE);
}

#[test]
fn lighten_color_returns_an_opaque_color() {
    assert_color_close(
        lighten_color(Color::from_rgba(0.2, 0.4, 0.6, 0.25)),
        Color::from_rgb(0.8, 0.85, 0.9),
    );
}

#[test]
fn contrast_text_color_uses_dark_text_for_bright_background() {
    assert_eq!(contrast_text_color(Color::from_rgb(0.8, 0.8, 0.8)), Color::from_rgb8(18, 18, 18));
}

#[test]
fn session_title_max_chars_uses_minimum_available_width_for_narrow_panel() {
    assert_eq!(session_title_max_chars(100.0), 18);
}

#[test]
fn session_title_max_chars_uses_panel_width_after_reserved_space() {
    assert_eq!(session_title_max_chars(300.0), 34);
}

#[test]
fn session_title_max_chars_keeps_minimum_available_width_at_boundary() {
    assert_eq!(session_title_max_chars(220.0), 18);
    assert_eq!(session_title_max_chars(225.0), 19);
}

#[test]
fn session_title_max_chars_scales_with_very_wide_panel() {
    assert_eq!(session_title_max_chars(2000.0), 367);
}

#[test]
fn truncate_display_width_returns_original_when_width_fits() {
    assert_eq!(truncate_display_width("Hello", 5), "Hello");
}

#[test]
fn truncate_display_width_returns_empty_for_empty_input() {
    assert_eq!(truncate_display_width("", 10), "");
}

#[test]
fn truncate_display_width_returns_empty_for_zero_width() {
    assert_eq!(truncate_display_width("Hello", 0), "");
}

#[test]
fn truncate_display_width_returns_ellipsis_for_width_one() {
    assert_eq!(truncate_display_width("Hello", 1), "…");
}

#[test]
fn truncate_display_width_truncates_ascii_with_ellipsis() {
    assert_eq!(truncate_display_width("Hello World", 8), "Hello W…");
}

#[test]
fn truncate_display_width_respects_wide_unicode_characters() {
    assert_eq!(truncate_display_width("你好世界", 5), "你好…");
}

#[test]
fn truncate_display_width_returns_wide_unicode_when_it_exactly_fits() {
    assert_eq!(truncate_display_width("你好", 4), "你好");
}

#[test]
fn truncate_display_width_keeps_zero_width_combining_mark() {
    assert_eq!(truncate_display_width("a\u{0301}bc", 2), "a\u{0301}…");
}

#[test]
fn truncate_display_width_skips_character_that_would_exceed_keep_width() {
    assert_eq!(truncate_display_width("你a", 2), "…");
}

#[test]
fn workspace_path_for_tooltip_returns_dot_for_project_root() {
    assert_eq!(workspace_path_for_tooltip("/tmp/project", "/tmp/project"), ".");
}

#[test]
fn workspace_path_for_tooltip_returns_relative_path_for_child_workspace() {
    assert_eq!(workspace_path_for_tooltip("/tmp/project/src/app", "/tmp/project"), "src/app");
}

#[test]
fn workspace_path_for_tooltip_returns_original_path_for_external_workspace() {
    assert_eq!(workspace_path_for_tooltip("/tmp/other", "/tmp/project"), "/tmp/other");
}

#[test]
fn workspace_path_for_tooltip_rejects_similar_prefix_sibling() {
    assert_eq!(
        workspace_path_for_tooltip("/tmp/project-copy/src", "/tmp/project"),
        "/tmp/project-copy/src"
    );
}

#[test]
fn mix_color_returns_first_color_when_t_is_below_range() {
    let first = Color::from_rgba(0.1, 0.2, 0.3, 0.4);
    let second = Color::from_rgba(0.9, 0.8, 0.7, 0.6);

    assert_color_close(mix_color(first, second, -1.0), first);
}

#[test]
fn mix_color_returns_second_color_when_t_is_above_range() {
    let first = Color::from_rgba(0.1, 0.2, 0.3, 0.4);
    let second = Color::from_rgba(0.9, 0.8, 0.7, 0.6);

    assert_color_close(mix_color(first, second, 2.0), second);
}

#[test]
fn mix_color_interpolates_all_channels() {
    let first = Color::from_rgba(0.0, 0.2, 0.4, 0.6);
    let second = Color::from_rgba(1.0, 0.6, 0.8, 1.0);

    assert_color_close(mix_color(first, second, 0.25), Color::from_rgba(0.25, 0.3, 0.5, 0.7));
}

fn assert_color_close(actual: Color, expected: Color) {
    const EPSILON: f32 = 0.000_001;

    assert!((actual.r - expected.r).abs() < EPSILON);
    assert!((actual.g - expected.g).abs() < EPSILON);
    assert!((actual.b - expected.b).abs() < EPSILON);
    assert!((actual.a - expected.a).abs() < EPSILON);
}
