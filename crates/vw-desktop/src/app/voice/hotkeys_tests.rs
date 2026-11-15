use super::*;
use std::time::{Duration, Instant};
#[test]
fn voice_task_15_local_preferences_roundtrip_and_cancel_do_not_change_gateway() {
    use super::super::preferences::{Edit, Preferences};
    let config = config_voice::VoiceConfig::default();
    let local = Preferences::default();
    assert!(!local.hotkeys.enabled);
    let mut edit = Edit::new(config, local.clone());
    edit.local.hotkeys = Config {
        enabled: cfg!(target_os = "macos"),
        dictation: Some(Shortcut { key: 0, modifiers: CONTROL | ALT }),
        chat: Some(Shortcut { key: 8, modifiers: CONTROL | ALT }),
    };
    assert_eq!(edit.patch().unwrap(), serde_json::json!({}));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.json");
    edit.local.write(&path).unwrap();
    assert_eq!(Preferences::read(&path).unwrap(), edit.local);
    assert_eq!(edit.cancel().1, local);
}
#[test]
fn voice_task_15_local_disable_preserves_other_preferences_without_gateway() {
    use super::super::preferences::Preferences;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.json");
    let mut prefs = Preferences {
        microphone: Some("device-a".into()),
        retain_recordings: true,
        hotkeys: Config {
            enabled: cfg!(target_os = "macos"),
            dictation: Some(Shortcut { key: 0, modifiers: CONTROL | ALT }),
            chat: Some(Shortcut { key: 8, modifiers: CONTROL | ALT }),
        },
        ..Default::default()
    };
    prefs.write(&path).unwrap();
    Preferences::disable_hotkeys_at(&path).unwrap();
    prefs.hotkeys.enabled = false;
    assert_eq!(Preferences::read(&path).unwrap(), prefs);
}

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
