use crate::voice::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

fn frame(kind: &str, payload: Value) -> Value {
    json!({"type":kind,"voice_session_id":"voice_demo","generation":1,"seq":1,"payload":payload})
}
fn parse(value: Value) -> Result<VoiceFrame, &'static str> {
    VoiceFrame::parse(&serde_json::to_vec(&value).unwrap())
}
fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
    value: &T,
) {
    assert_eq!(&serde_json::from_slice::<T>(&serde_json::to_vec(value).unwrap()).unwrap(), value);
}

#[test]
fn voice_task_02_frame_rejects_oversized_audio() {
    let json = json!({"audio":STANDARD.encode(vec![0;4802]),"samples_per_channel":2401});
    assert!(
        serde_json::from_value::<AudioPayload>(json.clone()).is_err(),
        "oversized PCM frame must be rejected"
    );
    assert!(parse(frame("audio.append", json.clone())).is_err());
    let mut delta = json;
    delta["response_id"] = json!("response1");
    delta["item_id"] = json!("item1");
    delta["content_index"] = json!(0);
    assert!(parse(frame("audio.delta", delta)).is_err());
}

#[test]
fn voice_task_02_audio_checks_base64_samples_and_format() {
    for (bytes, samples) in [(vec![0; 2], 1), (vec![0; 4800], 2400)] {
        let value = frame(
            "audio.append",
            json!({"audio":STANDARD.encode(bytes),"samples_per_channel":samples}),
        );
        roundtrip(&parse(value).unwrap());
    }
    for payload in [
        json!({"audio":"","samples_per_channel":0}),
        json!({"audio":"!!!!","samples_per_channel":1}),
        json!({"audio":"AAA=","samples_per_channel":2}),
        json!({"audio":"AA==","samples_per_channel":1}),
        json!({"audio":"AAB=","samples_per_channel":1}),
        json!({"audio":"AAA","samples_per_channel":1}),
    ] {
        assert!(parse(frame("audio.append", payload)).is_err());
    }
    for format in [
        json!({"encoding":"wav","sample_rate":24000,"channels":1}),
        json!({"encoding":"pcm_s16le","sample_rate":16000,"channels":1}),
        json!({"encoding":"pcm_s16le","sample_rate":24000,"channels":2}),
    ] {
        assert!(serde_json::from_value::<AudioFormat>(format).is_err());
    }
    roundtrip(&AudioFormat::default());
}

#[test]
fn voice_task_02_requests_reject_unknown_fields_ids_and_counters() {
    let request = json!({"request_id":"dict_demo","chat_session_id":"chat_demo","generation":1,"provider_id":"voice_primary"});
    roundtrip(&serde_json::from_value::<TranscriptionRequest>(request.clone()).unwrap());
    for id in ["".to_owned(), "x".repeat(129), "中文".into(), "a\nb".into()] {
        let mut invalid = request.clone();
        invalid["request_id"] = json!(id);
        assert!(serde_json::from_value::<TranscriptionRequest>(invalid).is_err());
    }
    for counter in [json!(0), json!(-1), json!(1.5), json!(MAX_SAFE_INTEGER + 1)] {
        let mut invalid = request.clone();
        invalid["generation"] = counter;
        assert!(serde_json::from_value::<TranscriptionRequest>(invalid).is_err());
    }
    for key in ["api_key", "service_url", "device_id"] {
        let mut invalid = request.clone();
        invalid[key] = json!("synthetic");
        assert!(serde_json::from_value::<TranscriptionRequest>(invalid).is_err());
    }
    assert_eq!(VoiceCounter::FIRST.checked_next().unwrap().get(), 2);
    assert!(VoiceCounter::try_from(MAX_SAFE_INTEGER).unwrap().checked_next().is_err());
    let mut invalid = frame("session.stop", json!({}));
    invalid["unknown"] = json!(true);
    assert!(parse(invalid).is_err());
    assert!(parse(frame("input.commit", json!({"unknown":1}))).is_err());
    assert!(parse(frame("unknown", json!({}))).is_err());
}

#[test]
fn voice_task_02_transcript_delta_and_final_are_distinct() {
    let delta = parse(frame(
        "transcript.delta",
        json!({"role":"user","item_id":"item1","content_index":0,"delta":"你好"}),
    ))
    .unwrap();
    let final_text = parse(frame(
        "transcript.final",
        json!({"role":"user","item_id":"item1","content_index":0,"text":"你好。"}),
    ))
    .unwrap();
    assert!(matches!(delta.event, VoiceEvent::TranscriptDelta { .. }));
    assert!(matches!(final_text.event, VoiceEvent::TranscriptFinal { .. }));
    roundtrip(&delta);
    roundtrip(&final_text);
    assert!(
        parse(frame(
            "transcript.final",
            json!({"role":"user","item_id":"item1","content_index":0,"delta":"incomplete"})
        ))
        .is_err()
    );
    assert!(
        parse(frame(
            "transcript.final",
            json!({"role":"system","item_id":"item1","content_index":0,"text":"x"})
        ))
        .is_err()
    );
    assert!(
        parse(frame(
            "transcript.final",
            json!({"role":"user","item_id":"item1","content_index":0,"text":"x".repeat(65537)})
        ))
        .is_err()
    );
    assert!(VoiceFrame::parse(&vec![b' '; MAX_FRAME_BYTES + 1]).is_err());
}

#[test]
fn voice_task_02_control_and_response_events_roundtrip() {
    let formats = json!({"encoding":"pcm_s16le","sample_rate":24000,"channels":1});
    let cases = [
        ("session.ready", json!({"format":formats,"voice":"marin"})),
        ("input.commit", json!({})),
        ("input.mute", json!({"muted":true})),
        ("session.stop", json!({})),
        (
            "response.interrupt",
            json!({"response_id":"r1","item_id":null,"content_index":null,"audio_end_ms":null}),
        ),
        (
            "playback.progress",
            json!({"response_id":"r1","item_id":"i1","content_index":0,"audio_end_ms":100}),
        ),
        (
            "generation.changed",
            json!({"reason":"vad","response_id":"r1","item_id":"i1","content_index":0}),
        ),
        (
            "audio.delta",
            json!({"response_id":"r1","item_id":"i1","content_index":0,"audio":"AAA=","samples_per_channel":1}),
        ),
        ("audio.done", json!({"response_id":"r1","item_id":"i1","content_index":0})),
        ("response.started", json!({"response_id":"r1"})),
        ("response.done", json!({"response_id":"r1","status":"cancelled"})),
        ("handoff.status", json!({"call_id":"c1","status":"awaiting_approval","text":"等待审批"})),
        ("session.closed", json!({"outcome":"ended","reason":"user_stop"})),
    ];
    for (kind, payload) in cases {
        roundtrip(&parse(frame(kind, payload)).unwrap());
    }
    assert!(
        parse(frame("session.ready", json!({"format":AudioFormat::default(),"voice":"cove"})))
            .is_err()
    );
    assert!(
        parse(frame(
            "response.interrupt",
            json!({"response_id":"r1","item_id":"i1","content_index":null,"audio_end_ms":10})
        ))
        .is_err()
    );
}

#[test]
fn voice_task_02_http_responses_and_safe_errors_roundtrip() {
    let response:TranscriptionResponse=serde_json::from_value(json!({"request_id":"d1","chat_session_id":"chat1","generation":1,"text":"完成","detected_languages":["de"]})).unwrap();
    roundtrip(&response);
    let start:VoiceSessionStart=serde_json::from_value(json!({"request_id":"s1","chat_session_id":"chat1","generation":1,"provider_id":"primary","turn_mode":"manual"})).unwrap();
    roundtrip(&start);
    let started:VoiceSessionStarted=serde_json::from_value(json!({"request_id":"s1","chat_session_id":"chat1","generation":1,"voice_session_id":"voice1","stream_path":"/api/voice/sessions/voice1/stream","format":AudioFormat::default(),"voice":"marin","expires_in_ms":1800000})).unwrap();
    roundtrip(&started);
    use VoiceErrorCode::*;
    for (code, status, retryable) in [
        (InvalidRequest, 400, false),
        (Unauthorized, 401, false),
        (Forbidden, 403, false),
        (NotFound, 404, false),
        (Conflict, 409, false),
        (UnsupportedMedia, 415, false),
        (PayloadTooLarge, 413, false),
        (UnsupportedCapability, 422, false),
        (ProviderAuth, 502, false),
        (RateLimited, 429, true),
        (ProviderUnavailable, 502, true),
        (Timeout, 504, true),
        (Cancelled, 409, false),
        (ProtocolError, 502, false),
        (Backpressure, 429, false),
    ] {
        let error = VoiceError::new(code, VoiceId::try_from("d1".to_string()).unwrap());
        assert_eq!(code.http_status(), status);
        assert_eq!(error.retryable, retryable);
        assert_eq!(error.message, code.message());
        roundtrip(&VoiceErrorResponse { error: error.clone() });
        roundtrip(&parse(frame("error", serde_json::to_value(error).unwrap())).unwrap());
    }
}
