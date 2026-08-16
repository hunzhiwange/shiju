use super::*;
use crate::app::message::{GeneralSettingsMessage, SettingsMessage};
use crate::app::state::GeneralSettingsTab;

#[test]
fn tab_selection_updates_general_settings_content() {
    let mut app = App::new().0;

    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TabSelected(
            GeneralSettingsTab::Editor,
        )),
    );

    assert_eq!(app.general_settings_tab, GeneralSettingsTab::Editor);
}

#[test]
fn timezone_selection_updates_state_and_persists_config() {
    let mut app = App::new().0;
    clear_last_persisted_timezone();

    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(
            "America/New_York".to_string(),
        )),
    );

    assert_eq!(app.chat_timezone, "America/New_York");
    assert_eq!(last_persisted_timezone().as_deref(), Some("America/New_York"));
    clear_last_persisted_timezone();
    let original = app.chat_timezone.clone();

    let _ = update(
        &mut app,
        SettingsMessage::General(GeneralSettingsMessage::TimezoneSelected(
            "Mars/Olympus_Mons".to_string(),
        )),
    );

    assert_eq!(app.chat_timezone, original);
    assert_eq!(last_persisted_timezone(), None);
}
