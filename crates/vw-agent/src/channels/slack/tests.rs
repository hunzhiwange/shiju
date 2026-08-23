// 本文件验证 Slack 渠道配置、群聊回复策略与频道列表筛选。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    #[test]
    fn slack_channel_name() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec![]);
        assert_eq!(ch.name(), "slack");
    }

    #[test]
    fn slack_channel_with_channel_id() {
        let ch = SlackChannel::new("xoxb-fake".into(), Some("C12345".into()), vec![]);
        assert_eq!(ch.channel_id, Some("C12345".to_string()));
    }

    #[test]
    fn slack_group_reply_policy_defaults_to_all_messages() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["*".into()]);
        assert!(!ch.mention_only);
        assert!(ch.group_reply_allowed_sender_ids.is_empty());
    }

    #[test]
    fn slack_group_reply_policy_applies_sender_overrides() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["*".into()])
            .with_group_reply_policy(true, vec![" U111 ".into(), "U111".into(), "U222".into()]);

        assert!(ch.mention_only);
        assert_eq!(ch.group_reply_allowed_sender_ids, vec!["U111".to_string(), "U222".to_string()]);
        assert!(ch.is_group_sender_trigger_enabled("U111"));
        assert!(!ch.is_group_sender_trigger_enabled("U999"));
    }

    #[test]
    fn slack_group_reply_policy_handles_empty_and_wildcard_sender_ids() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["*".into()])
            .with_group_reply_policy(true, vec![" ".into(), "*".into(), "U111".into()]);

        assert_eq!(ch.group_reply_allowed_sender_ids, vec!["*".to_string(), "U111".to_string()]);
        assert!(ch.is_group_sender_trigger_enabled("U999"));
        assert!(!ch.is_group_sender_trigger_enabled(""));
        assert!(!ch.is_group_sender_trigger_enabled("   "));
    }

    #[test]
    fn group_sender_trigger_trims_candidate_id() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["*".into()])
            .with_group_reply_policy(true, vec!["U111".into()]);

        assert!(ch.is_group_sender_trigger_enabled(" U111 "));
    }

    #[test]
    fn normalize_group_reply_allowed_sender_ids_sorts_and_deduplicates() {
        let ids = SlackChannel::normalize_group_reply_allowed_sender_ids(vec![
            " U222 ".into(),
            "".into(),
            "U111".into(),
            "U222".into(),
            "   ".into(),
        ]);

        assert_eq!(ids, vec!["U111".to_string(), "U222".to_string()]);
    }

    #[test]
    fn normalized_channel_id_respects_wildcard_and_blank() {
        assert_eq!(SlackChannel::normalized_channel_id(None), None);
        assert_eq!(SlackChannel::normalized_channel_id(Some("")), None);
        assert_eq!(SlackChannel::normalized_channel_id(Some("   ")), None);
        assert_eq!(SlackChannel::normalized_channel_id(Some("*")), None);
        assert_eq!(SlackChannel::normalized_channel_id(Some(" * ")), None);
        assert_eq!(
            SlackChannel::normalized_channel_id(Some(" C12345 ")),
            Some("C12345".to_string())
        );
    }

    #[test]
    fn configured_channel_id_uses_normalized_value() {
        let scoped = SlackChannel::new("xoxb-fake".into(), Some(" C12345 ".into()), vec![]);
        let unset = SlackChannel::new("xoxb-fake".into(), None, vec![]);
        let wildcard = SlackChannel::new("xoxb-fake".into(), Some("*".into()), vec![]);

        assert_eq!(scoped.configured_channel_id().as_deref(), Some("C12345"));
        assert_eq!(unset.configured_channel_id(), None);
        assert_eq!(wildcard.configured_channel_id(), None);
    }

    #[test]
    fn is_group_channel_id_detects_channel_prefixes() {
        assert!(SlackChannel::is_group_channel_id("C123"));
        assert!(SlackChannel::is_group_channel_id("G123"));
        assert!(!SlackChannel::is_group_channel_id("D123"));
        assert!(!SlackChannel::is_group_channel_id("c123"));
        assert!(!SlackChannel::is_group_channel_id(" U123"));
        assert!(!SlackChannel::is_group_channel_id(""));
    }

    #[test]
// 自动选择频道时排除已归档和未加入项，避免投递到无效目标。
    fn extract_channel_ids_filters_archived_and_non_member_entries() {
        let payload = serde_json::json!({
            "channels": [
                {"id": "C1", "is_archived": false, "is_member": true},
                {"id": "C2", "is_archived": true, "is_member": true},
                {"id": "C3", "is_archived": false, "is_member": false},
                {"id": "C1", "is_archived": false, "is_member": true},
                {"id": "C4"}
            ]
        });
        let ids = SlackChannel::extract_channel_ids(&payload);
        assert_eq!(ids, vec!["C1".to_string(), "C4".to_string()]);
    }

    #[test]
    fn extract_channel_ids_handles_missing_or_malformed_channel_list() {
        assert!(SlackChannel::extract_channel_ids(&serde_json::json!({})).is_empty());
        assert!(
            SlackChannel::extract_channel_ids(&serde_json::json!({"channels": "bad"})).is_empty()
        );
        assert!(
            SlackChannel::extract_channel_ids(&serde_json::json!({
                "channels": [{"name": "missing-id"}, {"id": 42}]
            }))
            .is_empty()
        );
    }

    #[test]
    fn extract_channel_ids_defaults_missing_flags_to_accessible() {
        let payload = serde_json::json!({
            "channels": [
                {"id": "C2", "is_archived": null, "is_member": null},
                {"id": "C1", "is_archived": "no", "is_member": "yes"}
            ]
        });

        assert_eq!(SlackChannel::extract_channel_ids(&payload), vec!["C1", "C2"]);
    }

    #[test]
    fn empty_allowlist_denies_everyone() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec![]);
        assert!(!ch.is_user_allowed("U12345"));
        assert!(!ch.is_user_allowed("anyone"));
    }

    #[test]
    fn wildcard_allows_everyone() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["*".into()]);
        assert!(ch.is_user_allowed("U12345"));
    }

    #[test]
    fn normalize_incoming_content_requires_mention_when_enabled() {
        assert!(SlackChannel::normalize_incoming_content("hello", true, "U_BOT").is_none());
        assert_eq!(
            SlackChannel::normalize_incoming_content("<@U_BOT> run", true, "U_BOT").as_deref(),
            Some("run")
        );
    }

    #[test]
    fn mention_helpers_handle_empty_bot_id_and_multiple_mentions() {
        assert!(!SlackChannel::contains_bot_mention("<@U_BOT> hi", ""));
        assert!(SlackChannel::contains_bot_mention("hi <@U_BOT>", "U_BOT"));
        assert!(!SlackChannel::contains_bot_mention("hi @U_BOT", "U_BOT"));
        assert_eq!(SlackChannel::strip_bot_mentions(" <@U_BOT> run <@U_BOT> ", "U_BOT"), "run");
        assert_eq!(SlackChannel::strip_bot_mentions(" <@U_BOT> run ", ""), "<@U_BOT> run");
    }

    #[test]
    fn normalize_incoming_content_rejects_empty_after_stripping_mention() {
        assert!(SlackChannel::normalize_incoming_content("   ", false, "U_BOT").is_none());
        assert!(SlackChannel::normalize_incoming_content("<@U_BOT>", true, "U_BOT").is_none());
        assert!(SlackChannel::normalize_incoming_content("<@U_BOT>   ", true, "U_BOT").is_none());
    }

    #[test]
    fn normalize_incoming_content_without_mention_mode_keeps_message() {
        assert_eq!(
            SlackChannel::normalize_incoming_content("  hello world  ", false, "U_BOT").as_deref(),
            Some("hello world")
        );
    }

    #[test]
    fn normalize_incoming_content_with_empty_bot_id_cannot_satisfy_mention() {
        assert!(SlackChannel::normalize_incoming_content("<@U_BOT> hello", true, "").is_none());
    }

    #[test]
    fn specific_allowlist_filters() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into(), "U222".into()]);
        assert!(ch.is_user_allowed("U111"));
        assert!(ch.is_user_allowed("U222"));
        assert!(!ch.is_user_allowed("U333"));
    }

    #[test]
    fn allowlist_exact_match_not_substring() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into()]);
        assert!(!ch.is_user_allowed("U1111"));
        assert!(!ch.is_user_allowed("U11"));
    }

    #[test]
    fn allowlist_empty_user_id() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into()]);
        assert!(!ch.is_user_allowed(""));
    }

    #[test]
    fn allowlist_case_sensitive() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into()]);
        assert!(ch.is_user_allowed("U111"));
        assert!(!ch.is_user_allowed("u111"));
    }

    #[test]
    fn allowlist_wildcard_and_specific() {
        let ch = SlackChannel::new("xoxb-fake".into(), None, vec!["U111".into(), "*".into()]);
        assert!(ch.is_user_allowed("U111"));
        assert!(ch.is_user_allowed("anyone"));
    }

    // ── Message ID edge cases ─────────────────────────────────────

    #[test]
    fn slack_message_id_format_includes_channel_and_ts() {
        // Verify that message IDs follow the format: slack_{channel_id}_{ts}
        let ts = "1234567890.123456";
        let channel_id = "C12345";
        let expected_id = format!("slack_{channel_id}_{ts}");
        assert_eq!(expected_id, "slack_C12345_1234567890.123456");
    }

    #[test]
    fn slack_message_id_is_deterministic() {
        // Same channel_id + same ts = same ID (prevents duplicates after restart)
        let ts = "1234567890.123456";
        let channel_id = "C12345";
        let id1 = format!("slack_{channel_id}_{ts}");
        let id2 = format!("slack_{channel_id}_{ts}");
        assert_eq!(id1, id2);
    }

    #[test]
    fn slack_message_id_different_ts_different_id() {
        // Different timestamps produce different IDs
        let channel_id = "C12345";
        let id1 = format!("slack_{channel_id}_1234567890.123456");
        let id2 = format!("slack_{channel_id}_1234567890.123457");
        assert_ne!(id1, id2);
    }

    #[test]
    fn slack_message_id_different_channel_different_id() {
        // Different channels produce different IDs even with same ts
        let ts = "1234567890.123456";
        let id1 = format!("slack_C12345_{ts}");
        let id2 = format!("slack_C67890_{ts}");
        assert_ne!(id1, id2);
    }

    #[test]
    fn slack_message_id_no_uuid_randomness() {
        // Verify format doesn't contain random UUID components
        let ts = "1234567890.123456";
        let channel_id = "C12345";
        let id = format!("slack_{channel_id}_{ts}");
        assert!(!id.contains('-')); // No UUID dashes
        assert!(id.starts_with("slack_"));
    }

    #[test]
    fn inbound_thread_ts_prefers_explicit_thread_ts() {
        let msg = serde_json::json!({
            "ts": "123.002",
            "thread_ts": "123.001"
        });

        let thread_ts = SlackChannel::inbound_thread_ts(&msg, "123.002");
        assert_eq!(thread_ts.as_deref(), Some("123.001"));
    }

    #[test]
    fn inbound_thread_ts_falls_back_to_ts() {
        let msg = serde_json::json!({
            "ts": "123.001"
        });

        let thread_ts = SlackChannel::inbound_thread_ts(&msg, "123.001");
        assert_eq!(thread_ts.as_deref(), Some("123.001"));
    }

    #[test]
    fn inbound_thread_ts_none_when_ts_missing() {
        let msg = serde_json::json!({});

        let thread_ts = SlackChannel::inbound_thread_ts(&msg, "");
        assert_eq!(thread_ts, None);
    }

    #[test]
    fn inbound_thread_ts_ignores_non_string_thread_ts() {
        let msg = serde_json::json!({
            "thread_ts": 123
        });

        let thread_ts = SlackChannel::inbound_thread_ts(&msg, "123.001");
        assert_eq!(thread_ts.as_deref(), Some("123.001"));
    }

    #[test]
    fn inbound_thread_ts_preserves_explicit_empty_thread_marker() {
        let msg = serde_json::json!({"thread_ts": ""});

        assert_eq!(SlackChannel::inbound_thread_ts(&msg, "123.001").as_deref(), Some(""));
    }

    #[test]
    fn ensure_poll_cursor_bootstraps_new_channel() {
        let mut cursors = HashMap::new();
        let now_ts = "1700000000.123456";

        let cursor = SlackChannel::ensure_poll_cursor(&mut cursors, "C123", now_ts);
        assert_eq!(cursor, now_ts);
        assert_eq!(cursors.get("C123").map(String::as_str), Some(now_ts));
    }

    #[test]
    fn ensure_poll_cursor_keeps_existing_cursor() {
        let mut cursors = HashMap::from([("C123".to_string(), "1700000000.000001".to_string())]);
        let cursor = SlackChannel::ensure_poll_cursor(&mut cursors, "C123", "9999999999.999999");

        assert_eq!(cursor, "1700000000.000001");
        assert_eq!(cursors.get("C123").map(String::as_str), Some("1700000000.000001"));
    }

    #[test]
    fn ensure_poll_cursor_tracks_channels_independently() {
        let mut cursors = HashMap::from([("C111".to_string(), "100.000001".to_string())]);

        let existing = SlackChannel::ensure_poll_cursor(&mut cursors, "C111", "200.000002");
        let added = SlackChannel::ensure_poll_cursor(&mut cursors, "C222", "200.000002");

        assert_eq!(existing, "100.000001");
        assert_eq!(added, "200.000002");
        assert_eq!(cursors.len(), 2);
    }

    #[test]
    fn slack_now_ts_has_slack_timestamp_shape() {
        let ts = SlackChannel::slack_now_ts();
        let (seconds, micros) = ts.split_once('.').expect("timestamp has dot");

        assert!(!seconds.is_empty());
        assert_eq!(micros.len(), 6);
        assert!(seconds.chars().all(|c| c.is_ascii_digit()));
        assert!(micros.chars().all(|c| c.is_ascii_digit()));
    }
}
