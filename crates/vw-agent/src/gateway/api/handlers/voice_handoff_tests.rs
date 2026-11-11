use super::*;
fn id(value: &str) -> VoiceId {
    value.to_owned().try_into().unwrap()
}
fn call() -> HandoffCall {
    HandoffCall {
        call_id: id("call"),
        item_id: id("item"),
        response_id: id("response"),
        generation: VoiceCounter::FIRST,
        instruction: "整理项目待办".into(),
    }
}
#[test]
fn voice_task_12_handoff_duplicate_call_executes_once() {
    let mut admissions = Admissions::default();
    let first = admissions.admit(&call(), VoiceCounter::FIRST).unwrap();
    let duplicate = admissions.admit(&call(), VoiceCounter::FIRST).unwrap();
    assert_eq!(
        [first, duplicate].into_iter().filter(|admitted| *admitted).count(),
        1,
        "duplicate final/tool calls must admit exactly one session execution"
    );
}
#[test]
fn voice_task_12_handoff_preserves_approval_denial() {
    let mut outcome = Outcome::default();
    outcome.observe(&serde_json::json!({"type":"chat.tool","kind":"complete","status":"denied","payload":{"error":"User rejected permission"}}));
    let (status, text) = outcome.finish("模型声称已完成".into());
    assert_eq!(
        status,
        HandoffStatus::Failed,
        "approval denial must override the model's success claim"
    );
    assert!(text.contains("拒绝"));
}

#[test]
fn voice_task_12_stale_and_conflicting_calls_never_admit_work() {
    let mut admissions = Admissions::default();
    assert!(!admissions.admit(&call(), VoiceCounter::FIRST.checked_next().unwrap()).unwrap());
    assert!(admissions.admit(&call(), VoiceCounter::FIRST).unwrap());
    let mut conflict = call();
    conflict.instruction = "different work".into();
    assert_eq!(
        admissions.admit(&conflict, VoiceCounter::FIRST),
        Err(VoiceErrorCode::ProtocolError)
    );
}

#[test]
fn voice_task_12_failed_work_and_control_heavy_results_are_bounded() {
    let mut outcome = Outcome::default();
    outcome.observe(&serde_json::json!({"type":"chat.tool","status":"failed"}));
    assert_eq!(outcome.finish("success claim".into()).0, HandoffStatus::Failed);
    let (_, text) = Outcome::default().finish("中\n\u{0001}".repeat(20000));
    assert!(text.len() <= 16 * 1024);
    let frame = vw_api_types::voice::VoiceFrame {
        voice_session_id: id("voice"),
        generation: VoiceCounter::FIRST,
        seq: VoiceCounter::FIRST,
        event: vw_api_types::voice::VoiceEvent::HandoffStatus {
            call_id: id("call"),
            status: HandoffStatus::Completed,
            text,
        },
    };
    let wire = serde_json::to_vec(&frame).unwrap();
    assert!(vw_api_types::voice::VoiceFrame::parse(&wire).is_ok());
}
