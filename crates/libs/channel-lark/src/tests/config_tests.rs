//! Lark/飞书配置转换测试。
//!
//! 本模块覆盖配置 serde、TOML 往返、默认值和平台构造函数，确保历史
//! 配置字段与当前区域选择逻辑保持兼容。

use super::*;
use config_channel_services::{FeishuConfig, LarkConfig, LarkReceiveMode};

#[test]
fn lark_config_serde() {
    // JSON 序列化是配置文件和外部接口的公共边界，需锁定字段往返行为。
    let lc = LarkConfig {
        app_id: "cli_app123".into(),
        app_secret: "secret456".into(),
        encrypt_key: None,
        verification_token: Some("vtoken789".into()),
        allowed_users: vec!["ou_user1".into(), "ou_user2".into()],
        mention_only: false,
        group_reply: None,
        use_feishu: false,
        receive_mode: LarkReceiveMode::default(),
        port: None,
        draft_update_interval_ms: 3_000,
        max_draft_edits: 20,
    };
    let json = serde_json::to_string(&lc).unwrap();
    let parsed: LarkConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.app_id, "cli_app123");
    assert_eq!(parsed.app_secret, "secret456");
    assert_eq!(parsed.verification_token.as_deref(), Some("vtoken789"));
    assert_eq!(parsed.allowed_users.len(), 2);
}

#[test]
fn lark_config_toml_roundtrip() {
    let lc = LarkConfig {
        app_id: "app".into(),
        app_secret: "secret".into(),
        encrypt_key: None,
        verification_token: Some("tok".into()),
        allowed_users: vec!["*".into()],
        mention_only: false,
        group_reply: None,
        use_feishu: false,
        receive_mode: LarkReceiveMode::Webhook,
        port: Some(9898),
        draft_update_interval_ms: 3_000,
        max_draft_edits: 20,
    };
    let toml_str = toml::to_string(&lc).unwrap();
    let parsed: LarkConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.app_id, "app");
    assert_eq!(parsed.verification_token.as_deref(), Some("tok"));
    assert_eq!(parsed.allowed_users, vec!["*"]);
}

#[test]
fn lark_config_defaults_optional_fields() {
    // 只提供必填凭据时，应由 schema 填充安全的默认接收模式和空白 allowlist。
    let json = r#"{"app_id":"a","app_secret":"s"}"#;
    let parsed: LarkConfig = serde_json::from_str(json).unwrap();

    assert!(parsed.verification_token.is_none());
    assert!(parsed.allowed_users.is_empty());
    assert!(!parsed.mention_only);
    assert_eq!(parsed.receive_mode, LarkReceiveMode::Websocket);
    assert!(parsed.port.is_none());
}

#[test]
fn lark_from_config_preserves_mode_and_region() {
    let cfg = LarkConfig {
        app_id: "cli_app123".into(),
        app_secret: "secret456".into(),
        encrypt_key: None,
        verification_token: Some("vtoken789".into()),
        allowed_users: vec!["*".into()],
        mention_only: false,
        group_reply: None,
        use_feishu: false,
        receive_mode: LarkReceiveMode::Webhook,
        port: Some(9898),
        draft_update_interval_ms: 3_000,
        max_draft_edits: 20,
    };

    let ch = LarkChannel::from_config(&cfg);

    assert_eq!(ch.api_base(), LARK_BASE_URL);
    assert_eq!(ch.ws_base(), LARK_WS_BASE_URL);
    assert_eq!(ch.receive_mode, LarkReceiveMode::Webhook);
    assert_eq!(ch.port, Some(9898));
}

#[test]
fn lark_from_lark_config_ignores_legacy_feishu_flag() {
    // 新路径显式区分 Lark/Feishu config，旧的 use_feishu 标志不能偷偷改变平台。
    let cfg = LarkConfig {
        app_id: "cli_app123".into(),
        app_secret: "secret456".into(),
        encrypt_key: None,
        verification_token: Some("vtoken789".into()),
        allowed_users: vec!["*".into()],
        mention_only: false,
        group_reply: None,
        use_feishu: true,
        receive_mode: LarkReceiveMode::Webhook,
        port: Some(9898),
        draft_update_interval_ms: 3_000,
        max_draft_edits: 20,
    };

    let ch = LarkChannel::from_lark_config(&cfg);

    assert_eq!(ch.api_base(), LARK_BASE_URL);
    assert_eq!(ch.ws_base(), LARK_WS_BASE_URL);
    assert_eq!(ch.name(), "lark");
}

#[test]
fn lark_from_feishu_config_sets_feishu_platform() {
    // 飞书配置必须切到飞书域名，避免把国内租户请求发到 Lark 国际域名。
    let cfg = FeishuConfig {
        app_id: "cli_feishu_app123".into(),
        app_secret: "secret456".into(),
        encrypt_key: None,
        verification_token: Some("vtoken789".into()),
        allowed_users: vec!["*".into()],
        group_reply: None,
        receive_mode: LarkReceiveMode::Webhook,
        port: Some(9898),
        draft_update_interval_ms: 3_000,
        max_draft_edits: 20,
    };

    let ch = LarkChannel::from_feishu_config(&cfg);

    assert_eq!(ch.api_base(), FEISHU_BASE_URL);
    assert_eq!(ch.ws_base(), FEISHU_WS_BASE_URL);
    assert_eq!(ch.name(), "feishu");
}
