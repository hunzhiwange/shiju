use super::blocks::assign_turn_ids;
use vw_api_external_session::SessionMessage;

#[test]
fn assign_turn_ids_skips_injected_and_preserves_native_ids() {
    let mut messages = vec![
        SessionMessage { role: "user".into(), content: "context".into(), injected: true, ..Default::default() },
        SessionMessage { role: "user".into(), content: "question".into(), ..Default::default() },
        SessionMessage { role: "assistant".into(), turn_id: Some("native".into()), ..Default::default() },
        SessionMessage { role: "user".into(), content: "next".into(), ..Default::default() },
    ];
    assign_turn_ids(&mut messages);
    assert_eq!(messages.iter().map(|m| m.turn_id.as_deref()).collect::<Vec<_>>(), vec![Some("t0"), Some("t1"), Some("native"), Some("t2")]);
}
