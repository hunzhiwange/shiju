use app_session_content::turns::{build_turn_index, project};
use vw_api_external_session::{SessionBlock, SessionMessage, SessionProvider};

#[test]
fn indexes_replies_and_tool_steps_without_a_user_question() {
    let messages = [
        SessionMessage {
            role: "user".into(),
            turn_id: Some("context".into()),
            injected: true,
            blocks: vec![SessionBlock::text("Workspace instructions")],
            ..SessionMessage::default()
        },
        SessionMessage {
            role: "user".into(),
            turn_id: Some("reply".into()),
            injected: true,
            blocks: vec![SessionBlock::text("Environment context")],
            ..SessionMessage::default()
        },
        SessionMessage {
            role: "assistant".into(),
            turn_id: Some("reply".into()),
            blocks: vec![SessionBlock::text("Task completed.")],
            ..SessionMessage::default()
        },
        SessionMessage {
            role: "tool".into(),
            turn_id: Some("execution".into()),
            content: "Command output".into(),
            ..SessionMessage::default()
        },
    ];

    let turns = project(&messages, SessionProvider::Codex);
    let index = build_turn_index(&turns);

    assert_eq!(index.len(), 2);
    assert_eq!(index[0].turn_id, "reply");
    assert_eq!(index[0].last_message_index, 2);
    assert!(index[0].has_final_reply);
    assert!(index[0].question_preview.is_empty());
    assert_eq!(index[1].turn_id, "execution");
    assert_eq!(index[1].step_count, 1);
    assert!(!index[1].has_final_reply);
}
