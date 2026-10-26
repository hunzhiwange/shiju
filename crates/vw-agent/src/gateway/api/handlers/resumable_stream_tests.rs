// 覆盖可恢复事件流的游标、重放、心跳及断开语义。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::Semaphore;
use vw_api_types::id::TenantScope;

use super::*;

fn tenant(company_id: &str) -> TenantScope {
    TenantScope::new("platform-test", company_id).expect("valid tenant")
}

// 事件序号和缓冲区容量显式固定，用于验证断线重连时的重放边界。
#[tokio::test]
async fn reconnect_replays_after_cursor_without_restarting_producer() {
    let store = ResumableStreamStore::new(8, Duration::from_secs(60));
    let signature = request_signature(b"same request");
    let starts = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(Semaphore::new(0));

    let opened =
        store.open(tenant("company-a"), StreamKind::Chat, "request-1", signature.clone(), 0);
    let mut first = opened.expect("first subscription");
    let producer = first.producer.take().expect("first subscriber starts producer");
    let starts_for_producer = Arc::clone(&starts);
    let release_for_producer = Arc::clone(&release);
    tokio::spawn(async move {
        starts_for_producer.fetch_add(1, Ordering::SeqCst);
        assert!(producer.publish(r#"{"type":"chat.delta","delta":"one"}"#, false));
        release_for_producer.acquire().await.expect("release producer").forget();
        assert!(producer.publish(r#"{"type":"chat.delta","delta":"two"}"#, false));
        assert!(producer.publish(r#"{"type":"chat.done"}"#, true));
    });

    let first_event = first.subscription.next().await.expect("first event");
    assert_eq!(first_event.id, 1);
    assert!(first_event.data.contains("one"));
    drop(first.subscription);

    let mut resumed = store
        .open(tenant("company-a"), StreamKind::Chat, "request-1", signature, first_event.id)
        .expect("resumed subscription");
    assert!(resumed.producer.is_none(), "reconnect must reuse the running producer");
    release.add_permits(1);

    let replayed = resumed.subscription.next().await.expect("event after cursor");
    let terminal = resumed.subscription.next().await.expect("terminal event");
    assert_eq!(replayed.id, 2);
    assert!(replayed.data.contains("two"));
    assert_eq!(terminal.id, 3);
    assert!(terminal.data.contains("chat.done"));
    assert!(resumed.subscription.next().await.is_none(), "terminal ends subscription");
    assert_eq!(starts.load(Ordering::SeqCst), 1, "producer starts exactly once");
}

#[test]
fn duplicate_request_id_with_different_body_is_rejected() {
    let store = ResumableStreamStore::new(8, Duration::from_secs(60));
    store
        .open(
            tenant("company-a"),
            StreamKind::TaskPlanRun,
            "request-1",
            request_signature(b"first body"),
            0,
        )
        .expect("first request");

    let error = store
        .open(
            tenant("company-a"),
            StreamKind::TaskPlanRun,
            "request-1",
            request_signature(b"different body"),
            0,
        )
        .expect_err("same id with another body must conflict");

    assert_eq!(error, StreamOpenError::RequestConflict);
}

#[test]
fn request_ids_are_isolated_by_tenant_and_stream_kind() {
    let store = ResumableStreamStore::new(8, Duration::from_secs(60));
    let signature = request_signature(b"same request");

    let chat_a = store
        .open(tenant("company-a"), StreamKind::Chat, "shared", signature.clone(), 0)
        .expect("tenant a chat");
    let chat_b = store
        .open(tenant("company-b"), StreamKind::Chat, "shared", signature.clone(), 0)
        .expect("tenant b chat");
    let plan_a = store
        .open(tenant("company-a"), StreamKind::TaskPlanRun, "shared", signature, 0)
        .expect("tenant a plan");

    assert!(chat_a.producer.is_some());
    assert!(chat_b.producer.is_some());
    assert!(plan_a.producer.is_some());
}

#[test]
fn registered_stream_cancellation_is_tenant_and_kind_scoped() {
    let store = ResumableStreamStore::new(8, Duration::from_secs(60));
    let signature = request_signature(b"same request");
    let cancellations = Arc::new(AtomicUsize::new(0));
    let producer = store
        .open(tenant("company-a"), StreamKind::Chat, "shared", signature, 0)
        .expect("chat stream")
        .producer
        .expect("chat producer");
    let cancellations_for_callback = Arc::clone(&cancellations);
    producer.install_cancellation(move || {
        cancellations_for_callback.fetch_add(1, Ordering::SeqCst);
    });

    assert!(!store.cancel(tenant("company-b"), StreamKind::Chat, "shared"));
    assert!(!store.cancel(tenant("company-a"), StreamKind::TaskPlanRun, "shared"));
    assert!(store.cancel(tenant("company-a"), StreamKind::Chat, "shared"));
    assert!(store.cancel(tenant("company-a"), StreamKind::Chat, "shared"));
    assert_eq!(cancellations.load(Ordering::SeqCst), 1, "cancellation remains idempotent");
}

#[test]
fn capacity_evicts_oldest_terminal_run_but_never_active_run() {
    let store = ResumableStreamStore::new(1, Duration::from_secs(60));
    let signature = request_signature(b"same request");

    let active = store
        .open(tenant("company-a"), StreamKind::Chat, "active", signature.clone(), 0)
        .expect("active run");
    let terminal_a = store
        .open(tenant("company-a"), StreamKind::Chat, "terminal-a", signature.clone(), 0)
        .expect("first terminal run")
        .producer
        .expect("first producer");
    assert!(terminal_a.publish(r#"{"type":"chat.done"}"#, true));
    let terminal_b = store
        .open(tenant("company-a"), StreamKind::Chat, "terminal-b", signature.clone(), 0)
        .expect("second terminal run")
        .producer
        .expect("second producer");
    assert!(terminal_b.publish(r#"{"type":"chat.done"}"#, true));

    let reopened_active = store
        .open(tenant("company-a"), StreamKind::Chat, "active", signature.clone(), 0)
        .expect("active run remains registered");
    let reopened_old_terminal = store
        .open(tenant("company-a"), StreamKind::Chat, "terminal-a", signature, 0)
        .expect("evicted terminal run can start again");

    assert!(active.producer.is_some());
    assert!(reopened_active.producer.is_none(), "active run must not be evicted");
    assert!(reopened_old_terminal.producer.is_some(), "oldest terminal run is evicted");
}

#[test]
fn expired_terminal_run_can_start_again() {
    let store = ResumableStreamStore::new(8, Duration::ZERO);
    let signature = request_signature(b"same request");
    let first = store
        .open(tenant("company-a"), StreamKind::Chat, "expired", signature.clone(), 0)
        .expect("first run")
        .producer
        .expect("first producer");
    assert!(first.publish(r#"{"type":"chat.done"}"#, true));

    let reopened = store
        .open(tenant("company-a"), StreamKind::Chat, "expired", signature, 0)
        .expect("expired run can reopen");

    assert!(reopened.producer.is_some());
}

#[test]
fn shell_session_completed_chat_retains_idempotent_stop_hook() {
    let store = ResumableStreamStore::new(8, Duration::from_secs(60));
    let producer = store
        .open(
            tenant("company-a"),
            StreamKind::Chat,
            "background",
            request_signature(b"background"),
            0,
        )
        .unwrap()
        .producer
        .unwrap();
    let cancellations = Arc::new(AtomicUsize::new(0));
    let observed = cancellations.clone();
    producer.install_cancellation(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    assert!(producer.publish("done", true));
    assert_eq!(
        cancellations.load(Ordering::SeqCst),
        0,
        "normal completion preserves background commands"
    );
    assert!(!store.cancel(tenant("company-b"), StreamKind::Chat, "background"));
    assert!(store.cancel(tenant("company-a"), StreamKind::Chat, "background"));
    assert!(store.cancel(tenant("company-a"), StreamKind::Chat, "background"));
    assert_eq!(cancellations.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn recovery_event_resume_keeps_request_and_terminal_error() {
    let store = ResumableStreamStore::new(8, Duration::from_secs(60));
    let signature = request_signature(b"recovery request");
    let mut first = store
        .open(tenant("company-a"), StreamKind::Chat, "recovery-request", signature.clone(), 0)
        .unwrap();
    let producer = first.producer.take().unwrap();
    let waiting = r#"{"type":"chat.model_recovery","phase":"waiting","attempt":1,"max_attempts":2,"wait_ms":500,"reason":"temporary","next_action":"retry","session_id":"session-recovery"}"#;
    assert!(producer.publish(waiting, false));
    let delivered = first.subscription.next().await.unwrap();
    assert_eq!(delivered.data, waiting);
    drop(first.subscription);
    let mut resumed = store
        .open(
            tenant("company-a"),
            StreamKind::Chat,
            "recovery-request",
            signature.clone(),
            delivered.id,
        )
        .unwrap();
    assert!(resumed.producer.is_none());
    let stopped = waiting.replace("waiting", "stopped");
    assert!(producer.publish(&stopped, false));
    assert!(producer.publish(r#"{"type":"chat.error","error":"exhausted"}"#, true));
    let status = resumed.subscription.next().await.unwrap();
    let terminal = resumed.subscription.next().await.unwrap();
    assert_eq!(status.id, 2);
    assert_eq!(status.data, stopped);
    assert_eq!(terminal.id, 3);
    assert!(terminal.data.contains("exhausted"));
    assert!(resumed.subscription.next().await.is_none());
    let mut repeated = store
        .open(tenant("company-a"), StreamKind::Chat, "recovery-request", signature, terminal.id)
        .unwrap();
    assert!(repeated.producer.is_none());
    assert!(repeated.subscription.next().await.is_none());
}
