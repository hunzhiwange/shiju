use super::*;
use crate::app::agent::config::{
    GroupReplyConfig, GroupReplyMode, LarkConfig, LarkReceiveMode, ProxyConfig, ProxyScope,
    runtime_proxy_config, set_runtime_proxy_config,
};
use std::time::{Duration, Instant};

fn make_mod_channel() -> LarkChannel {
    LarkChannel::new(
        "cli_test_app_id".into(),
        "test_app_secret".into(),
        "test_verification_token".into(),
        None,
        vec!["ou_testuser123".into()],
        true,
    )
}

async fn cache_token(ch: &LarkChannel, value: &str, refresh_after: Instant) {
    let mut cached = ch.tenant_token.write().await;
    *cached = Some(CachedTenantToken { value: value.to_string(), refresh_after });
}

#[tokio::test]
async fn lark_channel_new_preserves_constructor_fields_and_defaults() {
    let ch = LarkChannel::new(
        "cli_app".to_string(),
        "secret".to_string(),
        "verify".to_string(),
        Some(8848),
        vec!["ou_allowed".to_string()],
        true,
    );

    assert_eq!(ch.app_id, "cli_app");
    assert_eq!(ch.app_secret, "secret");
    assert_eq!(ch.verification_token, "verify");
    assert_eq!(ch.port, Some(8848));
    assert_eq!(ch.allowed_users, vec!["ou_allowed".to_string()]);
    assert!(ch.group_reply_allowed_sender_ids.is_empty());
    assert!(ch.mention_only);
    assert_eq!(ch.receive_mode, LarkReceiveMode::Websocket);
    assert!(ch.tenant_token.read().await.is_none());
    assert!(ch.ws_seen_ids.read().await.is_empty());
}

#[test]
fn lark_channel_from_config_legacy_feishu_preserves_fields_and_group_reply() {
    let cfg = LarkConfig {
        app_id: "cli_feishu".to_string(),
        app_secret: "secret".to_string(),
        encrypt_key: None,
        verification_token: None,
        allowed_users: vec!["ou_allowed".to_string()],
        mention_only: true,
        group_reply: Some(GroupReplyConfig {
            mode: Some(GroupReplyMode::AllMessages),
            allowed_sender_ids: vec![
                " ou_admin ".to_string(),
                "ou_admin".to_string(),
                "".to_string(),
                "ou_ops".to_string(),
            ],
        }),
        use_feishu: true,
        receive_mode: LarkReceiveMode::Webhook,
        port: Some(9898),
        draft_update_interval_ms: 1_000,
        max_draft_edits: 10,
    };

    let ch = LarkChannel::from_config(&cfg);

    assert_eq!(ch.name(), "feishu");
    assert_eq!(ch.api_base(), FEISHU_BASE_URL);
    assert_eq!(ch.ws_base(), FEISHU_WS_BASE_URL);
    assert_eq!(ch.verification_token, "");
    assert_eq!(ch.port, Some(9898));
    assert_eq!(ch.allowed_users, vec!["ou_allowed".to_string()]);
    assert!(!ch.mention_only);
    assert_eq!(
        ch.group_reply_allowed_sender_ids,
        vec!["ou_admin".to_string(), "ou_ops".to_string()]
    );
    assert_eq!(ch.receive_mode, LarkReceiveMode::Webhook);
}

#[tokio::test]
async fn lark_channel_health_check_uses_cached_valid_token() {
    let ch = make_mod_channel();
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;

    assert!(ch.health_check().await);
}

#[tokio::test]
async fn lark_channel_listen_webhook_reports_missing_port() {
    let mut ch = make_mod_channel();
    ch.receive_mode = LarkReceiveMode::Webhook;
    ch.port = None;
    let (tx, _rx) = tokio::sync::mpsc::channel(1);

    let err = ch.listen(tx).await.expect_err("webhook listen without port should fail");

    assert!(err.to_string().contains("requires `port`"));
}

#[tokio::test(flavor = "current_thread")]
async fn lark_channel_send_returns_error_when_proxy_is_unreachable() {
    let _guard =
        crate::test_support::global_runtime_proxy_lock().lock().unwrap_or_else(|e| e.into_inner());
    let original = runtime_proxy_config();
    set_runtime_proxy_config(ProxyConfig {
        enabled: true,
        https_proxy: Some("http://127.0.0.1:9".to_string()),
        scope: ProxyScope::Services,
        services: vec!["channel.lark".to_string()],
        ..ProxyConfig::default()
    });

    let ch = make_mod_channel();
    cache_token(&ch, "cached-token", Instant::now() + Duration::from_secs(60)).await;
    let result =
        tokio::time::timeout(Duration::from_secs(2), ch.send(&SendMessage::new("hello", "chat")))
            .await;

    set_runtime_proxy_config(original);

    assert!(matches!(result, Ok(Err(_))));
}
