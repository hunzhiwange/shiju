//! 飞书/Lark 通道测试入口。

use super::*;

#[path = "tests/ack_tests.rs"]
mod ack_tests;
#[path = "tests/config_tests.rs"]
mod config_tests;
#[path = "tests/parsing_tests.rs"]
mod parsing_tests;
#[path = "tests/token_tests.rs"]
mod token_tests;
#[path = "tests/ws_tests.rs"]
mod ws_tests;

pub(super) fn with_bot_open_id(ch: LarkChannel, bot_open_id: &str) -> LarkChannel {
    ch.set_resolved_bot_open_id(Some(bot_open_id.to_string()));
    ch
}

pub(super) fn make_channel() -> LarkChannel {
    with_bot_open_id(
        LarkChannel::new(
            "cli_test_app_id".into(),
            "test_app_secret".into(),
            "test_verification_token".into(),
            None,
            vec!["ou_testuser123".into()],
            true,
        ),
        "ou_bot",
    )
}

#[test]
fn lark_channel_new_uses_lark_defaults_and_private_url_helpers() {
    let ch = make_channel();

    assert_eq!(ch.name(), "lark");
    assert_eq!(ch.api_base(), LARK_BASE_URL);
    assert_eq!(ch.ws_base(), LARK_WS_BASE_URL);
    assert_eq!(
        ch.tenant_access_token_url(),
        format!("{LARK_BASE_URL}/auth/v3/tenant_access_token/internal")
    );
    assert_eq!(ch.bot_info_url(), format!("{LARK_BASE_URL}/bot/v3/info"));
    assert_eq!(
        ch.send_message_url(),
        format!("{LARK_BASE_URL}/im/v1/messages?receive_id_type=chat_id")
    );
    assert_eq!(ch.image_download_url("img_v2"), format!("{LARK_BASE_URL}/im/v1/images/img_v2"));
}

#[test]
fn lark_channel_feishu_config_uses_feishu_urls_and_name() {
    let cfg = config_channel_services::FeishuConfig {
        app_id: "cli_feishu".to_string(),
        app_secret: "secret".to_string(),
        encrypt_key: None,
        verification_token: Some("verify".to_string()),
        allowed_users: vec!["ou_allowed".to_string()],
        group_reply: None,
        receive_mode: config_channel_services::LarkReceiveMode::Webhook,
        port: Some(8080),
        draft_update_interval_ms: 1_000,
        max_draft_edits: 10,
    };

    let ch = LarkChannel::from_feishu_config(&cfg);

    assert_eq!(ch.name(), "feishu");
    assert_eq!(ch.api_base(), FEISHU_BASE_URL);
    assert_eq!(ch.ws_base(), FEISHU_WS_BASE_URL);
    assert_eq!(
        ch.message_reaction_url("om_1"),
        format!("{FEISHU_BASE_URL}/im/v1/messages/om_1/reactions")
    );
}

#[test]
fn lark_channel_allowlist_and_bot_open_id_state_are_exact() {
    let ch = LarkChannel::new(
        "app".to_string(),
        "secret".to_string(),
        "verify".to_string(),
        None,
        vec!["ou_allowed".to_string()],
        true,
    );

    assert!(ch.is_user_allowed("ou_allowed"));
    assert!(!ch.is_user_allowed("ou_allowed_extra"));
    assert_eq!(ch.resolved_bot_open_id(), None);

    ch.set_resolved_bot_open_id(Some("ou_bot".to_string()));
    assert_eq!(ch.resolved_bot_open_id().as_deref(), Some("ou_bot"));

    ch.set_resolved_bot_open_id(None);
    assert_eq!(ch.resolved_bot_open_id(), None);
}

#[test]
fn lark_channel_wildcard_allowlist_allows_any_user_id() {
    let ch = LarkChannel::new(
        "app".to_string(),
        "secret".to_string(),
        "verify".to_string(),
        None,
        vec!["*".to_string()],
        false,
    );

    assert!(ch.is_user_allowed("ou_anyone"));
    assert!(ch.is_user_allowed(""));
}
