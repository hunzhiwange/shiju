use super::*;

#[test]
fn qq_seed_from_secret_repeats_secret_bytes_to_seed_length() {
    let seed = qq_seed_from_secret("ab").expect("seed");

    assert_eq!(seed[0], b'a');
    assert_eq!(seed[1], b'b');
    assert_eq!(seed[2], b'a');
    assert!(qq_seed_from_secret("").is_none());
}

#[test]
fn qq_webhook_validation_signature_is_deterministic() {
    let first = qq_webhook_validation_signature("secret", "123", "plain").expect("signature");
    let second = qq_webhook_validation_signature("secret", "123", "plain").expect("signature");

    assert_eq!(first, second);
    assert_eq!(first.len(), 128);
}

#[test]
fn qq_webhook_validation_signature_rejects_empty_secret() {
    assert_eq!(qq_webhook_validation_signature("", "123", "plain"), None);
}

#[test]
fn webhook_validation_response_rejects_non_validation_or_missing_fields() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![]);

    assert!(channel.build_webhook_validation_response(&serde_json::json!({"op": 0})).is_none());
    assert!(channel.build_webhook_validation_response(&serde_json::json!({"op": 13})).is_none());
    assert!(
        channel
            .build_webhook_validation_response(&serde_json::json!({
                "op": 13,
                "d": {"event_ts": "123"}
            }))
            .is_none()
    );
    assert!(
        channel
            .build_webhook_validation_response(&serde_json::json!({
                "op": 13,
                "d": {"plain_token": "token", "event_ts": " "}
            }))
            .is_none()
    );
}

#[test]
fn webhook_validation_response_trims_plain_token_and_timestamp() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![]);
    let response = channel
        .build_webhook_validation_response(&serde_json::json!({
            "op": 13,
            "d": {"plain_token": " token ", "event_ts": " 123 "}
        }))
        .expect("validation response");

    assert_eq!(response["plain_token"], "token");
    assert_eq!(response["signature"].as_str().unwrap().len(), 128);
}

#[test]
fn webhook_validation_response_uses_trimmed_inputs_for_signature() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec![]);
    let response = channel
        .build_webhook_validation_response(&serde_json::json!({
            "op": 13,
            "d": {"plain_token": " token ", "event_ts": " 123 "}
        }))
        .expect("validation response");

    let expected_signature =
        qq_webhook_validation_signature("secret", "123", "token").expect("signature");

    assert_eq!(response["plain_token"], "token");
    assert_eq!(response["signature"], expected_signature);
}

#[test]
fn webhook_validation_response_rejects_empty_app_secret() {
    let channel = QQChannel::new("app".to_string(), "".to_string(), vec![]);

    assert!(
        channel
            .build_webhook_validation_response(&serde_json::json!({
                "op": 13,
                "d": {"plain_token": "token", "event_ts": "123"}
            }))
            .is_none()
    );
}

#[tokio::test]
async fn parse_webhook_payload_ignores_non_dispatch_payloads() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec!["*".to_string()]);

    assert!(channel.parse_webhook_payload(&serde_json::json!({"op": 13})).await.is_empty());
    assert!(
        channel.parse_webhook_payload(&serde_json::json!({"op": 0, "t": " "})).await.is_empty()
    );
    assert!(
        channel
            .parse_webhook_payload(&serde_json::json!({"op": 0, "t": "C2C_MESSAGE_CREATE"}))
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn parse_webhook_payload_dispatches_trimmed_c2c_event() {
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["user-1".to_string()]);
    let messages = channel
        .parse_webhook_payload(&serde_json::json!({
            "op": 0,
            "t": " C2C_MESSAGE_CREATE ",
            "d": {
                "id": "msg-webhook",
                "content": "hello",
                "author": {"id": "author-1", "user_openid": "user-1"}
            }
        }))
        .await;

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].sender, "user-1");
    assert_eq!(messages[0].reply_target, "user:user-1");
    assert_eq!(messages[0].content, "hello");
    assert_eq!(messages[0].thread_ts.as_deref(), Some("msg-webhook"));
}

#[tokio::test]
async fn parse_dispatch_message_event_handles_c2c_fallback_author_and_authorization() {
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["author-1".to_string()]);
    let payload = serde_json::json!({
        "id": "msg-1",
        "content": "hello",
        "author": {"id": "author-1"}
    });

    let message = channel
        .parse_dispatch_message_event("C2C_MESSAGE_CREATE", &payload)
        .await
        .expect("message");

    assert_eq!(message.sender, "author-1");
    assert_eq!(message.reply_target, "user:author-1");
    assert_eq!(message.thread_ts.as_deref(), Some("msg-1"));

    let unauthorized =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["other".to_string()]);
    assert!(
        unauthorized.parse_dispatch_message_event("C2C_MESSAGE_CREATE", &payload).await.is_none()
    );
}

#[tokio::test]
async fn parse_dispatch_message_event_handles_c2c_missing_author_and_empty_content() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec!["*".to_string()]);
    let payload = serde_json::json!({"id": "msg-unknown", "content": "hello"});

    let message = channel
        .parse_dispatch_message_event("C2C_MESSAGE_CREATE", &payload)
        .await
        .expect("message");
    let empty = channel
        .parse_dispatch_message_event(
            "C2C_MESSAGE_CREATE",
            &serde_json::json!({"id": "msg-empty", "content": " "}),
        )
        .await;

    assert_eq!(message.sender, "unknown");
    assert_eq!(message.reply_target, "user:unknown");
    assert!(empty.is_none());
}

#[tokio::test]
async fn parse_dispatch_message_event_deduplicates_c2c_messages() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec!["*".to_string()]);
    let payload = serde_json::json!({
        "id": "msg-c2c-duplicate",
        "content": "hello",
        "author": {"id": "author-1"}
    });

    assert!(channel.parse_dispatch_message_event("C2C_MESSAGE_CREATE", &payload).await.is_some());
    assert!(channel.parse_dispatch_message_event("C2C_MESSAGE_CREATE", &payload).await.is_none());
}

#[tokio::test]
async fn parse_dispatch_message_event_handles_group_messages_and_dedup() {
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["member-1".to_string()]);
    let payload = serde_json::json!({
        "id": "msg-2",
        "content": "group hello",
        "group_openid": "group-1",
        "author": {"member_openid": "member-1"}
    });

    let first = channel
        .parse_dispatch_message_event("GROUP_AT_MESSAGE_CREATE", &payload)
        .await
        .expect("message");
    let duplicate = channel.parse_dispatch_message_event("GROUP_AT_MESSAGE_CREATE", &payload).await;

    assert_eq!(first.sender, "member-1");
    assert_eq!(first.reply_target, "group:group-1");
    assert!(duplicate.is_none());
    assert!(channel.parse_dispatch_message_event("UNKNOWN", &payload).await.is_none());
}

#[tokio::test]
async fn parse_dispatch_message_event_handles_group_defaults_and_authorization() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec!["*".to_string()]);
    let payload = serde_json::json!({"content": "group hello"});

    let message = channel
        .parse_dispatch_message_event("GROUP_AT_MESSAGE_CREATE", &payload)
        .await
        .expect("message");

    assert_eq!(message.sender, "unknown");
    assert_eq!(message.reply_target, "group:unknown");
    assert_eq!(message.thread_ts, None);

    let unauthorized =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["member-2".to_string()]);
    assert!(
        unauthorized
            .parse_dispatch_message_event(
                "GROUP_AT_MESSAGE_CREATE",
                &serde_json::json!({
                    "id": "msg-group-unauthorized",
                    "content": "group hello",
                    "author": {"member_openid": "member-1"}
                }),
            )
            .await
            .is_none()
    );
    assert!(
        channel
            .parse_dispatch_message_event(
                "GROUP_AT_MESSAGE_CREATE",
                &serde_json::json!({"id": "msg-group-empty", "content": ""}),
            )
            .await
            .is_none()
    );
}

#[tokio::test]
async fn parse_webhook_payload_dispatches_group_event_and_deduplicates() {
    let channel =
        QQChannel::new("app".to_string(), "secret".to_string(), vec!["member-1".to_string()]);
    let payload = serde_json::json!({
        "op": 0,
        "t": "GROUP_AT_MESSAGE_CREATE",
        "d": {
            "id": "msg-group-webhook",
            "content": "group hello",
            "group_openid": "group-1",
            "author": {"member_openid": "member-1"}
        }
    });

    let first = channel.parse_webhook_payload(&payload).await;
    let duplicate = channel.parse_webhook_payload(&payload).await;

    assert_eq!(first.len(), 1);
    assert_eq!(first[0].sender, "member-1");
    assert_eq!(first[0].reply_target, "group:group-1");
    assert_eq!(first[0].content, "group hello");
    assert_eq!(first[0].thread_ts.as_deref(), Some("msg-group-webhook"));
    assert!(duplicate.is_empty());
}

#[tokio::test]
async fn parse_webhook_payload_returns_empty_for_unknown_dispatch_event() {
    let channel = QQChannel::new("app".to_string(), "secret".to_string(), vec!["*".to_string()]);

    let messages = channel
        .parse_webhook_payload(&serde_json::json!({
            "op": 0,
            "t": "UNKNOWN_EVENT",
            "d": {"id": "msg-unknown", "content": "hello"}
        }))
        .await;

    assert!(messages.is_empty());
}
// 本文件验证 QQ Webhook 签名响应、事件解析与消息去重。
