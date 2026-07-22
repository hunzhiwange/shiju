use super::*;
use crate::app::state::ProjectEditTab;
use iced::Color;

fn assert_color_near(actual: Color, expected: Color) {
    let epsilon = 0.001;
    assert!((actual.r - expected.r).abs() < epsilon, "red mismatch: {actual:?}");
    assert!((actual.g - expected.g).abs() < epsilon, "green mismatch: {actual:?}");
    assert!((actual.b - expected.b).abs() < epsilon, "blue mismatch: {actual:?}");
    assert!((actual.a - expected.a).abs() < epsilon, "alpha mismatch: {actual:?}");
}

#[test]
fn parse_hex_color_accepts_hashless_hash_and_whitespace() {
    assert_color_near(parse_hex_color("60a5fa").unwrap(), Color::from_rgb8(0x60, 0xa5, 0xfa));
    assert_color_near(parse_hex_color("#34d399").unwrap(), Color::from_rgb8(0x34, 0xd3, 0x99));
    assert_color_near(parse_hex_color("  #EF4444  ").unwrap(), Color::from_rgb8(0xef, 0x44, 0x44));
}

#[test]
fn parse_hex_color_rejects_invalid_lengths() {
    assert!(parse_hex_color("").is_none());
    assert!(parse_hex_color("#12345").is_none());
    assert!(parse_hex_color("#1234567").is_none());
}

#[test]
fn parse_hex_color_rejects_invalid_digits_in_each_rgb_channel() {
    assert!(parse_hex_color("#gg3456").is_none());
    assert!(parse_hex_color("#12gg56").is_none());
    assert!(parse_hex_color("#1234gg").is_none());
}

#[test]
fn parse_hex_color_accepts_repeated_hash_prefixes_after_trimming() {
    assert_color_near(parse_hex_color("  ##123456  ").unwrap(), Color::from_rgb8(0x12, 0x34, 0x56));
}

#[test]
fn format_hex_color_rounds_rgb_channels_to_lowercase_hex() {
    assert_eq!(format_hex_color(Color::from_rgb8(0x60, 0xa5, 0xfa)), "#60a5fa");
    assert_eq!(format_hex_color(Color::from_rgb(1.0, 0.5, 0.0)), "#ff8000");
}

#[test]
fn format_hex_color_discards_alpha_and_handles_rgb_extremes() {
    assert_eq!(format_hex_color(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }), "#000000");
    assert_eq!(format_hex_color(Color { r: 1.0, g: 1.0, b: 1.0, a: 0.25 }), "#ffffff");
}

#[test]
fn format_hex_color_saturates_out_of_range_and_nan_channels() {
    assert_eq!(format_hex_color(Color { r: -0.5, g: 1.5, b: f32::NAN, a: 1.0 }), "#00ff00");
}

#[test]
fn icon_image_handle_accepts_existing_plain_and_file_urls() {
    let path = std::env::temp_dir()
        .join(format!("vibe-window-project-edit-common-{}.png", std::process::id()));
    std::fs::write(&path, b"not a real png but enough for a path handle").unwrap();

    assert!(icon_image_handle(path.to_str().unwrap()).is_some());
    assert!(icon_image_handle(&format!("  {}  ", path.display())).is_some());
    assert!(icon_image_handle(&format!("file://{}", path.display())).is_some());
    assert!(icon_image_handle(&format!("file:///{}", path.display())).is_some());

    let _ = std::fs::remove_file(path);
}

#[test]
fn icon_image_handle_rejects_blank_and_missing_paths() {
    assert!(icon_image_handle("   ").is_none());
    assert!(icon_image_handle("/definitely/missing/vibe-window/icon.png").is_none());
}

#[test]
fn icon_image_handle_rejects_empty_file_url_and_non_file_uri() {
    assert!(icon_image_handle("file://").is_none());
    assert!(icon_image_handle("https://example.invalid/vibe-window/icon.png").is_none());
}

#[test]
fn tab_button_builds_every_tab_for_selected_and_unselected_states() {
    let buttons = [
        tab_button("基础信息", ProjectEditTab::General, true),
        tab_button("基础信息", ProjectEditTab::General, false),
        tab_button("启动与工作区", ProjectEditTab::Launch, true),
        tab_button("启动与工作区", ProjectEditTab::Launch, false),
        tab_button("刷新策略", ProjectEditTab::Refresh, true),
        tab_button("刷新策略", ProjectEditTab::Refresh, false),
        tab_button("调度与保护", ProjectEditTab::Scheduling, true),
        tab_button("调度与保护", ProjectEditTab::Scheduling, false),
    ];

    for button in buttons {
        let _ = std::hint::black_box(button);
    }
}
