use super::*;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, sleep};

#[test]
fn extra_builds_json_map_for_empty_and_multiple_pairs() {
    let empty = extra([]);
    assert!(empty.is_empty());

    let map = extra([("request_id", json!("q1")), ("count", json!(2))]);
    assert_eq!(map.get("request_id"), Some(&json!("q1")));
    assert_eq!(map.get("count"), Some(&json!(2)));
}

fn question(label: &str) -> Info {
    Info {
        question: format!("Pick {label}?"),
        header: "Choice".to_string(),
        options: vec![OptionInfo {
            label: label.to_string(),
            description: "A choice".to_string(),
            preview: None,
        }],
        multiple: Some(false),
        custom: Some(true),
    }
}

async fn pending_request(session_id: &str) -> Request {
    loop {
        if let Some(req) = list().into_iter().find(|r| r.session_id == session_id) {
            break req;
        }
        sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn ask_lists_pending_and_reply_returns_answers() {
    let session_id = format!("session-{}", uuid::Uuid::new_v4());
    let handle = tokio::spawn({
        let session_id = session_id.clone();
        async move {
            ask(AskInput {
                session_id,
                questions: vec![question("A")],
                tool: Some(ToolMeta { message_id: "m1".to_string(), call_id: "c1".to_string() }),
            })
            .await
        }
    });

    let request = pending_request(&session_id).await;
    assert_eq!(request.questions.len(), 1);
    assert!(request.tool.is_some());

    reply(ReplyInput {
        request_id: request.id.clone(),
        answers: vec![vec!["A".to_string(), "custom".to_string()]],
    });
    reply(ReplyInput { request_id: request.id, answers: vec![vec!["late".to_string()]] });

    let answers = handle.await.expect("task should join").expect("ask should resolve");
    assert_eq!(answers, vec![vec!["A".to_string(), "custom".to_string()]]);
    assert!(!list().iter().any(|r| r.session_id == session_id));
}

#[tokio::test]
async fn ask_accepts_empty_questions_and_empty_answers() {
    let session_id = format!("session-{}", uuid::Uuid::new_v4());
    let handle = tokio::spawn({
        let session_id = session_id.clone();
        async move { ask(AskInput { session_id, questions: vec![], tool: None }).await }
    });

    let request = pending_request(&session_id).await;
    assert!(request.questions.is_empty());
    assert!(request.tool.is_none());

    reply(ReplyInput { request_id: request.id, answers: vec![] });

    let answers = handle.await.expect("task should join").expect("ask should resolve");
    assert!(answers.is_empty());
}

#[tokio::test]
async fn ask_and_reply_publish_request_and_answer_payloads() {
    let asked_events = Arc::new(Mutex::new(Vec::new()));
    let asked_slot = Arc::clone(&asked_events);
    let unsubscribe_asked = bus::subscribe(event::ASKED, move |payload| {
        asked_slot.lock().unwrap_or_else(|e| e.into_inner()).push(payload);
    });
    let replied_events = Arc::new(Mutex::new(Vec::new()));
    let replied_slot = Arc::clone(&replied_events);
    let unsubscribe_replied = bus::subscribe(event::REPLIED, move |payload| {
        replied_slot.lock().unwrap_or_else(|e| e.into_inner()).push(payload);
    });

    let session_id = format!("session-events-{}", uuid::Uuid::new_v4());
    let handle = tokio::spawn({
        let session_id = session_id.clone();
        async move {
            ask(AskInput {
                session_id,
                questions: vec![question("Publish")],
                tool: Some(ToolMeta {
                    message_id: "message-event".to_string(),
                    call_id: "call-event".to_string(),
                }),
            })
            .await
        }
    });

    let request = pending_request(&session_id).await;
    {
        let events = asked_events.lock().unwrap_or_else(|e| e.into_inner());
        let payload = events
            .iter()
            .find(|payload| payload["properties"]["id"] == request.id)
            .expect("asked event should contain the pending request");
        assert_eq!(payload["type"], "question.asked");
        assert_eq!(payload["properties"]["sessionID"], session_id);
        assert_eq!(payload["properties"]["questions"][0]["question"], "Pick Publish?");
        assert_eq!(payload["properties"]["tool"]["messageID"], "message-event");
        assert_eq!(payload["properties"]["tool"]["callID"], "call-event");
    }

    let answers = vec![vec!["Publish".to_string(), "custom answer".to_string()]];
    reply(ReplyInput { request_id: request.id.clone(), answers: answers.clone() });

    assert_eq!(handle.await.expect("task should join").expect("ask should resolve"), answers);
    {
        let events = replied_events.lock().unwrap_or_else(|e| e.into_inner());
        let payload = events
            .iter()
            .find(|payload| payload["properties"]["requestID"] == request.id)
            .expect("replied event should contain the completed request");
        assert_eq!(payload["type"], "question.replied");
        assert_eq!(payload["properties"]["sessionID"], session_id);
        assert_eq!(payload["properties"]["answers"], json!([["Publish", "custom answer"]]));
    }

    unsubscribe_asked();
    unsubscribe_replied();
}

#[tokio::test]
async fn reject_publishes_session_and_request_payload() {
    let rejected_events = Arc::new(Mutex::new(Vec::new()));
    let rejected_slot = Arc::clone(&rejected_events);
    let unsubscribe = bus::subscribe(event::REJECTED, move |payload| {
        rejected_slot.lock().unwrap_or_else(|e| e.into_inner()).push(payload);
    });

    let session_id = format!("session-rejected-event-{}", uuid::Uuid::new_v4());
    let handle = tokio::spawn({
        let session_id = session_id.clone();
        async move {
            ask(AskInput { session_id, questions: vec![question("Reject")], tool: None }).await
        }
    });

    let request = pending_request(&session_id).await;
    reject(request.id.clone());

    let err = handle.await.expect("task should join").expect_err("ask should reject");
    assert!(matches!(err, Error::Rejected(_)));
    {
        let events = rejected_events.lock().unwrap_or_else(|e| e.into_inner());
        let payload = events
            .iter()
            .find(|payload| payload["properties"]["requestID"] == request.id)
            .expect("rejected event should contain the rejected request");
        assert_eq!(payload["type"], "question.rejected");
        assert_eq!(payload["properties"]["sessionID"], session_id);
    }

    unsubscribe();
}

#[tokio::test]
async fn list_returns_independent_snapshots_for_multiple_pending_requests() {
    let first_session = format!("session-list-first-{}", uuid::Uuid::new_v4());
    let second_session = format!("session-list-second-{}", uuid::Uuid::new_v4());
    let first_handle = tokio::spawn({
        let session_id = first_session.clone();
        async move { ask(AskInput { session_id, questions: vec![question("First")], tool: None }).await }
    });
    let second_handle = tokio::spawn({
        let session_id = second_session.clone();
        async move {
            ask(AskInput { session_id, questions: vec![question("Second")], tool: None }).await
        }
    });

    let first_request = pending_request(&first_session).await;
    let second_request = pending_request(&second_session).await;
    let mut snapshot = list()
        .into_iter()
        .find(|request| request.id == first_request.id)
        .expect("first request should be listed");
    assert!(list().iter().any(|request| request.id == second_request.id));

    snapshot.session_id.clear();
    snapshot.questions.clear();
    let fresh_snapshot = list()
        .into_iter()
        .find(|request| request.id == first_request.id)
        .expect("mutating a snapshot should not remove the pending request");
    assert_eq!(fresh_snapshot.session_id, first_session);
    assert_eq!(fresh_snapshot.questions.len(), 1);
    assert_eq!(fresh_snapshot.questions[0].question, "Pick First?");

    reply(ReplyInput { request_id: first_request.id, answers: vec![vec!["First".to_string()]] });
    reject(second_request.id);
    assert_eq!(
        first_handle.await.expect("first task should join").expect("first ask should resolve"),
        vec![vec!["First".to_string()]]
    );
    assert!(matches!(
        second_handle
            .await
            .expect("second task should join")
            .expect_err("second ask should reject"),
        Error::Rejected(_)
    ));
}

#[tokio::test]
async fn reply_and_reject_cleanup_when_ask_waiters_are_aborted() {
    let reply_session = format!("session-aborted-reply-{}", uuid::Uuid::new_v4());
    let reject_session = format!("session-aborted-reject-{}", uuid::Uuid::new_v4());
    let reply_handle = tokio::spawn({
        let session_id = reply_session.clone();
        async move { ask(AskInput { session_id, questions: vec![question("Reply")], tool: None }).await }
    });
    let reject_handle = tokio::spawn({
        let session_id = reject_session.clone();
        async move {
            ask(AskInput { session_id, questions: vec![question("Reject")], tool: None }).await
        }
    });

    let reply_request = pending_request(&reply_session).await;
    let reject_request = pending_request(&reject_session).await;
    reply_handle.abort();
    reject_handle.abort();
    assert!(reply_handle.await.expect_err("reply waiter should be cancelled").is_cancelled());
    assert!(reject_handle.await.expect_err("reject waiter should be cancelled").is_cancelled());

    reply(ReplyInput {
        request_id: reply_request.id.clone(),
        answers: vec![vec!["ignored by cancelled waiter".to_string()]],
    });
    reject(reject_request.id.clone());

    let pending = list();
    assert!(!pending.iter().any(|request| request.id == reply_request.id));
    assert!(!pending.iter().any(|request| request.id == reject_request.id));
}

#[tokio::test]
async fn reject_resolves_ask_with_rejected_error_and_unknowns_are_ignored() {
    reply(ReplyInput { request_id: "missing".to_string(), answers: vec![] });
    reject("missing");

    let session_id = format!("session-{}", uuid::Uuid::new_v4());
    let handle = tokio::spawn({
        let session_id = session_id.clone();
        async move { ask(AskInput { session_id, questions: vec![question("B")], tool: None }).await }
    });

    let request = pending_request(&session_id).await;

    reject(request.id.clone());
    reject(request.id);

    let err = handle.await.expect("task should join").expect_err("ask should reject");
    assert!(matches!(err, Error::Rejected(_)));
    assert!(err.to_string().contains("dismissed"));
    assert!(!list().iter().any(|r| r.session_id == session_id));
}

#[tokio::test]
async fn ask_returns_rejected_when_pending_sender_is_dropped() {
    let session_id = format!("session-{}", uuid::Uuid::new_v4());
    let handle = tokio::spawn({
        let session_id = session_id.clone();
        async move { ask(AskInput { session_id, questions: vec![question("C")], tool: None }).await }
    });

    let request = pending_request(&session_id).await;
    {
        let mut lock = STATE.lock().unwrap_or_else(|e| e.into_inner());
        lock.pending.remove(&request.id);
    }

    let err = handle.await.expect("task should join").expect_err("ask should reject");
    assert!(matches!(err, Error::Rejected(_)));
}

#[test]
fn poisoned_state_lock_recovers_for_public_operations() {
    let _ = std::thread::spawn(|| {
        let _guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
        panic!("poison question state");
    })
    .join();

    let _ = list();
    reply(ReplyInput { request_id: "poison-missing".to_string(), answers: vec![] });
    reject("poison-missing");
}

#[test]
fn question_error_display_conversion_and_events_are_stable() {
    let rejected = Error::Rejected(RejectedError);
    assert_eq!(rejected.to_string(), "The user dismissed this question");

    let id_error = id::Error::Random("random failed".to_string());
    let err = Error::from(id_error);
    assert!(matches!(err, Error::Id(_)));
    assert_eq!(err.to_string(), "random failed");

    assert_eq!(event::ASKED.r#type, "question.asked");
    assert_eq!(event::REPLIED.r#type, "question.replied");
    assert_eq!(event::REJECTED.r#type, "question.rejected");
}
