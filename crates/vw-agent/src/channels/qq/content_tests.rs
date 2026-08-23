// 本文件验证 QQ 消息内容、提及与附件信息的规范化处理。

use super::*;

#[test]
fn parse_outgoing_content_extracts_only_remote_image_markers() {
    let (text, images) =
        parse_outgoing_content("hello\n[IMAGE:https://example.com/a.png]\n[IMAGE:/tmp/a.png]");

    assert_eq!(text, "hello\n[IMAGE:/tmp/a.png]");
    assert_eq!(images, vec!["https://example.com/a.png".to_string()]);
}

#[test]
fn parse_outgoing_content_extracts_http_markers_with_whitespace() {
    let (text, images) =
        parse_outgoing_content("before\n  [IMAGE: http://example.com/a.png ]  \nafter");

    assert_eq!(text, "before\nafter");
    assert_eq!(images, vec!["http://example.com/a.png".to_string()]);
}

#[test]
fn compose_message_content_combines_text_and_image_attachments() {
    let payload = serde_json::json!({
        "content": "hello",
        "attachments": [{
            "url": "https://example.com/a.png",
            "content_type": "image/png"
        }]
    });

    assert_eq!(
        compose_message_content(&payload),
        Some("hello\n\n[IMAGE:https://example.com/a.png]".to_string())
    );
}

#[test]
fn build_text_message_body_trims_and_adds_passive_reply_fields() {
    let body = build_text_message_body(" hi ", Some("msg-1"), 7).expect("text body");

    assert_eq!(body["content"], "hi");
    assert_eq!(body["msg_type"], 0);
    assert_eq!(body["msg_id"], "msg-1");
    assert_eq!(body["msg_seq"], 7);
    assert!(build_text_message_body("   ", None, 0).is_none());
}

#[test]
fn compose_message_content_uses_filename_when_content_type_is_missing() {
    let payload = serde_json::json!({
        "content": "  ",
        "attachments": [
            {"filename": "photo.WEBP", "url": " https://example.com/p.webp "},
            {"filename": "notes.txt", "url": "https://example.com/notes.txt"},
            {"content_type": "image/svg+xml", "url": ""}
        ]
    });

    assert_eq!(
        compose_message_content(&payload),
        Some("[IMAGE:https://example.com/p.webp]".to_string())
    );
}

#[test]
fn compose_message_content_ignores_attachments_without_string_url() {
    let payload = serde_json::json!({
        "content": "body",
        "attachments": [
            {"filename": "photo.png"},
            {"filename": "photo.jpg", "url": 7},
            {"filename": "photo.gif", "url": "https://example.com/photo.gif"}
        ]
    });

    assert_eq!(
        compose_message_content(&payload),
        Some("body\n\n[IMAGE:https://example.com/photo.gif]".to_string())
    );
}

#[test]
// 文本与有效附件都为空时不构造伪消息。
fn compose_message_content_returns_none_for_empty_payload() {
    assert_eq!(compose_message_content(&serde_json::json!({})), None);
    assert_eq!(
        compose_message_content(&serde_json::json!({
            "content": " ",
            "attachments": [{"content_type": "application/pdf", "url": "https://e.test/a.pdf"}]
        })),
        None
    );
}

#[test]
fn parse_outgoing_content_trims_result_and_preserves_malformed_markers() {
    let (text, images) =
        parse_outgoing_content("  \n[IMAGE:]\n[IMAGE:https://example.com/a.png]\n tail  ");

    assert_eq!(text, "[IMAGE:]\n tail");
    assert_eq!(images, vec!["https://example.com/a.png".to_string()]);
}

#[test]
fn build_text_message_body_omits_passive_fields_when_no_message_id() {
    let body = build_text_message_body("hello", None, 99).expect("text body");

    assert_eq!(body, serde_json::json!({"content": "hello", "msg_type": 0}));
}

#[test]
fn build_media_message_body_omits_passive_fields_when_no_message_id() {
    let body = build_media_message_body("file-info", None, 99);

    assert_eq!(
        body,
        serde_json::json!({
            "content": " ",
            "msg_type": 7,
            "media": {"file_info": "file-info"}
        })
    );
}

#[test]
fn build_media_message_body_adds_passive_reply_fields() {
    let body = build_media_message_body("file-info", Some("msg-1"), 42);

    assert_eq!(
        body,
        serde_json::json!({
            "content": " ",
            "msg_type": 7,
            "media": {"file_info": "file-info"},
            "msg_id": "msg-1",
            "msg_seq": 42
        })
    );
}

#[test]
fn build_channel_message_sets_reply_metadata_and_thread_conditionally() {
    let with_thread = build_channel_message(
        "sender-1",
        "user:sender-1".to_string(),
        "hello".to_string(),
        "msg-1",
    );
    assert_eq!(with_thread.sender, "sender-1");
    assert_eq!(with_thread.reply_target, "user:sender-1");
    assert_eq!(with_thread.content, "hello");
    assert_eq!(with_thread.channel, "qq");
    assert_eq!(with_thread.thread_ts.as_deref(), Some("msg-1"));
    assert!(!with_thread.id.is_empty());

    let without_thread =
        build_channel_message("sender-2", "group:g".to_string(), "hi".to_string(), "");
    assert_eq!(without_thread.thread_ts, None);
}
