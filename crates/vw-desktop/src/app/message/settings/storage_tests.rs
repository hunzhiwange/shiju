use super::*;
use crate::app::message::settings::StorageMessage;
fn app() -> App {
    App::new().0
}
#[test]
fn non_storage_message_leaves_storage_state_unchanged() {
    let mut app = app();
    app.storage_settings.provider = "sqlite".to_string();
    app.storage_settings.save_error = Some("keep this error".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert_eq!(app.storage_settings.provider, "sqlite");
    assert_eq!(app.storage_settings.save_error.as_deref(), Some("keep this error"));
}
