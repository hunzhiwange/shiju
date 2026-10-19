use super::*;

#[test]
fn wechat_config_requires_token_and_trusted_base_url() {
    let valid =
        WeChatConfig { bot_token: "token".into(), base_url: DEFAULT_WECHAT_BASE_URL.into() };
    assert!(valid.has_required_credentials());
    assert!(!WeChatConfig { bot_token: " ".into(), ..valid.clone() }.has_required_credentials());
    assert!(
        !WeChatConfig { bot_token: "token".into(), base_url: "https://attacker.example".into() }
            .has_required_credentials()
    );
}

#[test]
fn wechat_config_defaults_to_official_ilink_base_url() {
    let config: WeChatConfig = serde_json::from_str(r#"{"bot_token":"token"}"#).unwrap();
    assert_eq!(config.base_url, DEFAULT_WECHAT_BASE_URL);
}
