use super::*;
#[test]
fn voice_task_13_voice_settings_cancel_preserves_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.json");
    let local = Preferences { microphone: Some("device-a".into()), ..Default::default() };
    local.write(&path).unwrap();
    let config = VoiceConfig::default();
    let mut edit = Edit::new(config.clone(), Preferences::read(&path).unwrap());
    edit.local.microphone = Some("device-b".into());
    edit.config.dictionary.push("识矩".into());
    edit.config.language = vw_config_types::voice::RecognitionLanguage::Zh;
    let (cancelled, cancelled_local) = edit.cancel();
    assert_eq!(cancelled, config);
    assert_eq!(cancelled_local, local);
    assert_eq!(Preferences::read(&path).unwrap(), local);
}
#[test]
fn voice_task_13_preferences_atomic_roundtrip_and_corruption() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("prefs.json");
    assert_eq!(Preferences::read(&path).unwrap(), Preferences::default());
    let local = Preferences { microphone: Some("设备中文".into()), ..Default::default() };
    local.write(&path).unwrap();
    assert_eq!(Preferences::read(&path).unwrap(), local);
    std::fs::write(&path, b"{broken").unwrap();
    assert!(Preferences::read(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
}
#[test]
fn voice_task_13_patch_only_changed_fields_and_validates_dictionary() {
    let config = VoiceConfig { provider_id: Some("openai".into()), ..Default::default() };
    let mut edit = Edit::new(config, Preferences::default());
    edit.config.provider_id = None;
    edit.config.voice = vw_config_types::voice::VoiceName::Cedar;
    assert_eq!(edit.patch().unwrap(), serde_json::json!({"provider_id":null,"voice":"cedar"}));
    edit.config.dictionary.push("<bad>".into());
    assert!(edit.patch().is_err());
}
