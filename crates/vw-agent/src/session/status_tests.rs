//! 验证会话忙闲状态存储、序列化及作用域事件发布行为。

use super::*;
use crate::app::agent::{bus, project::instance};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

#[test]
fn set_busy_then_idle_updates_state_map() {
    set("status-test-session", Info::Busy);
    assert!(matches!(get("status-test-session"), Info::Busy));
    assert!(list().contains_key("status-test-session"));

    set("status-test-session", Info::Idle);
    assert!(matches!(get("status-test-session"), Info::Idle));
    assert!(!list().contains_key("status-test-session"));
}

#[test]
fn retry_status_is_stored_listed_and_serialized() {
    let session_id = "status-retry-test-session";
    set(session_id, Info::Retry { attempt: 3, message: "rate limited".to_string(), next: 42 });

    let Info::Retry { attempt, message, next } = get(session_id) else {
        panic!("expected retry status");
    };
    assert_eq!(attempt, 3);
    assert_eq!(message, "rate limited");
    assert_eq!(next, 42);

    let listed = list();
    let json = serde_json::to_value(listed.get(session_id).expect("listed status")).unwrap();
    assert_eq!(json["type"], "retry");
    assert_eq!(json["attempt"], 3);

    set(session_id, Info::Idle);
}

#[test]
fn unknown_session_defaults_to_idle_without_inserting_state() {
    let session_id = "status-unknown-test-session";
    set(session_id, Info::Idle);

    assert!(matches!(get(session_id), Info::Idle));
    assert!(!list().contains_key(session_id));
}

#[test]
fn state_accessors_recover_from_poisoned_state_lock() {
    let poison_result = std::panic::catch_unwind(|| {
        let _guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
        panic!("poison session status state lock");
    });
    assert!(poison_result.is_err());

    let session_id = format!("status-poison-test-{}", uuid::Uuid::new_v4());
    set(&session_id, Info::Busy);
    assert!(matches!(get(&session_id), Info::Busy));
    assert!(list().contains_key(&session_id));

    set(&session_id, Info::Idle);
    assert!(matches!(get(&session_id), Info::Idle));
    assert!(!list().contains_key(&session_id));
}

#[test]
fn info_deserializes_all_tagged_status_variants() {
    let idle: Info = serde_json::from_value(serde_json::json!({ "type": "idle" })).unwrap();
    assert!(matches!(idle, Info::Idle));

    let busy: Info = serde_json::from_value(serde_json::json!({ "type": "busy" })).unwrap();
    assert!(matches!(busy, Info::Busy));

    let retry: Info = serde_json::from_value(serde_json::json!({
        "type": "retry",
        "attempt": 4,
        "message": "temporary",
        "next": 99
    }))
    .unwrap();
    assert!(matches!(
        retry,
        Info::Retry { attempt: 4, ref message, next: 99 } if message == "temporary"
    ));
}

#[tokio::test]
async fn set_publishes_status_and_idle_events_with_project_directory_scope() {
    let temp = TempDir::new().expect("tempdir should create");
    let session_id = format!("status-event-test-{}", uuid::Uuid::new_v4());
    let events = Arc::new(Mutex::new(Vec::new()));

    let events_for_callback = events.clone();
    let session_id_for_callback = session_id.clone();
    let unsubscribe = bus::global_subscribe(move |event| {
        let is_target_session = event
            .payload
            .get("properties")
            .and_then(|properties| properties.get("sessionID"))
            .and_then(|session_id| session_id.as_str())
            == Some(session_id_for_callback.as_str());
        if is_target_session {
            events_for_callback.lock().expect("events lock").push(event);
        }
    });

    let session_id_for_context = session_id.clone();
    instance::provide(temp.path(), None, move || {
        Box::pin(async move {
            set(&session_id_for_context, Info::Busy);
            set(&session_id_for_context, Info::Idle);
        })
    })
    .await
    .expect("project context should provide");
    unsubscribe();

    let events = events.lock().expect("events lock");
    let directory = temp.path().to_string_lossy().to_string();
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|event| event.directory.as_deref() == Some(directory.as_str())));
    assert_eq!(events[0].payload["type"], "session.status");
    assert_eq!(events[0].payload["properties"]["status"]["type"], "busy");
    assert_eq!(events[1].payload["type"], "session.status");
    assert_eq!(events[1].payload["properties"]["status"]["type"], "idle");
    assert_eq!(events[2].payload["type"], "session.idle");
}
