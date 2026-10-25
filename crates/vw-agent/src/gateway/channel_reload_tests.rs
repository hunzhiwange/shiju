//! 验证重载时连接凭据、签名密钥和禁用状态一致更新。
use super::*;

#[test]
fn channel_reload_replaces_webhook_auth_and_disables_removed_channels() {
    let mut config = ChannelsConfig::default();
    config.webhook = Some(
        serde_json::from_value(serde_json::json!({"port":8787,"secret":"first-secret"})).unwrap(),
    );
    config.whatsapp = Some(serde_json::from_value(serde_json::json!({
        "access_token":"access", "phone_number_id":"phone", "verify_token":"first-token", "app_secret":"first-app-secret"
    })).unwrap());
    let first = build_gateway_channels(&config);
    assert_eq!(
        first.webhook_secret_hash.as_deref(),
        Some(hash_webhook_secret("first-secret").as_str())
    );
    assert_eq!(first.whatsapp.unwrap().verify_token(), "first-token");
    config.webhook.as_mut().unwrap().secret = Some("second-secret".into());
    config.whatsapp.as_mut().unwrap().verify_token = Some("second-token".into());
    let second = build_gateway_channels(&config);
    assert_eq!(
        second.webhook_secret_hash.as_deref(),
        Some(hash_webhook_secret("second-secret").as_str())
    );
    assert_eq!(second.whatsapp.unwrap().verify_token(), "second-token");
    let disabled = build_gateway_channels(&ChannelsConfig::default());
    assert!(disabled.webhook_secret_hash.is_none());
    assert!(disabled.whatsapp.is_none());
    assert!(disabled.qq.is_none());
    assert!(!disabled.qq_webhook_enabled);
}
