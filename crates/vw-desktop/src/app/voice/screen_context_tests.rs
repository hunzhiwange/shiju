use super::*;
#[test]
fn voice_task_17_screen_encodes_bounded_png_and_rejects_untrusted_wire() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use vw_api_voice::*;
    // High-entropy pixels exercise actual resizing/encoding rather than a tiny fixture.
    let mut value = 123u32;
    let rgba: Vec<u8> = (0..1024 * 1024 * 4)
        .map(|_| {
            value ^= value << 13;
            value ^= value >> 17;
            value ^= value << 5;
            value as u8
        })
        .collect();
    let png = encode(1024, 1024, rgba).unwrap();
    assert!(png.len() <= MAX_SCREEN_BYTES);
    let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap();
    assert!(decoded.width() <= MAX_SCREEN_EDGE && decoded.height() <= MAX_SCREEN_EDGE);
    let payload = ScreenPayload { png: STANDARD.encode(png) };
    assert!(!format!("{payload:?}").contains(&payload.png));
    let wire = serde_json::to_vec(&VoiceFrame {
        voice_session_id: "screen-current".to_owned().try_into().unwrap(),
        generation: VoiceCounter::FIRST,
        seq: VoiceCounter::FIRST,
        event: VoiceEvent::ScreenShare(payload),
    })
    .unwrap();
    assert!(wire.len() <= MAX_FRAME_BYTES);
    assert!(matches!(VoiceFrame::parse(&wire).unwrap().event, VoiceEvent::ScreenShare(_)));
    let mut value: serde_json::Value = serde_json::from_slice(&wire).unwrap();
    value["payload"]["png"] = serde_json::json!("https://example.invalid/screen.png");
    assert!(VoiceFrame::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    value["payload"]["png"] = serde_json::json!("A".repeat(MAX_FRAME_BYTES));
    assert!(VoiceFrame::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    assert!(encode(1025, 1, vec![0; 4100]).is_err());
}
