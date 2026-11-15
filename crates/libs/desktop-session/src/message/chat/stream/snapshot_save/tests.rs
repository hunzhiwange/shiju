use session_ui_types::{ChatMessage as ModelChatMessage, ChatRole};
fn chat_message(role: ChatRole, content: &str) -> ModelChatMessage {
    ModelChatMessage {
        role,
        content: content.to_string(),
        model: None,
        think_timing: Vec::new(),
        processing_timing: None,
    }
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
// 同一会话保存进行中时只保留最新快照，防止旧写入在完成后覆盖新状态。
fn stream_session_saves_coalesce_to_latest_snapshot_while_in_flight() {
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    let coordinator = Arc::new(super::StreamSessionSaveCoordinator::default());
    let saved_contents = Arc::new(Mutex::new(Vec::new()));
    let other_session_saved = Arc::new(Mutex::new(false));
    let (first_save_started_tx, first_save_started_rx) = mpsc::channel();
    let (release_first_save_tx, release_first_save_rx) = mpsc::channel();

    let snapshot = |content: &str| session_ui_types::ChatSession {
        id: "session-1".to_string(),
        title: "Session".to_string(),
        messages: vec![chat_message(ChatRole::Assistant, content)],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };
    let first_snapshot = snapshot("first");
    let middle_snapshot = snapshot("middle");
    let latest_snapshot = snapshot("latest");
    let mut other_session = snapshot("other");
    other_session.id = "session-2".to_string();

    let first_coordinator = Arc::clone(&coordinator);
    let first_saved_contents = Arc::clone(&saved_contents);
    let first_save = std::thread::spawn(move || {
        first_coordinator.save(
            first_snapshot,
            Some("/project".to_string()),
            move |session, directory| {
                first_saved_contents
                    .lock()
                    .expect("saved contents lock")
                    .push((session.messages[0].content.clone(), directory.map(str::to_string)));
                if session.messages[0].content == "first" {
                    first_save_started_tx.send(()).expect("signal first save start");
                    release_first_save_rx
                        .recv_timeout(Duration::from_secs(10))
                        .expect("release first save");
                }
                Ok(Some(std::path::PathBuf::from(&session.id)))
            },
        );
    });

    first_save_started_rx.recv_timeout(Duration::from_secs(10)).expect("first save should start");

    coordinator.save(middle_snapshot, Some("/middle".to_string()), |_, _| {
        panic!("an in-flight save must use the active runner")
    });
    coordinator.save(latest_snapshot, Some("/latest".to_string()), |_, _| {
        panic!("an in-flight save must use the active runner")
    });
    let other_session_saved_by_callback = Arc::clone(&other_session_saved);
    coordinator.save(other_session, Some("/other".to_string()), move |session, directory| {
        assert_eq!(session.id, "session-2");
        assert_eq!(directory, Some("/other"));
        *other_session_saved_by_callback.lock().expect("other session saved lock") = true;
        Ok(Some(std::path::PathBuf::from(&session.id)))
    });
    assert!(*other_session_saved.lock().expect("other session saved lock"));

    release_first_save_tx.send(()).expect("release first save");
    first_save.join().expect("first save thread");

    assert_eq!(
        saved_contents.lock().expect("saved contents lock").as_slice(),
        [
            ("first".to_string(), Some("/project".to_string())),
            ("latest".to_string(), Some("/latest".to_string())),
        ]
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn stream_session_save_retains_latest_snapshot_after_failure() {
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::Duration;

    let coordinator = Arc::new(super::StreamSessionSaveCoordinator::default());
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let (first_save_started_tx, first_save_started_rx) = mpsc::channel();
    let (release_first_save_tx, release_first_save_rx) = mpsc::channel();
    let snapshot = |content: &str| session_ui_types::ChatSession {
        id: "retry-session".to_string(),
        title: "Session".to_string(),
        messages: vec![chat_message(ChatRole::Assistant, content)],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };

    let first_coordinator = Arc::clone(&coordinator);
    let first_attempts = Arc::clone(&attempts);
    let first = std::thread::spawn(move || {
        first_coordinator.save(snapshot("first"), None, move |session, _| {
            first_attempts.lock().expect("attempts lock").push(session.messages[0].content.clone());
            first_save_started_tx.send(()).expect("signal first save start");
            release_first_save_rx
                .recv_timeout(Duration::from_secs(10))
                .expect("release first save");
            Err("gateway request failed: 503 Service Unavailable".to_string())
        })
    });
    first_save_started_rx.recv_timeout(Duration::from_secs(10)).expect("first save should start");

    let queued = coordinator.save(snapshot("queued-latest"), None, |_, _| {
        panic!("an in-flight save must use the active runner")
    });
    assert_eq!(queued, super::StreamSessionSaveResult::Coalesced);
    release_first_save_tx.send(()).expect("release first save");
    assert!(matches!(
        first.join().expect("first save thread"),
        super::StreamSessionSaveResult::Deferred(_)
    ));
    assert_eq!(attempts.lock().expect("attempts lock").as_slice(), ["first"]);

    let retry_attempts = Arc::clone(&attempts);
    let retried = coordinator.save(snapshot("newest"), None, move |session, _| {
        retry_attempts.lock().expect("attempts lock").push(session.messages[0].content.clone());
        Ok(Some(std::path::PathBuf::from(&session.id)))
    });

    assert_eq!(retried, super::StreamSessionSaveResult::Committed);
    assert_eq!(attempts.lock().expect("attempts lock").as_slice(), ["first", "newest"]);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn stream_session_save_ack_requires_atomic_commit() {
    let coordinator = super::StreamSessionSaveCoordinator::default();
    let snapshot = session_ui_types::ChatSession {
        id: "ack-session".to_string(),
        title: "Session".to_string(),
        messages: vec![chat_message(ChatRole::Assistant, "answer")],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };

    let failed =
        coordinator.save(snapshot.clone(), None, |_, _| Err("database is locked".to_string()));
    assert!(matches!(&failed, super::StreamSessionSaveResult::Deferred(_)));
    assert!(matches!(super::save_notification(Some(failed)), super::SaveNotification::None));

    let committed = coordinator.save(snapshot, None, |_, _| Ok(None));
    assert_eq!(committed, super::StreamSessionSaveResult::Committed);
    assert!(matches!(
        super::save_notification(Some(committed)),
        super::SaveNotification::Ack
    ));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn stream_session_save_stops_retries_after_payload_too_large() {
    use std::sync::{Arc, Mutex};

    let coordinator = super::StreamSessionSaveCoordinator::default();
    let saved_count = Arc::new(Mutex::new(0));
    let first_count = Arc::clone(&saved_count);
    let snapshot = session_ui_types::ChatSession {
        id: "oversized-session".to_string(),
        title: "Session".to_string(),
        messages: vec![chat_message(ChatRole::Assistant, "answer")],
        message_ids: vec![None],
        calls: Vec::new(),
        steps: Vec::new(),
        created_ms: 1,
        updated_ms: 2,
    };

    let failed = coordinator.save(snapshot.clone(), None, move |_, _| {
        *first_count.lock().expect("save count lock") += 1;
        Err("gateway request failed: 413 Payload Too Large".to_string())
    });
    assert!(matches!(&failed, super::StreamSessionSaveResult::PermanentlyFailed(_)));
    assert!(matches!(
        super::save_notification(Some(failed)),
        super::SaveNotification::Failed(_)
    ));

    let second_count = Arc::clone(&saved_count);
    let blocked = coordinator.save(snapshot, None, move |_, _| {
        *second_count.lock().expect("save count lock") += 1;
        Ok(None)
    });

    assert_eq!(blocked, super::StreamSessionSaveResult::Blocked);
    assert!(matches!(super::save_notification(Some(blocked)), super::SaveNotification::None));
    assert_eq!(*saved_count.lock().expect("save count lock"), 1);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn session_history_conflict_is_reported_without_disabling_future_saves() {
    let coordinator = super::StreamSessionSaveCoordinator::default();
    let snapshot = session_ui_types::ChatSession {
        id: "history-conflict".into(),
        title: "Session".into(),
        messages: vec![chat_message(ChatRole::User, "next question")],
        message_ids: vec![None],
        calls: vec![],
        steps: vec![],
        created_ms: 1,
        updated_ms: 2,
    };
    let result = coordinator.save(snapshot.clone(), None, |_, _| {
        Err("409 Conflict: session history conflict at seq_no 0".into())
    });
    assert!(matches!(
        super::save_notification(Some(result)),
        super::SaveNotification::Failed(_)
    ));
    assert_eq!(
        coordinator.save(snapshot, None, |_, _| Ok(None)),
        super::StreamSessionSaveResult::Committed
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn session_history_conflict_keeps_a_newer_pending_snapshot() {
    let coordinator = super::StreamSessionSaveCoordinator::default();
    let snapshot = session_ui_types::ChatSession {
        id: "history-pending".into(),
        title: "Session".into(),
        messages: vec![chat_message(ChatRole::User, "question")],
        message_ids: vec![None],
        calls: vec![],
        steps: vec![],
        created_ms: 1,
        updated_ms: 1,
    };
    let mut attempts = Vec::new();
    let result = coordinator.save(snapshot, None, |session, _| {
        attempts.push(session.updated_ms);
        if session.updated_ms == 1 {
            let mut newer = session.clone();
            newer.updated_ms = 2;
            assert_eq!(
                coordinator.save(newer, None, |_, _| panic!("coalesced")),
                super::StreamSessionSaveResult::Coalesced
            );
            Err("session history conflict at seq_no 0".into())
        } else {
            Ok(None)
        }
    });
    assert_eq!(result, super::StreamSessionSaveResult::Committed);
    assert_eq!(attempts, vec![1, 2]);
}
