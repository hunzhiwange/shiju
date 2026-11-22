use super::*;
use std::future::Future;

#[tokio::test]
async fn acp_platform_approval_binding_rejects_stale_callback_after_replacement() {
    let slot = Mutex::new(None);
    let old = PromptApproval::bind(
        &slot,
        Some(Arc::new(|_, _| Box::pin(async { true }))),
        CancellationToken::new(),
    );
    let captured = slot.lock().clone().unwrap();
    let current = PromptApproval::bind(
        &slot,
        Some(Arc::new(|_, _| Box::pin(async { true }))),
        CancellationToken::new(),
    );
    drop(old);
    let input = serde_json::json!({"path": "/tmp/shiju-approval/note.txt"});
    assert!(!captured("Native write".into(), input.clone()).await);
    let confirm = slot.lock().clone().unwrap();
    assert!(confirm("Mounted write".into(), input).await);
    drop(current);
    assert!(slot.lock().is_none());
}

#[tokio::test]
async fn acp_platform_approval_binding_cancels_inflight_confirmation() {
    let slot = Mutex::new(None);
    let cancel = CancellationToken::new();
    let _binding = PromptApproval::bind(
        &slot,
        Some(Arc::new(|_, _| Box::pin(std::future::pending()))),
        cancel.clone(),
    );
    let confirm = slot.lock().clone().unwrap();
    let pending = confirm("Terminal".into(), serde_json::json!({"command": "/usr/bin/true"}));
    tokio::pin!(pending);
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(pending.as_mut().poll(cx).is_pending()))
            .await
    );
    cancel.cancel();
    assert!(!tokio::time::timeout(std::time::Duration::from_secs(1), pending).await.unwrap());
}
