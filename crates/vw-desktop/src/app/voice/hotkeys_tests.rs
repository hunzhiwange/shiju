use super::*;
use std::time::{Duration, Instant};
#[test]
fn voice_task_15_hotkey_repeat_does_not_restart_capture() {
    let now = Instant::now();
    let mut state = Machine::default();
    assert_eq!(state.press(now, false), Some(Action::Start));
    assert_eq!(state.press(now + Duration::from_millis(50), true), None);
}
#[test]
fn voice_task_15_hotkey_double_tap_enters_hands_free() {
    let now = Instant::now();
    let mut state = Machine::default();
    assert_eq!(state.press(now, false), Some(Action::Start));
    assert_eq!(state.release(now + Duration::from_millis(60)), None);
    assert_eq!(state.press(now + Duration::from_millis(160), false), None);
    assert_eq!(state.release(now + Duration::from_millis(200)), None);
    assert!(state.active);
    assert_eq!(state.tick(now + Duration::from_millis(600)), None);
    assert!(state.active);
    assert_eq!(state.press(now + Duration::from_secs(1), false), Some(Action::Finish));
}
#[test]
fn voice_task_15_hold_release_finishes_and_late_release_is_ignored() {
    let now = Instant::now();
    let mut state = Machine::default();
    assert_eq!(state.press(now, false), Some(Action::Start));
    assert_eq!(state.release(now + Duration::from_secs(1)), Some(Action::Finish));
    assert_eq!(state.release(now + Duration::from_secs(2)), None);
    assert!(!state.active);
}
#[test]
fn voice_task_15_single_tap_waits_for_double_tap_then_finishes_once() {
    let now = Instant::now();
    let mut state = Machine::default();
    state.press(now, false);
    state.release(now + Duration::from_millis(50));
    assert_eq!(state.tick(now + Duration::from_millis(200)), None);
    assert_eq!(state.tick(now + Duration::from_millis(331)), Some(Action::Finish));
    assert_eq!(state.tick(now + Duration::from_secs(2)), None);
}
#[test]
fn voice_task_15_late_second_press_cannot_enter_hands_free() {
    let now = Instant::now();
    let mut state = Machine::default();
    state.press(now, false);
    state.release(now + Duration::from_millis(50));
    assert_eq!(state.press(now + Duration::from_secs(1), false), Some(Action::Finish));
    assert!(!state.active);
}
#[test]
fn voice_task_15_escape_and_safety_loss_cancel_without_transcription() {
    let now = Instant::now();
    for input in [Input::Escape, Input::Unsafe] {
        let mut state = Machine::default();
        state.press(now, false);
        assert_eq!(state.input(input, now), Some(Action::Cancel));
        assert_eq!(state.release(now), None);
        assert_eq!(state.tick(now + Duration::from_secs(1)), None);
    }
}
#[test]
fn voice_task_15_chat_shortcut_ignores_repeat_and_active_dictation() {
    let now = Instant::now();
    let mut state = Machine::default();
    assert_eq!(state.input(Input::Chat(true), now), None);
    assert_eq!(state.input(Input::Chat(false), now), Some(Action::Chat));
    state.press(now, false);
    assert_eq!(state.input(Input::Chat(false), now), None);
}
#[test]
fn voice_task_15_local_preferences_roundtrip_and_cancel_do_not_change_gateway() {
    use super::super::preferences::{Edit, Preferences};
    let config = vw_config_types::voice::VoiceConfig::default();
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
fn voice_task_15_fn_plain_duplicate_and_reserved_chords_are_rejected() {
    assert!(Shortcut { key: 63, modifiers: 0 }.validate().is_err());
    assert!(Shortcut { key: 0, modifiers: SHIFT }.validate().is_err());
    assert!(Shortcut { key: 49, modifiers: COMMAND | SHIFT }.validate().is_err());
    assert!(Shortcut { key: 0, modifiers: CONTROL | ALT }.validate().is_ok());
    let shortcut = Some(Shortcut { key: 0, modifiers: CONTROL | ALT });
    assert!(Config { enabled: false, dictation: shortcut, chat: shortcut }.validate().is_err());
}
#[test]
fn voice_task_15_disable_preserves_chords_and_unregisters_machine_ownership() {
    let now = Instant::now();
    let mut runtime = Runtime::default();
    runtime.machine.press(now, false);
    runtime.stop_session();
    runtime.reload();
    assert!(runtime.blocked);
    assert!(runtime.poll().is_empty());
    assert!(!runtime.machine.active);
    assert_eq!(runtime.machine.release(now), None);
    runtime.resume_after_save();
    assert!(!runtime.blocked);
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
