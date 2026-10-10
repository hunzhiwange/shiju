// 本文件验证聊天渠道列表顺序、启用判定与旧版飞书配置兼容。

use super::chat::ChannelsConfigExt;
use super::other::{QQConfig, QQReceiveMode};
use super::{
    ChannelsConfig, FeishuConfig, FeishuInstanceConfig, LarkConfig, LarkReceiveMode,
    TelegramConfig, WebhookConfig,
};

fn feishu_config(app_id: &str, app_secret: &str) -> FeishuConfig {
    FeishuConfig {
        app_id: app_id.to_string(),
        app_secret: app_secret.to_string(),
        encrypt_key: None,
        verification_token: None,
        allowed_users: Vec::new(),
        group_reply: None,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    }
}

fn legacy_feishu_config(app_id: &str, app_secret: &str) -> LarkConfig {
    LarkConfig {
        app_id: app_id.to_string(),
        app_secret: app_secret.to_string(),
        encrypt_key: None,
        verification_token: None,
        allowed_users: Vec::new(),
        mention_only: false,
        group_reply: None,
        use_feishu: true,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    }
}

#[test]
fn channels_list_adds_webhook_after_regular_channels() {
    let config = ChannelsConfig::default();
    let regular_len = config.channels_except_webhook().len();
    let all = config.channels();

    assert_eq!(all.len(), regular_len + 1);
    assert_eq!(all.last().unwrap().0.name(), "Webhook");
    assert!(!all.last().unwrap().1);
}


#[test]
fn channels_except_webhook_excludes_configured_webhook() {
    let config = ChannelsConfig {
        webhook: Some(WebhookConfig { port: 8787, secret: Some("secret".to_string()) }),
        ..Default::default()
    };

    assert!(config.channels_except_webhook().iter().all(|(handle, _)| handle.name() != "Webhook"));
}


#[test]
fn blank_feishu_is_not_reported_as_enabled() {
    let config = ChannelsConfig {
        lark: Some(legacy_feishu_config(" ", "")),
        feishu: Some(feishu_config("", "secret")),
        feishu_instances: vec![FeishuInstanceConfig {
            id: "partial".to_string(),
            config: feishu_config("app-id", "\t"),
        }],
        ..Default::default()
    };

    let channels = config.channels_except_webhook();

    assert!(
        channels
            .iter()
            .all(|(handle, enabled)| !matches!(handle.name(), "Lark" | "Feishu") || !*enabled)
    );
}
