use super::SessionScope;
use super::dynamic_calls::{
    ClientToolHost, ClientToolRequests, DynamicCallOwner, MAX_CLIENT_RESPONSE_BYTES,
    MAX_PENDING_CLIENT_CALLS,
};
use crate::tools::ToolSpec;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use vw_api_types::tools::{DynamicToolContent, DynamicToolResponse};

fn connection() -> (Arc<ClientToolHost>, ClientToolRequests) {
    let scope = SessionScope::new(Default::default(), std::env::current_dir().unwrap()).unwrap();
    ClientToolHost::register(DynamicCallOwner::new(scope, "session", "host").unwrap())
}
fn response() -> DynamicToolResponse {
    DynamicToolResponse {
        success: true,
        content: vec![DynamicToolContent::Text { text: "ok".into() }],
    }
}
fn spec() -> ToolSpec {
    ToolSpec::new("client_echo", "Echo", json!({"type":"object"}))
}
fn start(
    host: &Arc<ClientToolHost>,
) -> tokio::task::JoinHandle<anyhow::Result<DynamicToolResponse>> {
    let host = host.clone();
    tokio::spawn(async move { host.request(&spec(), json!({}), "turn".into()).await })
}

#[tokio::test]
async fn dynamic_tools_callback_response_owner_is_validated() {
    let (host, mut requests) = connection();
    let task = start(&host);
    let request = requests.recv().await.unwrap();
    let owner = host.owner().clone();
    let mut wrong = owner.clone();
    wrong.host = "other".into();
    assert!(host.respond(&wrong, &request.call_id, response()).is_err());
    wrong = owner.clone();
    wrong.session = "other".into();
    assert!(host.respond(&wrong, &request.call_id, response()).is_err());
    wrong = owner.clone();
    wrong.scope = SessionScope::new(
        vw_api_types::id::TenantScope::new("other", "company").unwrap(),
        owner.scope.directory(),
    )
    .unwrap();
    assert!(host.respond(&wrong, &request.call_id, response()).is_err());
    let other_dir = tempfile::tempdir().unwrap();
    wrong = owner.clone();
    wrong.scope = SessionScope::new(owner.scope.tenant().clone(), other_dir.path()).unwrap();
    assert!(host.respond(&wrong, &request.call_id, response()).is_err());
    assert_eq!(host.pending_count(), 1);
    host.respond(&owner, &request.call_id, response()).unwrap();
    assert_eq!(task.await.unwrap().unwrap(), response());
    assert_eq!(host.pending_count(), 0);
    assert!(host.respond(&owner, &request.call_id, response()).is_err());
    assert!(host.respond(&owner, "unknown", response()).is_err());
}

#[tokio::test]
async fn dynamic_tools_callback_cancel_and_disconnect_reclaim_pending() {
    let (host, mut requests) = connection();
    let task = start(&host);
    let request = requests.recv().await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(host.pending_count(), 0);
    assert!(host.respond(host.owner(), &request.call_id, response()).is_err());
    let task = start(&host);
    requests.recv().await.unwrap();
    drop(requests);
    assert!(task.await.unwrap().unwrap_err().to_string().contains("disconnected"));
    assert_eq!(host.pending_count(), 0);
    assert!(host.request(&spec(), json!({}), "turn".into()).await.is_err());
}

#[tokio::test]
async fn dynamic_tools_callback_timeout_and_expired_reply_are_rejected() {
    let (host, mut requests) = connection();
    let timeout_host = host.clone();
    let timeout_call = tokio::spawn(async move {
        timeout_host.request_with_timeout(&spec(), json!({}), "turn".into(), Duration::ZERO).await
    });
    requests.recv().await.unwrap();
    assert!(timeout_call.await.unwrap().unwrap_err().to_string().contains("timed out"));
    assert_eq!(host.pending_count(), 0);
    let host_clone = host.clone();
    let call = tokio::spawn(async move {
        host_clone.request_with_timeout(&spec(), json!({}), "turn".into(), Duration::ZERO).await
    });
    let request = requests.recv().await.unwrap();
    assert!(host.respond(host.owner(), &request.call_id, response()).is_err());
    assert!(call.await.unwrap().is_err());
    assert_eq!(host.pending_count(), 0);
}

#[tokio::test]
async fn dynamic_tools_callback_limits_pending_arguments_and_response_bytes() {
    let (host, mut requests) = connection();
    let mut tasks = Vec::new();
    for _ in 0..MAX_PENDING_CLIENT_CALLS {
        tasks.push(start(&host));
        requests.recv().await.unwrap();
    }
    assert_eq!(host.pending_count(), MAX_PENDING_CLIENT_CALLS);
    assert!(
        host.request(&spec(), json!({}), "turn".into())
            .await
            .unwrap_err()
            .to_string()
            .contains("limit")
    );
    host.disconnect();
    for task in tasks {
        assert!(task.await.unwrap().is_err());
    }
    assert_eq!(host.pending_count(), 0);
    let (host, mut requests) = connection();
    assert!(
        host.request(&spec(), json!({"x":"x".repeat(1024 * 1024)}), "turn".into()).await.is_err()
    );
    assert_eq!(host.pending_count(), 0);
    let task = start(&host);
    let request = requests.recv().await.unwrap();
    let oversized = DynamicToolResponse {
        success: true,
        content: vec![DynamicToolContent::Text { text: "x".repeat(MAX_CLIENT_RESPONSE_BYTES) }],
    };
    assert!(host.respond(host.owner(), &request.call_id, oversized).is_err());
    host.respond(host.owner(), &request.call_id, response()).unwrap();
    assert!(task.await.unwrap().is_ok());
}

#[tokio::test]
async fn dynamic_tools_callback_closed_host_cannot_insert_pending() {
    let (host, _requests) = connection();
    host.disconnect();
    assert!(host.request(&spec(), json!({}), "turn".into()).await.is_err());
    assert_eq!(host.pending_count(), 0);
}

#[tokio::test]
async fn dynamic_tools_callback_full_delivery_queue_does_not_leak_pending() {
    let (host, mut requests) = connection();
    for _ in 0..MAX_PENDING_CLIENT_CALLS {
        // Poll until the request is queued, then drop its waiter to cancel it.
        let tool_spec = spec();
        let call = host.request(&tool_spec, json!({}), "turn".into());
        tokio::pin!(call);
        assert!(futures_util::poll!(&mut call).is_pending());
    }
    assert_eq!(host.pending_count(), 0);
    assert!(
        host.request(&spec(), json!({}), "turn".into())
            .await
            .unwrap_err()
            .to_string()
            .contains("queue full")
    );
    assert_eq!(host.pending_count(), 0);
    requests.recv().await.unwrap();
    let task = start(&host);
    // Drain the cancelled calls, then answer the new live call.
    for _ in 0..MAX_PENDING_CLIENT_CALLS - 1 {
        requests.recv().await.unwrap();
    }
    let request = requests.recv().await.unwrap();
    host.respond(host.owner(), &request.call_id, response()).unwrap();
    assert!(task.await.unwrap().is_ok());
}
