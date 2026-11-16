//! Turn-owned task cancellation, isolation and concurrency contracts.

use super::AsyncHooks;

#[tokio::test]
async fn codex_plugin_compat_t35_drops_output_from_finished_turn() {
    let owner = AsyncHooks::default();
    let (send, receive) = tokio::sync::oneshot::channel();
    owner
        .schedule(async move {
            std::future::pending::<()>().await;
            let _ = send.send("late output");
        })
        .unwrap();
    drop(owner);
    assert!(receive.await.is_err());
}

#[tokio::test]
async fn codex_plugin_compat_t35_rejects_invalid_input() {
    let owner = AsyncHooks::default();
    owner.shutdown().await;
    assert!(owner.schedule(std::future::pending()).unwrap_err().to_string().contains("closed"));
}

#[tokio::test]
async fn codex_plugin_compat_t35_preserves_isolation() {
    let first = AsyncHooks::default();
    let second = AsyncHooks::default();
    let (send, receive) = tokio::sync::oneshot::channel();
    second
        .schedule(async move {
            let _ = send.send("second");
        })
        .unwrap();
    drop(first);
    assert_eq!(receive.await.unwrap(), "second");
}

#[tokio::test]
async fn codex_plugin_compat_t35_shutdown_cancels_and_rejects_restart() {
    let owner = AsyncHooks::default();
    let (send, receive) = tokio::sync::oneshot::channel::<()>();
    owner
        .schedule(async move {
            std::future::pending::<()>().await;
            drop(send);
        })
        .unwrap();
    owner.shutdown().await;
    assert!(receive.await.is_err());
    assert!(owner.schedule(async {}).is_err());
}

#[tokio::test]
async fn codex_plugin_compat_t35_queues_beyond_eight_active_commands() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let owner = AsyncHooks::default();
    let active = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0));
    for _ in 0..9 {
        let active = active.clone();
        let gate = gate.clone();
        owner
            .schedule(async move {
                active.fetch_add(1, Ordering::SeqCst);
                let _permit = gate.acquire().await.unwrap();
            })
            .unwrap();
    }
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while active.load(Ordering::SeqCst) < 8 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    tokio::task::yield_now().await;
    assert_eq!(active.load(Ordering::SeqCst), 8);
    gate.add_permits(1);
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while active.load(Ordering::SeqCst) < 9 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    owner.shutdown().await;
}
