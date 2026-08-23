//! 验证活动提示词的取消请求、会话匹配、完成等待与超时处理。

use std::collections::HashMap;

use tokio::sync::watch;
use tokio::time::{Duration, sleep};

use crate::types::AcpAgentConfig;

use super::{AcpClient, AcpError};

fn client() -> AcpClient {
    AcpClient::new(
        "test-agent",
        AcpAgentConfig { command: "agent".to_string(), args: Vec::new(), env: HashMap::new() },
    )
}

fn register_prompt(
    client: &AcpClient,
    session_id: &str,
) -> (watch::Receiver<bool>, watch::Sender<bool>) {
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let (completed_tx, completed_rx) = watch::channel(false);

    client.register_active_prompt(session_id.to_string(), cancel_tx, completed_rx);

    (cancel_rx, completed_tx)
}

#[tokio::test]
async fn cancel_returns_false_without_active_prompt() {
    let client = client();

    assert!(!client.has_active_prompt());
    assert!(!client.cancel("session-1").await.expect("cancel"));
    assert!(!client.request_cancel_active_prompt().await.expect("request cancel"));
    assert!(!client.cancel_active_prompt(1).await.expect("cancel active prompt"));
}

#[tokio::test]
async fn cancel_returns_false_for_non_matching_session() {
    let client = client();
    let (cancel_rx, _completed_tx) = register_prompt(&client, "session-1");

    assert!(!client.cancel("session-2").await.expect("cancel"));
    assert!(!*cancel_rx.borrow());
    assert!(client.cancelling_session_ids.lock().is_empty());
}

#[tokio::test]
async fn cancel_sends_signal_and_tracks_session() {
    let client = client();
    let (mut cancel_rx, _completed_tx) = register_prompt(&client, "session-1");

    assert!(client.has_active_prompt());
    assert!(client.cancel("session-1").await.expect("cancel"));
    cancel_rx.changed().await.expect("cancel signal");

    assert!(*cancel_rx.borrow());
    assert!(client.cancelling_session_ids.lock().contains("session-1"));
}

#[tokio::test]
async fn request_cancel_active_prompt_cancels_current_prompt() {
    let client = client();
    let (mut cancel_rx, _completed_tx) = register_prompt(&client, "session-1");

    assert!(client.request_cancel_active_prompt().await.expect("request cancel"));
    cancel_rx.changed().await.expect("cancel signal");

    assert!(*cancel_rx.borrow());
}

#[tokio::test]
async fn cancel_active_prompt_with_zero_wait_only_requests_cancel() {
    let client = client();
    let (mut cancel_rx, _completed_tx) = register_prompt(&client, "session-1");

    assert!(client.cancel_active_prompt(0).await.expect("cancel active prompt"));
    cancel_rx.changed().await.expect("cancel signal");

    assert!(*cancel_rx.borrow());
}

#[tokio::test]
async fn cancel_active_prompt_returns_true_when_already_completed() {
    let client = client();
    let (cancel_tx, _cancel_rx) = watch::channel(false);
    let (completed_tx, completed_rx) = watch::channel(false);
    completed_tx.send(true).expect("mark completed");
    client.register_active_prompt("session-1".to_string(), cancel_tx, completed_rx);

    assert!(client.cancel_active_prompt(10).await.expect("cancel active prompt"));
}

#[tokio::test]
async fn cancel_active_prompt_waits_for_completion_signal() {
    let client = client();
    let (_cancel_rx, completed_tx) = register_prompt(&client, "session-1");

    tokio::spawn(async move {
        sleep(Duration::from_millis(1)).await;
        completed_tx.send(true).expect("mark completed");
    });

    assert!(client.cancel_active_prompt(50).await.expect("cancel active prompt"));
}

#[tokio::test]
async fn cancel_active_prompt_treats_closed_completion_channel_as_done() {
    let client = client();
    let (cancel_tx, _cancel_rx) = watch::channel(false);
    let (completed_tx, completed_rx) = watch::channel(false);
    client.register_active_prompt("session-1".to_string(), cancel_tx, completed_rx);
    drop(completed_tx);

    assert!(client.cancel_active_prompt(10).await.expect("cancel active prompt"));
}

#[tokio::test]
async fn cancel_active_prompt_returns_false_on_timeout() {
    let client = client();
    let (_cancel_rx, _completed_tx) = register_prompt(&client, "session-1");

    assert!(!client.cancel_active_prompt(1).await.expect("cancel active prompt"));
}

#[tokio::test]
async fn cancel_returns_error_when_cancel_receiver_is_closed() {
    let client = client();
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let (completed_tx, completed_rx) = watch::channel(false);
    client.register_active_prompt("session-1".to_string(), cancel_tx, completed_rx);
    drop(cancel_rx);

    let err = client.cancel("session-1").await.expect_err("cancel should fail");

    assert!(matches!(err, AcpError::Cancel(_)));
    assert!(client.cancelling_session_ids.lock().contains("session-1"));
    drop(completed_tx);
}

#[test]
fn clear_active_prompt_only_removes_matching_session() {
    let client = client();
    let (_cancel_rx, _completed_tx) = register_prompt(&client, "session-1");

    client.clear_active_prompt("session-2");
    assert!(client.has_active_prompt());

    client.clear_active_prompt("session-1");
    assert!(!client.has_active_prompt());
}

#[tokio::test]
async fn update_active_prompt_session_only_changes_matching_prompt() {
    let client = client();
    let (mut cancel_rx, _completed_tx) = register_prompt(&client, "session-1");

    client.update_active_prompt_session("session-2", "ignored".to_string());
    assert!(!client.cancel("ignored").await.expect("cancel ignored"));

    client.update_active_prompt_session("session-1", "session-actual".to_string());
    assert!(client.cancel("session-actual").await.expect("cancel updated"));
    cancel_rx.changed().await.expect("cancel signal");

    assert!(*cancel_rx.borrow());
}
