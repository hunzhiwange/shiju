use super::*;
fn target() -> Target {
    Target { pid: 42, window: 17, label: "Fixture · 单窗口".into() }
}
#[test]
fn voice_task_17_screen_context_disabled_captures_nothing() {
    let mut state = State::default();
    let mut captured = 0;
    if let Some(ticket) = state.begin(target(), true) {
        let _ = ticket.capture(|_| {
            captured += 1;
            Ok(())
        });
    }
    assert_eq!(captured, 0, "disabled sharing must never enter the OS capture boundary");
}
#[test]
fn voice_task_17_screen_context_rejects_late_frame() {
    let mut state = State::default();
    state.set_enabled(true);
    let ticket = state.begin(target(), true).unwrap();
    let frame = ticket.capture(|_| Ok(vec![1, 2, 3])).unwrap();
    state.cancel();
    assert!(
        state.accept(&ticket, frame, true).is_none(),
        "cancelled/session-retired frame must never be sent"
    );
}

#[test]
fn voice_task_17_screen_denied_or_cancelled_enters_no_capture_boundary() {
    let mut state = State::default();
    state.set_enabled(true);
    let mut captures = 0;
    assert!(state.begin(target(), false).is_none());
    let ticket = state.begin(target(), true).unwrap();
    state.cancel();
    assert!(
        ticket
            .capture(|_| {
                captures += 1;
                Ok(())
            })
            .is_err()
    );
    assert_eq!(captures, 0);
}
#[test]
fn voice_task_17_screen_only_one_frame_for_current_ticket() {
    let mut state = State::default();
    state.set_enabled(true);
    let old = state.begin(target(), true).unwrap();
    let current = state.begin(target(), true).unwrap();
    assert!(state.accept(&old, vec![0], true).is_none());
    assert!(state.accept(&current, vec![1], false).is_none());
    assert_eq!(state.accept(&current, vec![1], true), Some(vec![1]));
    assert!(state.accept(&current, vec![1], true).is_none());
    state.set_enabled(false);
    assert!(!current.is_live());
}
#[test]
fn voice_task_17_screen_owner_drop_retires_inflight_worker() {
    let ticket = {
        let mut state = State::default();
        state.set_enabled(true);
        state.begin(target(), true).unwrap()
    };
    assert!(!ticket.is_live());
}
#[test]
fn voice_task_17_screen_encodes_bounded_png_and_rejects_untrusted_wire() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use vw_gateway_client::vw_api_types::voice::*;
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
