use super::*;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use std::collections::{HashSet, VecDeque};

fn channel() -> MatrixChannel {
    MatrixChannel::new_with_session_hint_and_vibewindow_dir(
        "https://matrix.example/".to_string(),
        "  token  ".to_string(),
        "  !room:matrix.example  ".to_string(),
        vec![" @Alice:matrix.example ".to_string(), "".to_string()],
        Some(" @bot:matrix.example ".to_string()),
        Some(" DEVICE ".to_string()),
        Some(PathBuf::from("/tmp/vw")),
    )
}

fn parse_event(value: serde_json::Value) -> OriginalSyncRoomMessageEvent {
    serde_json::from_value(value).expect("valid Matrix room message event")
}

#[test]
fn constructors_normalize_core_fields_and_session_hints() {
    let ch = channel();

    assert_eq!(ch.homeserver, "https://matrix.example");
    assert_eq!(ch.access_token, "token");
    assert_eq!(ch.room_id, "!room:matrix.example");
    assert_eq!(ch.allowed_users, vec!["@Alice:matrix.example"]);
    assert_eq!(ch.session_owner_hint.as_deref(), Some("@bot:matrix.example"));
    assert_eq!(ch.session_device_id_hint.as_deref(), Some("DEVICE"));
    assert_eq!(
        ch.matrix_store_dir(),
        Some(PathBuf::from("/tmp/vw/state/matrix/%40bot%3Amatrix.example"))
    );
}

#[test]
fn normalize_room_reference_handles_alias_permalinks_and_invalid_escapes() {
    assert_eq!(
        MatrixChannel::normalize_room_reference(
            " https://matrix.to/#/%23room%3Amatrix.example?via=matrix.example "
        ),
        "#room:matrix.example"
    );
    assert_eq!(
        MatrixChannel::normalize_room_reference("room:matrix.example"),
        "#room:matrix.example"
    );
    assert_eq!(MatrixChannel::normalize_room_reference("https://matrix.to/#/%ZZroom"), "%ZZroom");
    assert_eq!(MatrixChannel::normalize_room_reference("https://matrix.to/#/%FF"), "\u{FFFD}");
}

#[test]
fn channel_instance_matrix_store_dir_is_scoped_by_instance_id_and_user() {
    let ch = MatrixChannel::new_with_session_hint_instance_id_and_vibewindow_dir(
        "https://matrix.example".to_string(),
        "token".to_string(),
        "!room:matrix.example".to_string(),
        Vec::new(),
        Some("@bot:matrix.example".to_string()),
        Some("DEVICE".to_string()),
        Some("planner".to_string()),
        Some(PathBuf::from("/tmp/vw")),
    );

    assert_eq!(
        ch.matrix_store_dir(),
        Some(PathBuf::from("/tmp/vw/state/matrix/planner/%40bot%3Amatrix.example"))
    );
}

#[test]
fn matrix_store_dir_for_user_id_scopes_default_instance_by_resolved_user() {
    let ch = MatrixChannel::new_with_session_hint_instance_id_and_vibewindow_dir(
        "http://localhost".to_string(),
        "token".to_string(),
        "!room:localhost".to_string(),
        Vec::new(),
        None,
        Some("DEVICE".to_string()),
        Some(vw_config_types::channels::chat::DEFAULT_CHANNEL_INSTANCE_ID.to_string()),
        Some(PathBuf::from("/tmp/vw")),
    );

    assert_eq!(
        ch.matrix_store_dir_for_user_id(" @vibewindow-bot:localhost "),
        Some(PathBuf::from("/tmp/vw/state/matrix/%40vibewindow-bot%3Alocalhost"))
    );
}

#[test]
fn default_constructor_uses_empty_session_hints_and_store_dir() {
    let ch = MatrixChannel::new(
        "https://matrix.example/".to_string(),
        " token ".to_string(),
        " !room:matrix.example ".to_string(),
        vec![" @user:matrix.example ".to_string()],
    );

    assert_eq!(ch.homeserver, "https://matrix.example");
    assert_eq!(ch.access_token, "token");
    assert_eq!(ch.room_id, "!room:matrix.example");
    assert_eq!(ch.allowed_users, vec!["@user:matrix.example"]);
    assert!(ch.session_owner_hint.is_none());
    assert!(ch.session_device_id_hint.is_none());
    assert!(ch.matrix_store_dir().is_none());
}

#[test]
fn empty_session_hints_and_missing_store_dir_are_ignored() {
    let ch = MatrixChannel::new_with_session_hint(
        "https://matrix.example".to_string(),
        "token".to_string(),
        "!room:matrix.example".to_string(),
        Vec::new(),
        Some("   ".to_string()),
        Some(String::new()),
    );

    assert!(ch.session_owner_hint.is_none());
    assert!(ch.session_device_id_hint.is_none());
    assert!(ch.matrix_store_dir().is_none());
}

#[test]
fn builders_apply_enabled_transcription_and_mention_only_flags() {
    let mut enabled = crate::app::agent::config::TranscriptionConfig::default();
    enabled.enabled = true;

    assert!(channel().with_transcription(enabled).transcription.is_some());
    assert!(
        channel()
            .with_transcription(crate::app::agent::config::TranscriptionConfig::default())
            .transcription
            .is_none()
    );
    assert!(channel().with_mention_only(true).mention_only);
    assert!(!channel().with_mention_only(false).mention_only);
}

#[test]
fn sanitize_error_for_log_redacts_details_but_keeps_type() {
    let err = anyhow::anyhow!("secret token should not appear");

    let sanitized = MatrixChannel::sanitize_error_for_log(&err);

    assert!(sanitized.contains("details redacted"));
    assert!(!sanitized.contains("secret token"));
}

#[test]
fn otk_conflict_detection_is_case_insensitive_and_requires_both_phrases() {
    assert!(MatrixChannel::is_otk_conflict_message("ONE TIME KEY abc ALREADY EXISTS"));
    assert!(!MatrixChannel::is_otk_conflict_message("one time key upload failed"));
    assert!(!MatrixChannel::is_otk_conflict_message("already exists"));
}

#[test]
fn otk_conflict_recovery_message_includes_store_dir_when_available() {
    let message = channel().otk_conflict_recovery_message();

    assert!(message.contains("one-time key upload conflict"));
    assert!(message.contains("/tmp/vw/state/matrix/%40bot%3Amatrix.example"));
}

#[test]
fn otk_conflict_recovery_message_omits_store_dir_when_unavailable() {
    let ch = MatrixChannel::new(
        "https://matrix.example".to_string(),
        "token".to_string(),
        "!room:matrix.example".to_string(),
        Vec::new(),
    );

    let message = ch.otk_conflict_recovery_message();

    assert!(message.contains("one-time key upload conflict"));
    assert!(!message.contains("Local crypto store"));
}

#[test]
fn encode_path_segment_percent_encodes_reserved_and_unicode_bytes() {
    assert_eq!(MatrixChannel::encode_path_segment("abc-._~XYZ"), "abc-._~XYZ");
    assert_eq!(
        MatrixChannel::encode_path_segment("#room:matrix.example/汉"),
        "%23room%3Amatrix.example%2F%E6%B1%89"
    );
}

#[test]
fn auth_header_value_uses_trimmed_token() {
    assert_eq!(channel().auth_header_value(), "Bearer token");
}

#[test]
fn is_sender_allowed_supports_wildcard_and_case_insensitive_match() {
    assert!(MatrixChannel::is_sender_allowed(&["*".to_string()], "@any:server"));
    assert!(MatrixChannel::is_sender_allowed(
        &["@Alice:matrix.example".to_string()],
        "@alice:matrix.example"
    ));
    assert!(!MatrixChannel::is_sender_allowed(
        &["@Alice:matrix.example".to_string()],
        "@bob:matrix.example"
    ));
    assert!(channel().is_user_allowed("@alice:matrix.example"));
}

#[test]
fn message_type_and_body_filters_match_supported_inputs() {
    assert!(MatrixChannel::is_supported_message_type("m.text"));
    assert!(MatrixChannel::is_supported_message_type("m.notice"));
    assert!(MatrixChannel::is_supported_message_type("m.audio"));
    assert!(!MatrixChannel::is_supported_message_type("m.image"));

    assert!(MatrixChannel::has_non_empty_body(" hello "));
    assert!(!MatrixChannel::has_non_empty_body(" \n\t "));
}

#[test]
fn event_mentions_user_detects_structured_mentions_case_insensitively() {
    let event = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$event:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "hello bot",
            "m.mentions": {
                "user_ids": ["@Bot:Matrix.Example"]
            }
        }
    }));

    assert!(MatrixChannel::event_mentions_user(&event, "hello bot", "@bot:matrix.example"));
}

#[test]
fn event_mentions_user_requires_plain_text_identifier_boundaries() {
    let event = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$event:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "hello"
        }
    }));

    assert!(MatrixChannel::event_mentions_user(
        &event,
        "Hello, @BOT:matrix.example!",
        "@bot:matrix.example"
    ));
    assert!(!MatrixChannel::event_mentions_user(
        &event,
        "prefix@bot:matrix.example suffix @bot:matrix.examplex",
        "@bot:matrix.example"
    ));
    assert!(!MatrixChannel::event_mentions_user(&event, "", "@bot:matrix.example"));
    assert!(!MatrixChannel::event_mentions_user(&event, "hello", ""));
}

#[test]
fn event_mentions_user_detects_formatted_and_encoded_user_ids() {
    let formatted = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$formatted:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "hello bot",
            "format": "org.matrix.custom.html",
            "formatted_body": "<strong>@bot:matrix.example</strong>"
        }
    }));
    let encoded = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$encoded:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "hello bot",
            "format": "org.matrix.custom.html",
            "formatted_body": "<a href=\"https://matrix.to/#/%40bot%3Amatrix.example\">bot</a>"
        }
    }));

    assert!(MatrixChannel::event_mentions_user(&formatted, "hello bot", "@bot:matrix.example"));
    assert!(MatrixChannel::event_mentions_user(&encoded, "hello bot", "@bot:matrix.example"));
}

#[test]
fn event_mentions_user_reads_notice_and_emote_formatted_bodies() {
    for msgtype in ["m.notice", "m.emote"] {
        let event = parse_event(serde_json::json!({
            "type": "m.room.message",
            "event_id": format!("${msgtype}:matrix.example"),
            "sender": "@user:matrix.example",
            "origin_server_ts": 1u64,
            "content": {
                "msgtype": msgtype,
                "body": "hello bot",
                "format": "org.matrix.custom.html",
                "formatted_body": "<span>@bot:matrix.example</span>"
            }
        }));

        assert!(MatrixChannel::event_mentions_user(&event, "hello bot", "@bot:matrix.example"));
    }
}

#[test]
fn event_mentions_user_returns_false_for_custom_message_without_match() {
    let event = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$custom:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "org.example.custom",
            "body": "hello bot"
        }
    }));

    assert!(!MatrixChannel::event_mentions_user(&event, "hello bot", "@bot:matrix.example"));
}

#[test]
fn reply_target_event_id_handles_thread_and_non_reply_relations() {
    let reply = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$reply:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "reply",
            "m.relates_to": {
                "m.in_reply_to": {
                    "event_id": "$bot:matrix.example"
                }
            }
        }
    }));
    let thread = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$thread:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "thread",
            "m.relates_to": {
                "rel_type": "m.thread",
                "event_id": "$root:matrix.example",
                "m.in_reply_to": {
                    "event_id": "$thread-reply:matrix.example"
                }
            }
        }
    }));
    let replacement = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$edit:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": " * edited",
            "m.new_content": {
                "msgtype": "m.text",
                "body": "edited"
            },
            "m.relates_to": {
                "rel_type": "m.replace",
                "event_id": "$old:matrix.example"
            }
        }
    }));
    let no_relation = parse_event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$plain:matrix.example",
        "sender": "@user:matrix.example",
        "origin_server_ts": 1u64,
        "content": {
            "msgtype": "m.text",
            "body": "plain"
        }
    }));

    assert_eq!(
        MatrixChannel::reply_target_event_id(&reply).as_deref(),
        Some("$bot:matrix.example")
    );
    assert_eq!(
        MatrixChannel::reply_target_event_id(&thread).as_deref(),
        Some("$thread-reply:matrix.example")
    );
    assert!(MatrixChannel::reply_target_event_id(&replacement).is_none());
    assert!(MatrixChannel::reply_target_event_id(&no_relation).is_none());
}

#[test]
fn should_process_message_respects_mention_only_gate() {
    assert!(MatrixChannel::should_process_message(false, false, false, false));
    assert!(MatrixChannel::should_process_message(true, true, false, false));
    assert!(MatrixChannel::should_process_message(true, false, true, false));
    assert!(MatrixChannel::should_process_message(true, false, false, true));
    assert!(!MatrixChannel::should_process_message(true, false, false, false));
}

#[test]
fn cache_event_id_detects_duplicates_and_evicts_oldest_entries() {
    let mut order = VecDeque::new();
    let mut lookup = HashSet::new();

    assert!(!MatrixChannel::cache_event_id("$event", &mut order, &mut lookup));
    assert!(MatrixChannel::cache_event_id("$event", &mut order, &mut lookup));

    for idx in 0..2050 {
        MatrixChannel::cache_event_id(&format!("${idx}"), &mut order, &mut lookup);
    }

    assert!(order.len() <= 2048);
    assert!(!lookup.contains("$event"));
}

#[test]
fn sync_filter_for_room_clamps_timeline_limit_to_at_least_one() {
    let filter = MatrixChannel::sync_filter_for_room("!room:matrix.example", 0);
    let parsed: serde_json::Value = serde_json::from_str(&filter).unwrap();

    assert_eq!(parsed["room"]["rooms"][0], "!room:matrix.example");
    assert_eq!(parsed["room"]["timeline"]["limit"], 1);
}
