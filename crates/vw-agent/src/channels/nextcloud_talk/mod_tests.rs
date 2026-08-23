// 本文件验证 Nextcloud Talk Webhook 解析、签名校验、收发与健康检查。

use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn one_shot_http_server(status_line: &'static str, body: &'static str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut buffer = vec![0; 4096];
        let _ = stream.read(&mut buffer).await.unwrap();
        let response = format!(
            "{status_line}\r\ncontent-length: {}\r\ncontent-type: application/json\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    format!("http://{addr}")
}

#[test]
fn constructor_trims_base_url_and_actor_ids_are_canonicalized() {
    let channel = NextcloudTalkChannel::new(
        "https://cloud.example.com///".to_string(),
        "app-token".to_string(),
        vec!["users/Alice".to_string()],
    );

    assert_eq!(channel.base_url, "https://cloud.example.com");
    assert_eq!(NextcloudTalkChannel::canonical_actor_id(" users/Alice "), "Alice");
    assert!(channel.is_user_allowed("alice"));
    assert!(channel.is_user_allowed("https://cloud.example.com/users/Alice"));
    assert!(!channel.is_user_allowed(""));
}

#[test]
fn parse_webhook_payload_handles_activity_content_object_and_timestamp_string() {
    let channel = NextcloudTalkChannel::new(
        "https://cloud.example.com".to_string(),
        "app-token".to_string(),
        vec!["alice".to_string()],
    );
    let payload = serde_json::json!({
        "type": "create",
        "timestamp": "1735701200123",
        "actor": {
            "type": "Person",
            "id": "https://cloud.example.com/users/alice"
        },
        "object": {
            "type": "note",
            "id": 99,
            "token": "room-token",
            "content": {
                "message": "  hello from object  "
            }
        }
    });

    let messages = channel.parse_webhook_payload(&payload);

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "99");
    assert_eq!(messages[0].sender, "alice");
    assert_eq!(messages[0].reply_target, "room-token");
    assert_eq!(messages[0].content, "hello from object");
    assert_eq!(messages[0].timestamp, 1_735_701_200);
}

#[test]
fn helper_parsers_cover_timestamp_value_and_content_variants() {
    let before = NextcloudTalkChannel::now_unix_secs();
    let fallback_timestamp =
        NextcloudTalkChannel::parse_timestamp_secs(Some(&serde_json::json!({"nested": true})));
    let after = NextcloudTalkChannel::now_unix_secs();

    assert_eq!(NextcloudTalkChannel::parse_timestamp_secs(Some(&serde_json::json!(42))), 42);
    assert_eq!(
        NextcloudTalkChannel::parse_timestamp_secs(Some(&serde_json::json!("1735701200123"))),
        1_735_701_200
    );
    assert!((before..=after).contains(&fallback_timestamp));

    assert_eq!(
        NextcloudTalkChannel::value_to_string(Some(&serde_json::json!("room-token"))),
        Some("room-token".to_string())
    );
    assert_eq!(
        NextcloudTalkChannel::value_to_string(Some(&serde_json::json!(99))),
        Some("99".to_string())
    );
    assert_eq!(NextcloudTalkChannel::value_to_string(Some(&serde_json::json!(true))), None);

    let json_string_payload = serde_json::json!({
        "object": { "content": r#"{"message":"  hello from json string  "}"# }
    });
    assert_eq!(
        NextcloudTalkChannel::extract_content_from_as2_object(&json_string_payload),
        Some("hello from json string".to_string())
    );

    let plain_string_payload = serde_json::json!({
        "object": { "content": "  plain text  " }
    });
    assert_eq!(
        NextcloudTalkChannel::extract_content_from_as2_object(&plain_string_payload),
        Some("plain text".to_string())
    );

    let blank_payload = serde_json::json!({
        "object": { "content": "   " }
    });
    assert_eq!(NextcloudTalkChannel::extract_content_from_as2_object(&blank_payload), None);
}

#[test]
fn extract_content_rejects_object_without_message_and_non_text_values() {
    let missing_message = serde_json::json!({
        "object": { "content": { "other": "value" } }
    });
    let unsupported_content = serde_json::json!({
        "object": { "content": ["message"] }
    });

    assert_eq!(NextcloudTalkChannel::extract_content_from_as2_object(&missing_message), None);
    assert_eq!(NextcloudTalkChannel::extract_content_from_as2_object(&unsupported_content), None);
}

#[test]
fn parse_webhook_payload_skips_unsupported_and_incomplete_payloads() {
    let channel = NextcloudTalkChannel::new(
        "https://cloud.example.com".to_string(),
        "app-token".to_string(),
        vec!["alice".to_string()],
    );

    let unsupported_object = serde_json::json!({
        "type": "create",
        "actor": {
            "type": "Person",
            "id": "alice"
        },
        "object": {
            "type": "article",
            "token": "room-token",
            "content": "ignored"
        }
    });
    assert!(channel.parse_webhook_payload(&unsupported_object).is_empty());

    let missing_room_token = serde_json::json!({
        "type": "message",
        "message": {
            "actorType": "users",
            "actorId": "alice",
            "message": "hello"
        }
    });
    assert!(channel.parse_webhook_payload(&missing_room_token).is_empty());

    let missing_actor_id = serde_json::json!({
        "type": "message",
        "object": { "token": "room-token" },
        "message": {
            "actorType": "users",
            "message": "hello"
        }
    });
    assert!(channel.parse_webhook_payload(&missing_actor_id).is_empty());

    let non_comment = serde_json::json!({
        "type": "message",
        "object": { "token": "room-token" },
        "message": {
            "actorType": "users",
            "actorId": "alice",
            "messageType": "reaction",
            "message": "hello"
        }
    });
    assert!(channel.parse_webhook_payload(&non_comment).is_empty());
}

#[test]
fn parse_webhook_payload_falls_back_to_plain_content_generated_id_and_current_timestamp() {
    let channel = NextcloudTalkChannel::new(
        "https://cloud.example.com".to_string(),
        "app-token".to_string(),
        vec!["Alice".to_string()],
    );
    let before = NextcloudTalkChannel::now_unix_secs();
    let payload = serde_json::json!({
        "type": "create",
        "actor": {
            "type": "Person",
            "id": "https://cloud.example.com/users/alice"
        },
        "object": {
            "type": "note",
            "content": "  plain content fallback  "
        },
        "target": {
            "id": "target-room"
        }
    });

    let messages = channel.parse_webhook_payload(&payload);
    let after = NextcloudTalkChannel::now_unix_secs();

    assert_eq!(messages.len(), 1);
    assert!(Uuid::parse_str(&messages[0].id).is_ok());
    assert_eq!(messages[0].reply_target, "target-room");
    assert_eq!(messages[0].sender, "alice");
    assert_eq!(messages[0].content, "plain content fallback");
    assert!((before..=after).contains(&messages[0].timestamp));
}

#[test]
// 签名输入缺失或编码非法时默认拒绝，不能退化为弱校验。
fn signature_verification_rejects_empty_random_and_bad_hex() {
    assert!(!verify_nextcloud_talk_signature("secret", "", "{}", "deadbeef"));
    assert!(!verify_nextcloud_talk_signature("secret", "nonce", "{}", "not-hex"));
}

#[tokio::test]
async fn send_posts_to_encoded_room_token() {
    let base_url = one_shot_http_server("HTTP/1.1 200 OK", "{}").await;
    let channel =
        NextcloudTalkChannel::new(base_url, "app-token".to_string(), vec!["*".to_string()]);

    channel.send(&SendMessage::new("hello", "room token/with slash")).await.unwrap();
}

#[tokio::test]
async fn send_reports_non_success_status() {
    let base_url =
        one_shot_http_server("HTTP/1.1 403 Forbidden", r#"{"ocs":{"meta":{"status":"failure"}}}"#)
            .await;
    let channel =
        NextcloudTalkChannel::new(base_url, "app-token".to_string(), vec!["*".to_string()]);

    let error =
        channel.send(&SendMessage::new("hello", "room-token")).await.unwrap_err().to_string();

    assert!(error.contains("Nextcloud Talk API error"));
    assert!(error.contains("403 Forbidden"));
}

#[tokio::test]
async fn health_check_tracks_http_success_and_failure() {
    let healthy_base_url = one_shot_http_server("HTTP/1.1 200 OK", "{}").await;
    let healthy_channel =
        NextcloudTalkChannel::new(healthy_base_url, "app-token".to_string(), vec!["*".to_string()]);
    assert!(healthy_channel.health_check().await);

    let unhealthy_base_url = one_shot_http_server("HTTP/1.1 503 Service Unavailable", "{}").await;
    let unhealthy_channel = NextcloudTalkChannel::new(
        unhealthy_base_url,
        "app-token".to_string(),
        vec!["*".to_string()],
    );
    assert!(!unhealthy_channel.health_check().await);
}

#[tokio::test]
async fn listen_remains_pending_in_webhook_mode() {
    let channel = NextcloudTalkChannel::new(
        "https://cloud.example.com".to_string(),
        "app-token".to_string(),
        vec!["*".to_string()],
    );
    let (tx, _rx) = tokio::sync::mpsc::channel(1);

    let result =
        tokio::time::timeout(std::time::Duration::from_millis(10), channel.listen(tx)).await;

    assert!(result.is_err());
}
