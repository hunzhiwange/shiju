use axum::{Json, Router, routing::post};
use base64::Engine;
use ring::aead::{AES_256_GCM, Aad, LessSafeKey, Nonce, UnboundKey};
use serde_json::json;
use tokio::net::TcpListener;

use super::{PollState, decrypt_secret, fetch_from, map_poll_response};
use crate::app::agent::gateway::api::handlers::channel_qr::ChannelQrStatus;

const KEY: [u8; 32] = [0x42; 32];
const NONCE: [u8; 12] = [0x24; 12];

fn encrypted_secret(secret: &str) -> String {
    let key = LessSafeKey::new(UnboundKey::new(&AES_256_GCM, &KEY).expect("AES key"));
    let mut ciphertext = secret.as_bytes().to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(NONCE),
        Aad::empty(),
        &mut ciphertext,
    )
    .expect("encrypt fixture");
    let mut payload = NONCE.to_vec();
    payload.extend(ciphertext);
    base64::engine::general_purpose::STANDARD.encode(payload)
}

#[test]
fn qq_qr_rejects_tampered_secret_and_maps_valid_credentials() {
    let encrypted = encrypted_secret("app-secret");
    assert_eq!(decrypt_secret(&encrypted, &KEY).expect("decrypt fixture"), "app-secret");

    let valid = map_poll_response(
        json!({
            "retcode": 0,
            "data": {
                "status": 2,
                "bot_appid": 123456,
                "bot_encrypt_secret": encrypted,
                "user_openid": "user-1"
            }
        }),
        &KEY,
    );
    assert_eq!(valid.status, ChannelQrStatus::Authorized);
    assert_eq!(
        valid.credentials.expect("credentials"),
        json!({
            "app_id": "123456",
            "app_secret": "app-secret",
            "user_openid": "user-1"
        })
        .as_object()
        .expect("credential object")
        .clone()
    );

    let mut tampered = encrypted_secret("app-secret");
    tampered.replace_range(20..21, "A");
    let rejected = map_poll_response(
        json!({
            "retcode": 0,
            "data": {
                "status": 2,
                "bot_appid": "123456",
                "bot_encrypt_secret": tampered
            }
        }),
        &KEY,
    );
    assert_eq!(rejected.status, ChannelQrStatus::Failed);
    assert!(rejected.credentials.is_none());

    let missing = map_poll_response(
        json!({
            "retcode": 0,
            "data": {"status": 2, "bot_appid": "123456"}
        }),
        &KEY,
    );
    assert_eq!(missing.status, ChannelQrStatus::Failed);
    assert!(missing.credentials.is_none());

    let expired = map_poll_response(
        json!({
            "retcode": 0,
            "data": {"status": 3}
        }),
        &KEY,
    );
    assert_eq!(expired.status, ChannelQrStatus::Expired);
    assert!(expired.credentials.is_none());
}

#[tokio::test]
async fn qq_qr_fetch_creates_server_side_poll_state_from_portal_response() {
    let app = Router::new().route(
        "/lite/create_bind_task",
        post(|| async { Json(json!({"retcode": 0, "data": {"task_id": "task/1"}})) }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind QQ portal fixture");
    let address = listener.local_addr().expect("QQ fixture address");
    let server =
        tokio::spawn(async move { axum::serve(listener, app).await.expect("serve fixture") });

    let fetched = fetch_from(&reqwest::Client::new(), &format!("http://{address}"))
        .await
        .expect("fetch bind task");
    assert!(fetched.scan_url.contains("task_id=task%2F1"));
    assert!(!fetched.scan_url.contains("key"));
    let state: PollState = serde_json::from_str(&fetched.poll_state).expect("server poll state");
    assert_eq!(state.task_id, "task/1");
    assert_eq!(base64::engine::general_purpose::STANDARD.decode(state.key).unwrap().len(), 32);

    server.abort();
}
