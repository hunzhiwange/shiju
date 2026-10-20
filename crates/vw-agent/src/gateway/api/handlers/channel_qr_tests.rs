use axum::extract::{Extension, Path, Query};
use serde_json::{Map, Value, json};
use vw_api_types::id::TenantScope;
use vw_config_types::channels::{
    DingTalkConfig, FeishuConfig, QQConfig, QQReceiveMode, WeChatConfig, WeComConfig,
};

use crate::app::agent::channels::manager::collect_configured_channels;
use crate::app::agent::config::Config;

use super::{
    ChannelQrStatusQuery, channel_qr_fetch, channel_qr_status, seed_pending_handle_for_test,
};

fn qr_credential(
    credentials: &Map<String, Value>,
    channel: &str,
    field: &str,
) -> Result<String, String> {
    credentials
        .get(channel)
        .and_then(Value::as_object)
        .and_then(|fields| fields.get(field))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{channel} QR credentials are missing {field}"))
}

fn runtime_config_from_qr_credentials(credentials: &Map<String, Value>) -> Result<Config, String> {
    let mut config = Config::default();
    config.channels_config.dingtalk = Some(DingTalkConfig {
        client_id: qr_credential(credentials, "dingtalk", "client_id")?,
        client_secret: qr_credential(credentials, "dingtalk", "client_secret")?,
        allowed_users: Vec::new(),
    });
    config.channels_config.feishu = Some(FeishuConfig {
        app_id: qr_credential(credentials, "feishu", "app_id")?,
        app_secret: qr_credential(credentials, "feishu", "app_secret")?,
        encrypt_key: None,
        verification_token: None,
        allowed_users: Vec::new(),
        group_reply: None,
        receive_mode: crate::app::agent::config::schema::LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 3_000,
        max_draft_edits: 20,
    });
    config.channels_config.qq = Some(QQConfig {
        app_id: qr_credential(credentials, "qq", "app_id")?,
        app_secret: qr_credential(credentials, "qq", "app_secret")?,
        allowed_users: Vec::new(),
        receive_mode: QQReceiveMode::Websocket,
    });
    config.channels_config.wecom = Some(WeComConfig {
        bot_id: qr_credential(credentials, "wecom", "bot_id")?,
        secret: qr_credential(credentials, "wecom", "secret")?,
    });
    config.channels_config.wechat = Some(WeChatConfig {
        bot_token: qr_credential(credentials, "wechat", "bot_token")?,
        base_url: qr_credential(credentials, "wechat", "base_url")?,
    });
    Ok(config)
}

#[tokio::test]
async fn channel_qr_rejects_unknown_and_cross_tenant_poll() {
    let tenant_a = TenantScope::new("platform-a", "company-a").expect("tenant A");
    let tenant_b = TenantScope::new("platform-b", "company-b").expect("tenant B");

    let unknown =
        channel_qr_fetch(Extension(tenant_a.clone()), Path("unknown-channel".to_string()))
            .await
            .expect_err("unknown channel must be rejected");
    assert_eq!(unknown.status.as_u16(), 400);

    let handle = seed_pending_handle_for_test(&tenant_a, "dingtalk");
    let cross_tenant = channel_qr_status(
        Extension(tenant_b),
        Path("dingtalk".to_string()),
        Query(ChannelQrStatusQuery { handle }),
    )
    .await
    .expect_err("a tenant must not poll another tenant's handle");
    assert_eq!(cross_tenant.status.as_u16(), 404);
}

#[test]
fn five_channel_qr_credentials_match_runtime_config() {
    let credentials = Map::from_iter([
        (
            "dingtalk".to_string(),
            json!({"client_id": "ding-client", "client_secret": "ding-secret"}),
        ),
        ("feishu".to_string(), json!({"app_id": "feishu-app", "app_secret": "feishu-secret"})),
        ("qq".to_string(), json!({"app_id": "qq-app", "app_secret": "qq-secret"})),
        ("wecom".to_string(), json!({"bot_id": "wecom-bot", "secret": "wecom-secret"})),
        (
            "wechat".to_string(),
            json!({
                "bot_token": "wechat-token",
                "base_url": vw_config_types::channels::DEFAULT_WECHAT_BASE_URL,
            }),
        ),
    ]);

    let config = runtime_config_from_qr_credentials(&credentials).expect("valid QR credentials");
    let names = collect_configured_channels(&config, "QR regression")
        .into_iter()
        .map(|channel| channel.display_name)
        .collect::<Vec<_>>();

    assert_eq!(names, ["Feishu", "DingTalk", "QQ", "WeCom", "WeChat"]);
}
