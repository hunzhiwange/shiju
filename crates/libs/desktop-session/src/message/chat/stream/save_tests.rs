use super::{SaveOutcome, SessionMessageDeltaSaveError, StreamSaveState};
use session_ui_types::{ChatMessage, ChatRole, SessionMessageRecord};

fn message(role: ChatRole, content: &str) -> ChatMessage {
    ChatMessage {
        role,
        content: content.into(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
fn history_conflict_resumes_when_gateway_confirms_provisional_ids() {
    let generation = 8;
    let user = message(ChatRole::User, "question");
    let partial_reply = message(ChatRole::Assistant, "partial");
    let chat = [user.clone(), message(ChatRole::Assistant, "partial reply")];
    let ids = [Some("user-id".into()), Some("assistant-id".into())];
    let conflict = Err(SessionMessageDeltaSaveError::HistoryConflict("conflict".into()));

    for confirm_before_ack in [false, true] {
        let mut save = StreamSaveState { seq_base: 20, ..Default::default() };
        for (seq_no, message) in [(20, user.clone()), (21, partial_reply.clone())] {
            save.mark(
                SessionMessageRecord { seq_no, message_id: None, message },
                1,
                Some(generation),
            );
        }
        let (_, revision, delta) = save.prepare("session", false, 2).unwrap();
        let confirm = |save: &mut StreamSaveState| {
            save.confirm_terminal_ids(
                generation,
                &chat,
                &ids,
                Some("user-id"),
                Some("assistant-id"),
            );
        };
        if confirm_before_ack {
            confirm(&mut save);
        }
        let (outcome, _) = save.acknowledge(
            generation,
            revision,
            delta.updated_ms,
            &conflict,
            1,
            Some(generation),
            |_| None,
        );
        if confirm_before_ack {
            assert_eq!(outcome, SaveOutcome::Retry);
        } else {
            assert_eq!(outcome, SaveOutcome::Stopped);
            assert!(save.prepare("session", false, 3).is_none());
            confirm(&mut save);
        }
        let (_, revision, retried) = save.prepare("session", false, 3).unwrap();
        assert_eq!(retried.upserts[0].message_id.as_deref(), Some("user-id"));
        assert_eq!(retried.upserts[1].message_id.as_deref(), Some("assistant-id"));
        assert_eq!(retried.upserts[1].message.content, "partial reply");

        let (outcome, _) = save.acknowledge(
            generation,
            revision,
            retried.updated_ms,
            &conflict,
            1,
            Some(generation),
            |_| None,
        );
        assert_eq!(outcome, SaveOutcome::Stopped);
    }
}
