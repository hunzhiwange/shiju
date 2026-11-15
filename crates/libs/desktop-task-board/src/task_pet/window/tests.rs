#[cfg(not(target_arch = "wasm32"))]
#[test]
fn task_pet_window_settings_are_fixed_transparent_and_always_on_top() {
    let size = iced::Size::new(320.0, 180.0);
    let position = iced::Point::new(48.0, 72.0);

    let settings = super::task_pet_window_settings(size, position);

    assert_eq!(settings.size, size);
    assert_eq!(settings.min_size, None);
    assert_eq!(settings.max_size, None);
    match settings.position {
        iced::window::Position::Specific(actual) => assert_eq!(actual, position),
        other => panic!("expected specific position, got {other:?}"),
    }
    assert!(!settings.resizable);
    assert!(!settings.decorations);
    assert!(settings.transparent);
    assert_eq!(settings.level, iced::window::Level::AlwaysOnTop);
    assert!(!settings.exit_on_close_request);
}
