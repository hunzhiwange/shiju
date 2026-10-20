// 本文件验证聊天渠道列表顺序、启用判定与旧版飞书配置兼容。

use super::chat::ChannelsConfigExt;
use super::other::{QQConfig, QQReceiveMode};
use super::{
    ChannelsConfig, DingTalkConfig, FeishuConfig, FeishuInstanceConfig, LarkConfig,
    LarkReceiveMode, TelegramConfig, WebhookConfig,
};
use vw_config_types::channels::{WeChatConfig, WeComConfig};

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

#[test]
fn channels_except_webhook_reports_enterprise_channels_with_runtime_eligibility() {
    let config = ChannelsConfig {
        feishu: Some(feishu_config("app-id", "app-secret")),
        dingtalk: Some(DingTalkConfig {
            client_id: "client-id".to_string(),
            client_secret: "client-secret".to_string(),
            allowed_users: Vec::new(),
        }),
        wecom: Some(WeComConfig { bot_id: "bot-id".to_string(), secret: "secret".to_string() }),
        wechat: Some(WeChatConfig {
            bot_token: "bot-token".to_string(),
            base_url: "https://ilinkai.weixin.qq.com".to_string(),
        }),
        qq: Some(QQConfig {
            app_id: "app-id".to_string(),
            app_secret: "app-secret".to_string(),
            allowed_users: Vec::new(),
            receive_mode: QQReceiveMode::Websocket,
        }),
        ..Default::default()
    };

    let channels = config.channels_except_webhook();
    for name in ["Feishu", "DingTalk", "WeCom", "WeChat", "QQ Official"] {
        assert!(
            channels.iter().any(|(handle, enabled)| handle.name() == name && *enabled),
            "{name} should be reported as runtime eligible"
        );
    }

    let ineligible = ChannelsConfig {
        wecom: Some(WeComConfig { bot_id: "bot-id".to_string(), secret: " ".to_string() }),
        wechat: Some(WeChatConfig {
            bot_token: "bot-token".to_string(),
            base_url: "http://untrusted.example.com".to_string(),
        }),
        qq: Some(QQConfig {
            app_id: "app-id".to_string(),
            app_secret: "app-secret".to_string(),
            allowed_users: Vec::new(),
            receive_mode: QQReceiveMode::Webhook,
        }),
        ..Default::default()
    };
    let channels = ineligible.channels_except_webhook();
    for name in ["WeCom", "WeChat", "QQ Official"] {
        assert!(
            channels.iter().any(|(handle, enabled)| handle.name() == name && !*enabled),
            "{name} should remain listed but not runtime eligible"
        );
    }
}
