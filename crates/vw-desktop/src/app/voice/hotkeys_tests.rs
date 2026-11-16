use super::*;

#[test]
fn voice_task_15_recording_chords_stays_in_edit_until_save_and_cancel_discards() {
    use super::super::preferences::{Edit, Preferences};
    use crate::app::{
        App,
        components::system_settings::SystemTab,
        message::settings::voice::{self, Event},
    };
    let mut app = App::new().0;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Voice;
    app.voice.settings.edit = Some(Edit::new(Default::default(), Preferences::default()));
    let _ = voice::update(&mut app, Event::RecordHotkey(Some(false)));
    let shortcut = Shortcut { key: 0, modifiers: CONTROL | ALT };
    let _ = voice::update(&mut app, Event::RecordedHotkey(shortcut));
    assert_eq!(app.voice.settings.edit.as_ref().unwrap().local.hotkeys.dictation, Some(shortcut));
    assert!(!app.voice.settings.edit.as_ref().unwrap().local.hotkeys.enabled);
    assert_eq!(app.voice.settings.recording_hotkey, None);
    let _ = voice::update(&mut app, Event::RecordHotkey(Some(true)));
    let _ = voice::update(&mut app, Event::RecordHotkey(None));
    assert_eq!(app.voice.settings.edit.as_ref().unwrap().local.hotkeys.chat, None);
    let _ = voice::update(&mut app, Event::Cancel);
    assert!(app.voice.settings.edit.is_none());
}
