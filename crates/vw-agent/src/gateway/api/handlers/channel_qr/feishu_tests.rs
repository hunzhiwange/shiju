use super::{fetch_from_endpoint, poll_from_endpoint};
use crate::app::agent::gateway::api::handlers::channel_qr::ChannelQrStatus;
use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::routing::post;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct MockState {
    requests: Arc<Mutex<Vec<String>>>,
    responses: Arc<Mutex<Vec<Value>>>,
}

async fn registration(State(state): State<MockState>, body: Bytes) -> axum::Json<Value> {
    state
        .requests
        .lock()
        .expect("request log")
        .push(String::from_utf8(body.to_vec()).expect("form body"));
    axum::Json(state.responses.lock().expect("response queue").remove(0))
}

async fn mock_server(responses: Vec<Value>) -> (String, Arc<Mutex<Vec<String>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let state =
        MockState { requests: Arc::clone(&requests), responses: Arc::new(Mutex::new(responses)) };
    let app =
        Router::new().route("/oauth/v1/app/registration", post(registration)).with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind mock server");
    let address = listener.local_addr().expect("mock server address");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock server should remain available");
    });
    (format!("http://{address}/oauth/v1/app/registration"), requests)
}

#[tokio::test]
async fn feishu_qr_maps_device_grant_and_expiry() {
    let (endpoint, requests) = mock_server(vec![
        json!({"supported_auth_methods": ["client_secret"]}),
        json!({
            "device_code": "device-123",
            "verification_uri_complete": "https://accounts.feishu.cn/qr?code=abc"
        }),
        json!({"error": "expired_token"}),
    ])
    .await;

    let fetched = fetch_from_endpoint(&endpoint).await.expect("device grant should begin");
    assert_eq!(fetched.poll_state, "device-123");
    assert_eq!(fetched.scan_url, "https://accounts.feishu.cn/qr?code=abc&source=VibeWindow");

    let expired = poll_from_endpoint(&endpoint, &fetched.poll_state)
        .await
        .expect("expired response should map to a terminal state");
    assert_eq!(expired.status, ChannelQrStatus::Expired);
    assert!(expired.credentials.is_none());

    assert_eq!(
        requests.lock().expect("request log").as_slice(),
        [
            "action=init",
            "action=begin&archetype=PersonalAgent&auth_method=client_secret&request_user_info=open_id",
            "action=poll&device_code=device-123",
        ]
    );
}

#[tokio::test]
async fn feishu_qr_maps_pending_success_and_failures() {
    let (endpoint, _) = mock_server(vec![
        json!({"error": "authorization_pending"}),
        json!({"error": "slow_down"}),
        json!({"client_id": "cli-123", "client_secret": "secret-456"}),
        json!({"error": "access_denied"}),
        json!({"error": "unexpected_error"}),
    ])
    .await;

    for expected in [ChannelQrStatus::Pending, ChannelQrStatus::Pending] {
        let result = poll_from_endpoint(&endpoint, "device-123").await.expect("pending response");
        assert_eq!(result.status, expected);
        assert!(result.credentials.is_none());
    }

    let authorized =
        poll_from_endpoint(&endpoint, "device-123").await.expect("authorized response");
    assert_eq!(authorized.status, ChannelQrStatus::Authorized);
    assert_eq!(
        authorized.credentials.expect("mapped credentials"),
        json!({"app_id": "cli-123", "app_secret": "secret-456"})
            .as_object()
            .expect("credential object")
            .clone()
    );

    for expected in [ChannelQrStatus::Failed, ChannelQrStatus::Failed] {
        let result = poll_from_endpoint(&endpoint, "device-123").await.expect("failure response");
        assert_eq!(result.status, expected);
        assert!(result.credentials.is_none());
    }
}
