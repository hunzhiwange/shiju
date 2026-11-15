use super::{local::*, permissions::PermissionStatus};

fn capability() -> Capabilities {
    Capabilities {
        language: "zh-CN".into(),
        supports_on_device: true,
        available: true,
        permission: PermissionStatus::Granted,
        voices: vec![],
    }
}
#[test]
fn local_voice_offline_gate_rejects_network_only_recognizer() {
    let mut state = capability();
    state.supports_on_device = false;
    assert_eq!(super::local::recognition_gate(&state), Err(LocalSpeechError::OfflineUnavailable));
    state.supports_on_device = true;
    for status in [
        PermissionStatus::Unknown,
        PermissionStatus::NotDetermined,
        PermissionStatus::Denied,
        PermissionStatus::Restricted,
        PermissionStatus::Unsupported,
    ] {
        state.permission = status;
        assert_eq!(
            super::local::recognition_gate(&state),
            Err(LocalSpeechError::Permission(status))
        );
    }
    state.permission = PermissionStatus::Granted;
    state.available = false;
    assert_eq!(super::local::recognition_gate(&state), Err(LocalSpeechError::Unavailable));
}
#[test]
fn local_voice_cancel_rejects_late_result() {
    let cancel = Cancellation::default();
    let other_thread = cancel.clone();
    std::thread::spawn(move || other_thread.cancel()).join().unwrap();
    assert_eq!(cancel.complete(Ok(123)), Err(LocalSpeechError::Cancelled));
    assert_eq!(
        transcribe_pcm(&[2; 10], Some("zh-CN"), &cancel).unwrap_err(),
        LocalSpeechError::Cancelled
    );
    assert_eq!(synthesize("测试", None, &cancel).unwrap_err(), LocalSpeechError::Cancelled);
}
#[test]
fn local_voice_rejects_invalid_inputs_before_system_access() {
    let cancel = Cancellation::default();
    assert_eq!(transcribe_pcm(&[], None, &cancel).unwrap_err(), LocalSpeechError::InvalidAudio);
    assert_eq!(
        transcribe_pcm(&vec![0; 24_000 * 60 + 1], None, &cancel).unwrap_err(),
        LocalSpeechError::InvalidAudio
    );
    for text in ["".into(), " ".into(), "x".repeat(MAX_TEXT_CHARS + 1), "a\0b".into()] {
        assert_eq!(synthesize(&text, None, &cancel).unwrap_err(), LocalSpeechError::InvalidText);
    }
    assert_eq!(super::local::language(Some("zh_CN")).unwrap(), Some("zh-CN".into()));
    assert!(super::local::language(Some("zh-CN; upload")).is_err());
    assert!(super::local::language(Some("-zh")).is_err());
}

#[test]
fn local_voice_pcm_resampling_is_bounded() {
    let mut output = super::local::PcmOutput::default();
    for _ in 0..44_100 {
        output.push(0.5, 44_100).unwrap();
    }
    assert_eq!(output.pcm.len(), 24_000);
    assert!(output.pcm.iter().all(|v| *v == 16384));
    assert_eq!(output.push(0.0, 48_000), Err(LocalSpeechError::InvalidAudio));
    let mut output = super::local::PcmOutput::default();
    output.pcm.resize(SAMPLE_RATE as usize * MAX_SYNTHESIS_SECONDS, 0);
    assert_eq!(output.push(1.0, 24_000), Err(LocalSpeechError::Limit));
}
#[test]
fn local_voice_debug_omits_payloads() {
    let result = Transcript { language: "zh-CN".into(), text: "private transcript fixture".into() };
    assert!(!format!("{result:?}").contains("private"));
    assert!(!format!("{:?}", SpeechAudio { pcm: vec![12345, -23456] }).contains("12345"));
}
#[cfg(target_os = "macos")]
#[test]
fn local_voice_native_request_is_always_on_device() {
    assert!(super::local::offline_request_probe().unwrap());
}
#[cfg(target_os = "macos")]
#[test]
fn local_voice_native_query_and_permission_guard() {
    let capability = capabilities(Some("zh-CN")).unwrap();
    println!(
        "zh-CN offline={} available={} permission={:?} voices={}",
        capability.supports_on_device,
        capability.available,
        capability.permission,
        capability.voices.len()
    );
    if capability.supports_on_device && capability.permission != PermissionStatus::Granted {
        let result = transcribe_pcm(&[0; 24_000], Some("zh-CN"), &Cancellation::default());
        assert_eq!(result.unwrap_err(), LocalSpeechError::Permission(capability.permission));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn local_voice_native_rejects_uninstalled_voice() {
    assert_eq!(
        synthesize("test", Some("third.party.online.voice"), &Cancellation::default()).unwrap_err(),
        LocalSpeechError::VoiceUnavailable
    );
    assert!(!voices().unwrap().is_empty());
}
