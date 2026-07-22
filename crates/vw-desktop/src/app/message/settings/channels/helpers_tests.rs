use super::*;
use vw_config_types::channels::{GroupReplyConfig, GroupReplyMode, LarkReceiveMode, QQReceiveMode};

#[test]
fn default_matrix_config_uses_empty_restrictive_values() {
    let matrix = default_matrix_config();

    assert!(matrix.homeserver.is_empty());
    assert!(matrix.access_token.is_empty());
    assert_eq!(matrix.user_id, None);
    assert_eq!(matrix.device_id, None);
    assert!(matrix.room_id.is_empty());
    assert!(matrix.allowed_users.is_empty());
    assert!(!matrix.mention_only);
}

#[test]
fn default_feishu_config_uses_websocket_and_draft_defaults() {
    let feishu = default_feishu_config();

    assert!(feishu.app_id.is_empty());
    assert!(feishu.app_secret.is_empty());
    assert_eq!(feishu.encrypt_key, None);
    assert_eq!(feishu.verification_token, None);
    assert!(feishu.allowed_users.is_empty());
    assert!(feishu.group_reply.is_none());
    assert_eq!(feishu.receive_mode, LarkReceiveMode::Websocket);
    assert_eq!(feishu.port, None);
    assert_eq!(feishu.draft_update_interval_ms, 3000);
    assert_eq!(feishu.max_draft_edits, 20);
}

#[test]
fn default_dingtalk_config_uses_empty_credentials_and_allowlist() {
    let dingtalk = default_dingtalk_config();

    assert!(dingtalk.client_id.is_empty());
    assert!(dingtalk.client_secret.is_empty());
    assert!(dingtalk.allowed_users.is_empty());
}

#[test]
fn trim_to_option_trims_non_empty_values_and_rejects_whitespace() {
    assert_eq!(trim_to_option(" \t user-1 \n"), Some("user-1".to_string()));
    assert_eq!(trim_to_option("\n\t  "), None);
}

#[test]
fn parse_receive_mode_accepts_normalized_webhook_and_defaults_to_websocket() {
    assert_eq!(parse_receive_mode("  WeBhOoK \n"), LarkReceiveMode::Webhook);
    assert_eq!(parse_receive_mode("websocket"), LarkReceiveMode::Websocket);
    assert_eq!(parse_receive_mode("unsupported"), LarkReceiveMode::Websocket);
}

#[test]
fn parse_qq_receive_mode_accepts_normalized_websocket_and_defaults_to_webhook() {
    assert_eq!(parse_qq_receive_mode("  WeBsOcKeT \n"), QQReceiveMode::Websocket);
    assert_eq!(parse_qq_receive_mode("webhook"), QQReceiveMode::Webhook);
    assert_eq!(parse_qq_receive_mode("unsupported"), QQReceiveMode::Webhook);
}

#[test]
fn set_group_reply_mode_inserts_config_and_selects_each_mode() {
    let mut group_reply = None;
    set_group_reply_mode(&mut group_reply, "  MENTION_ONLY ");

    let reply = group_reply.as_ref().expect("group reply should exist");
    assert_eq!(reply.mode, Some(GroupReplyMode::MentionOnly));
    assert!(reply.allowed_sender_ids.is_empty());

    set_group_reply_mode(&mut group_reply, "all_messages");

    let reply = group_reply.expect("group reply should remain present");
    assert_eq!(reply.mode, Some(GroupReplyMode::AllMessages));
}

#[test]
fn set_group_reply_mode_preserves_existing_allowed_senders() {
    let mut group_reply = Some(GroupReplyConfig {
        mode: None,
        allowed_sender_ids: vec!["existing-user".to_string()],
    });

    set_group_reply_mode(&mut group_reply, "unexpected-mode");

    let reply = group_reply.expect("group reply should remain present");
    assert_eq!(reply.mode, Some(GroupReplyMode::AllMessages));
    assert_eq!(reply.allowed_sender_ids, vec!["existing-user".to_string()]);
}

#[test]
fn set_group_reply_allowed_parses_separators_and_preserves_existing_mode() {
    let mut group_reply = Some(GroupReplyConfig {
        mode: Some(GroupReplyMode::MentionOnly),
        allowed_sender_ids: vec!["old-user".to_string()],
    });

    set_group_reply_allowed(&mut group_reply, " user-1, user-2\n\n user-3, ");

    let reply = group_reply.expect("group reply should exist");
    assert_eq!(reply.mode, Some(GroupReplyMode::MentionOnly));
    assert_eq!(
        reply.allowed_sender_ids,
        vec!["user-1".to_string(), "user-2".to_string(), "user-3".to_string(),]
    );
}

#[test]
fn set_group_reply_allowed_inserts_config_and_accepts_empty_input() {
    let mut group_reply = None;

    set_group_reply_allowed(&mut group_reply, " , \n ");

    let reply = group_reply.expect("group reply should exist");
    assert_eq!(reply.mode, None);
    assert!(reply.allowed_sender_ids.is_empty());
}
