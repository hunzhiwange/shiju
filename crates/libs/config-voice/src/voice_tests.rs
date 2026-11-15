use crate::*;
use serde_json::json;

#[test]
fn voice_task_02_config_defaults_disable_capture() {
    let config: VoiceConfig = serde_json::from_str("{}").unwrap();
    assert!(!config.dictation_enabled, "missing config must disable dictation");
    assert!(!config.realtime_enabled, "missing config must disable realtime capture");
    assert!(config.provider_id.is_none());
    assert_eq!(config.voice, VoiceName::Marin);
    assert_eq!(config.language.service_code(), None);
    assert_eq!(config.validate(), Ok(()));
}

#[test]
fn voice_task_02_config_rejects_unsupported_capabilities_and_secrets() {
    for value in [
        json!({"voice":"cove"}),
        json!({"language":"de"}),
        json!({"language":"en-US"}),
        json!({"provider":"private"}),
        json!({"transcription_model":"arbitrary"}),
        json!({"realtime_model":"arbitrary"}),
        json!({"api_key":"synthetic"}),
        json!({"device_id":"local"}),
        json!({"provider_id":""}),
    ] {
        assert!(serde_json::from_value::<VoiceConfig>(value).is_err());
    }
    for voice in
        ["alloy", "ash", "ballad", "coral", "echo", "sage", "shimmer", "verse", "marin", "cedar"]
    {
        assert!(serde_json::from_value::<VoiceConfig>(json!({"voice":voice})).is_ok());
    }
    for (language, code) in [
        ("auto", None),
        ("en", Some("en")),
        ("es", Some("es")),
        ("fr", Some("fr")),
        ("zh", Some("zh")),
    ] {
        let config: VoiceConfig = serde_json::from_value(json!({"language":language})).unwrap();
        assert_eq!(config.language.service_code(), code);
    }
}

#[test]
fn voice_task_02_dictionary_and_prompt_enforce_unicode_boundaries() {
    let valid = json!({"dictionary":vec!["字".repeat(80);100], "prompt":"文".repeat(2000)});
    assert!(serde_json::from_value::<VoiceConfig>(valid).is_ok());
    for invalid in [
        json!({"dictionary":vec!["word";101]}),
        json!({"dictionary":["字".repeat(81)]}),
        json!({"prompt":"文".repeat(2001)}),
    ] {
        assert!(serde_json::from_value::<VoiceConfig>(invalid).is_err());
    }
    for entry in ["", "  ", "a<b", "a>b", "a\rb", "a\nb"] {
        assert!(serde_json::from_value::<VoiceConfig>(json!({"dictionary":[entry]})).is_err());
    }
    let edited = VoiceConfig { dictionary: vec!["<bad>".into()], ..Default::default() };
    assert!(edited.validate().is_err());
}
