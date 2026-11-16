use crate::messages::SettingsMessage;
use crate::multimodal::update;
#[test]
fn update_ignores_messages_for_other_settings_sections() {
    let mut app = crate::state::MultimodalSettingsState::default();
    app.max_images = 7;
    app.max_image_size_mb = 12;
    app.allow_remote_fetch = true;
    app.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ReliabilityHelpClose);

    assert_eq!(app.max_images, 7);
    assert_eq!(app.max_image_size_mb, 12);
    assert!(app.allow_remote_fetch);
    assert_eq!(app.save_error.as_deref(), Some("keep"));
}
