#[cfg(not(target_arch = "wasm32"))]
#[test]
fn is_dark_theme_detects_dark_and_light_palettes() {
    assert!(super::is_dark_theme(&iced::Theme::Dark));
    assert!(!super::is_dark_theme(&iced::Theme::Light));
}
