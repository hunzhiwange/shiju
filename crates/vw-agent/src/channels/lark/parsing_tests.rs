use super::*;

fn test_channel(mention_only: bool) -> LarkChannel {
    LarkChannel::new(
        "id".to_string(),
        "secret".to_string(),
        "token".to_string(),
        None,
        vec!["*".to_string()],
        mention_only,
    )
}

fn text_payload(text: &str) -> serde_json::Value {
    serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": serde_json::json!({ "text": text }).to_string(),
                "chat_id": "oc_chat",
                "create_time": "2000"
            }
        }
    })
}

#[test]
fn parse_post_content_details_extracts_text_links_and_mentions() {
    let content = serde_json::json!({
        "zh_cn": {
            "title": "Title",
            "content": [[
                { "tag": "text", "text": "hello " },
                { "tag": "a", "text": "site", "href": "https://example.com" },
                { "tag": "at", "user_name": "Bot", "user_id": "ou_bot" }
            ]]
        }
    })
    .to_string();

    let parsed = parse_post_content_details(&content).expect("post content");

    assert_eq!(parsed.text, "Title\n\nhello site@Bot");
    assert_eq!(parsed.mentioned_open_ids, vec!["ou_bot".to_string()]);
}

#[test]
fn parse_post_content_details_falls_back_to_en_or_first_locale_and_ignores_empty_content() {
    let en_content = serde_json::json!({
        "en_us": {
            "content": [[
                { "tag": "a", "href": "https://example.com" },
                { "tag": "at", "user_id": "ou_user" }
            ]]
        }
    })
    .to_string();

    let parsed = parse_post_content_details(&en_content).expect("en post content");
    assert_eq!(parsed.text, "https://example.com@ou_user");
    assert_eq!(parse_post_content(&en_content).as_deref(), Some("https://example.com@ou_user"));

    let first_locale = serde_json::json!({
        "fr_fr": { "title": "Bonjour", "content": [] }
    })
    .to_string();
    assert_eq!(parse_post_content(&first_locale).as_deref(), Some("Bonjour"));

    assert!(parse_post_content_details("not-json").is_none());
    assert!(parse_post_content_details(r#"{"zh_cn":{"content":[[]]}}"#).is_none());
}

#[test]
fn parse_post_content_details_handles_sparse_elements_and_missing_locale() {
    let content = serde_json::json!({
        "zh_cn": {
            "title": "",
            "content": [
                [
                    { "tag": "text" },
                    { "tag": "a", "text": "", "href": "https://fallback.example" },
                    { "tag": "a" },
                    { "tag": "at", "user_name": "", "user_id": "  " },
                    { "tag": "at" },
                    { "tag": "img" }
                ],
                "not-a-paragraph"
            ]
        }
    })
    .to_string();

    let parsed = parse_post_content_details(&content).expect("post content");

    assert_eq!(parsed.text, "https://fallback.example@@user");
    assert!(parsed.mentioned_open_ids.is_empty());
    assert!(parse_post_content_details("[]").is_none());
    assert!(parse_post_content_details(r#"{"zh_cn":null}"#).is_none());
}

#[test]
fn parse_image_key_and_strip_at_placeholders_handle_boundaries() {
    assert_eq!(parse_image_key(r#"{"image_key":"img_v2"}"#), Some("img_v2".to_string()));
    assert_eq!(parse_image_key(r#"{}"#), None);
    assert_eq!(parse_image_key(r#"{"image_key":42}"#), None);
    assert_eq!(parse_image_key("not json"), None);
    assert_eq!(strip_at_placeholders("hi @_user_1 there"), "hi there");
    assert_eq!(strip_at_placeholders("@_user_12"), "");
    assert_eq!(strip_at_placeholders("hi @alice there"), "hi @alice there");
}

#[test]
fn mention_matching_supports_nested_and_flat_open_id_shapes() {
    assert!(mention_matches_bot_open_id(
        &serde_json::json!({"id": {"open_id": "ou_bot"}}),
        "ou_bot"
    ));
    assert!(mention_matches_bot_open_id(&serde_json::json!({"open_id": "ou_bot"}), "ou_bot"));
    assert!(!mention_matches_bot_open_id(
        &serde_json::json!({"id": {"open_id": "ou_other"}}),
        "ou_bot"
    ));
    assert!(!mention_matches_bot_open_id(&serde_json::json!({"id": {}}), "ou_bot"));
    assert!(!mention_matches_bot_open_id(&serde_json::json!({"open_id": 42}), "ou_bot"));
}

#[test]
fn normalize_group_reply_sender_ids_trims_sorts_and_deduplicates() {
    assert_eq!(
        normalize_group_reply_allowed_sender_ids(vec![
            " ou_b ".to_string(),
            "".to_string(),
            "ou_a".to_string(),
            "ou_b".to_string(),
        ]),
        vec!["ou_a".to_string(), "ou_b".to_string()]
    );
}

#[test]
fn should_respond_in_group_covers_override_all_messages_and_mention_only() {
    assert!(should_respond_in_group(true, "ou_sender", &["*".to_string()], None, &[], &[]));
    assert!(!should_respond_in_group(true, "", &["*".to_string()], None, &[], &[]));
    assert!(should_respond_in_group(false, "ou_sender", &[], None, &[], &[]));
    assert!(!should_respond_in_group(true, "ou_sender", &[], Some("ou_bot"), &[], &[]));
    assert!(should_respond_in_group(
        true,
        "ou_sender",
        &[],
        Some("ou_bot"),
        &[serde_json::json!({"open_id": "ou_bot"})],
        &[]
    ));
    assert!(should_respond_in_group(
        true,
        "ou_sender",
        &[],
        Some("ou_bot"),
        &[],
        &["ou_bot".to_string()]
    ));
    assert!(should_respond_in_group(
        true,
        " ou_sender ",
        &["ou_sender".to_string()],
        None,
        &[],
        &[]
    ));
    assert!(!should_respond_in_group(
        true,
        "ou_sender",
        &[],
        Some(""),
        &[serde_json::json!({"open_id": "ou_bot"})],
        &[]
    ));
    assert!(!should_respond_in_group(
        true,
        "ou_sender",
        &[],
        Some("ou_bot"),
        &[serde_json::json!({"open_id": "ou_other"})],
        &["ou_other".to_string()]
    ));
}

#[test]
fn parse_event_payload_accepts_post_and_uses_runtime_fallbacks() {
    let channel = test_channel(false);
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let content = serde_json::json!({
        "zh_cn": {
            "content": [[
                { "tag": "text", "text": "hello" },
                { "tag": "at", "user_id": "ou_bot", "user_name": "Bot" }
            ]]
        }
    })
    .to_string();
    let payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "post",
                "content": content,
                "create_time": "not-a-number"
            }
        }
    });

    let messages = channel.parse_event_payload(&payload);
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "hello@Bot");
    assert_eq!(messages[0].sender, "ou_user");
    assert_eq!(messages[0].reply_target, "ou_user");
    assert!((before..=after).contains(&messages[0].timestamp));
}

#[test]
fn parse_event_payload_skips_invalid_post_unsupported_type_and_group_without_bot() {
    let channel = test_channel(true);
    let invalid_post = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "post",
                "content": "not-json",
                "chat_id": "oc_chat"
            }
        }
    });
    let unsupported = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "audio",
                "content": "{}",
                "chat_id": "oc_chat"
            }
        }
    });
    let group_without_bot = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": serde_json::json!({ "text": "hello" }).to_string(),
                "chat_type": "group",
                "chat_id": "oc_chat",
                "mentions": [{ "open_id": "ou_bot" }]
            }
        }
    });

    assert!(channel.parse_event_payload(&invalid_post).is_empty());
    assert!(channel.parse_event_payload(&unsupported).is_empty());
    assert!(channel.parse_event_payload(&group_without_bot).is_empty());
}

#[test]
fn parse_event_payload_rejects_invalid_envelope_sender_and_text() {
    let allowed_channel = test_channel(false);
    let denied_channel = LarkChannel::new(
        "id".to_string(),
        "secret".to_string(),
        "token".to_string(),
        None,
        vec!["ou_allowed".to_string()],
        false,
    );
    let missing_event = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" }
    });
    let empty_sender = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": { "message": { "message_type": "text", "content": "{}" } }
    });
    let invalid_text = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": { "message_type": "text", "content": "not-json" }
        }
    });

    assert!(
        allowed_channel
            .parse_event_payload(&serde_json::json!({"header": {"event_type": "other"}}))
            .is_empty()
    );
    assert!(allowed_channel.parse_event_payload(&missing_event).is_empty());
    assert!(allowed_channel.parse_event_payload(&empty_sender).is_empty());
    assert!(allowed_channel.parse_event_payload(&invalid_text).is_empty());
    assert!(denied_channel.parse_event_payload(&text_payload("hello")).is_empty());
}

#[tokio::test]
async fn parse_event_payload_async_accepts_text_and_post_messages() {
    let channel = test_channel(false);
    let text_messages = channel.parse_event_payload_async(&text_payload("hello")).await;
    let post_content = serde_json::json!({
        "en_us": {
            "title": "Post",
            "content": [[{ "tag": "text", "text": " body" }]]
        }
    })
    .to_string();
    let post_payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "post",
                "content": post_content,
                "chat_id": "oc_chat",
                "create_time": "3000"
            }
        }
    });
    let post_messages = channel.parse_event_payload_async(&post_payload).await;

    assert_eq!(text_messages.len(), 1);
    assert_eq!(text_messages[0].content, "hello");
    assert_eq!(text_messages[0].timestamp, 2);
    assert_eq!(post_messages.len(), 1);
    assert_eq!(post_messages[0].content, "Post\n\n body");
    assert_eq!(post_messages[0].timestamp, 3);
}

#[tokio::test]
async fn parse_event_payload_async_skips_invalid_inputs_without_network() {
    let allowed_channel = test_channel(false);
    let denied_channel = LarkChannel::new(
        "id".to_string(),
        "secret".to_string(),
        "token".to_string(),
        None,
        vec!["ou_allowed".to_string()],
        false,
    );
    let missing_event = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" }
    });
    let empty_sender = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": { "message": { "message_type": "text", "content": "{}" } }
    });
    let unsupported = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": { "message_type": "file", "content": "{}" }
        }
    });
    let invalid_text = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": { "message_type": "text", "content": "not-json" }
        }
    });
    let invalid_post = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": { "message_type": "post", "content": "{}" }
        }
    });
    let unauthorized = text_payload("hello");

    assert!(
        allowed_channel
            .parse_event_payload_async(&serde_json::json!({"header": {"event_type": "other"}}))
            .await
            .is_empty()
    );
    assert!(allowed_channel.parse_event_payload_async(&missing_event).await.is_empty());
    assert!(allowed_channel.parse_event_payload_async(&empty_sender).await.is_empty());
    assert!(allowed_channel.parse_event_payload_async(&unsupported).await.is_empty());
    assert!(allowed_channel.parse_event_payload_async(&invalid_text).await.is_empty());
    assert!(allowed_channel.parse_event_payload_async(&invalid_post).await.is_empty());
    assert!(denied_channel.parse_event_payload_async(&unauthorized).await.is_empty());
}

#[tokio::test]
async fn parse_event_payload_async_handles_image_missing_key_and_group_filter() {
    let channel = test_channel(true);
    let image_payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "image",
                "content": "{}",
                "chat_id": "oc_chat"
            }
        }
    });
    let group_payload = serde_json::json!({
        "header": { "event_type": "im.message.receive_v1" },
        "event": {
            "sender": { "sender_id": { "open_id": "ou_user" } },
            "message": {
                "message_type": "text",
                "content": serde_json::json!({ "text": "hello" }).to_string(),
                "chat_type": "group",
                "chat_id": "oc_chat",
                "mentions": []
            }
        }
    });

    let image_messages = channel.parse_event_payload_async(&image_payload).await;

    assert_eq!(image_messages.len(), 1);
    assert_eq!(image_messages[0].content, LARK_IMAGE_DOWNLOAD_FALLBACK_TEXT);
    assert!(channel.parse_event_payload_async(&group_payload).await.is_empty());
}
