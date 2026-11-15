use super::*;
#[test]
fn update_ignores_non_acp_settings_messages() {
    let (mut app, _) = crate::app::App::new();
    app.acp_settings.loading = true;
    app.acp_settings.saving_agent = Some("codex".to_string());
    app.acp_settings.save_error = Some("unchanged".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.acp_settings.loading);
    assert_eq!(app.acp_settings.saving_agent.as_deref(), Some("codex"));
    assert_eq!(app.acp_settings.save_error.as_deref(), Some("unchanged"));
}
