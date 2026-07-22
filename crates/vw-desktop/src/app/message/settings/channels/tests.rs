use super::*;
use crate::app::App;
use crate::app::message::settings::ChannelsMessage;

#[test]
fn channels_top_level_update_delegates_to_updates_module() {
    let mut app = App::new().0;

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::PanelToggled("telegram".to_string())),
    );

    assert!(app.channels_settings.expanded_panels.contains("telegram"));
}

#[test]
fn channels_top_level_update_ignores_non_channels_messages() {
    let mut app = App::new().0;
    app.channels_settings.cli = true;
    app.channels_settings.save_error = Some("existing error".to_string());
    let expanded_panels = app.channels_settings.expanded_panels.clone();

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.channels_settings.cli);
    assert_eq!(app.channels_settings.save_error.as_deref(), Some("existing error"));
    assert_eq!(app.channels_settings.expanded_panels, expanded_panels);
}
