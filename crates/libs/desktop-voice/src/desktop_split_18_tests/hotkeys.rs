#![cfg(not(target_arch = "wasm32"))]
use crate::hotkeys::*;
#[test]
fn voice_task_15_local_preferences_roundtrip_and_cancel_do_not_change_gateway() {
    use crate::preferences::{Edit, Preferences};
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
    use crate::preferences::Preferences;
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
