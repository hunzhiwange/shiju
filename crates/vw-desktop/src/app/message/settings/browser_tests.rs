use super::*;
fn app() -> App { App::new().0 }
#[test]
fn update_ignores_unrelated_settings_messages() {
    let mut app = app();
    app.browser_settings.enabled = true;
    app.browser_settings.backend = "native".to_string();
    app.browser_settings.save_error = Some("keep".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.browser_settings.enabled);
    assert_eq!(app.browser_settings.backend, "native");
    assert_eq!(app.browser_settings.save_error.as_deref(), Some("keep"));
}
