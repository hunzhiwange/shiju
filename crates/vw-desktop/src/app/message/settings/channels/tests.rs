use super::*;
use crate::app::App;
use crate::app::message::settings::ChannelsMessage;

#[test]
fn channels_top_level_update_delegates_to_updates_module() {
    let mut app = App::new().0;

    let _ = update(
        &mut app,
        SettingsMessage::Channels(ChannelsMessage::TabSelected(
            crate::app::state::ChannelsSettingsTab::Telegram,
        )),
    );

    assert_eq!(app.channels_settings.active_tab, crate::app::state::ChannelsSettingsTab::Telegram);
}

#[test]
fn channels_top_level_update_ignores_non_channels_messages() {
    let mut app = App::new().0;
    app.channels_settings.cli = true;
    app.channels_settings.save_error = Some("existing error".to_string());
    let active_tab = app.channels_settings.active_tab;

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.channels_settings.cli);
    assert_eq!(app.channels_settings.save_error.as_deref(), Some("existing error"));
    assert_eq!(app.channels_settings.active_tab, active_tab);
}
