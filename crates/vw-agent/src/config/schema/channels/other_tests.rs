// 覆盖其它渠道配置的展示元数据及 WhatsApp 后端判定规则。

use super::other::{
    DingTalkConfig, IrcConfig, LinqConfig, MatrixConfig, NextcloudTalkConfig, NostrConfig,
    QQConfig, SignalConfig, WatiConfig, WhatsAppConfig,
};
use crate::app::agent::config::traits::ChannelConfig;

#[test]
fn other_channel_metadata_is_user_facing() {
    assert_eq!(MatrixConfig::name(), "Matrix");
    assert_eq!(MatrixConfig::desc(), "self-hosted chat");
    assert_eq!(SignalConfig::name(), "Signal");
    assert_eq!(SignalConfig::desc(), "An open-source, encrypted messaging service");
    assert_eq!(WhatsAppConfig::name(), "WhatsApp");
    assert_eq!(WhatsAppConfig::desc(), "Business Cloud API");
    assert_eq!(LinqConfig::name(), "Linq");
    assert_eq!(LinqConfig::desc(), "iMessage/RCS/SMS via Linq API");
    assert_eq!(WatiConfig::name(), "WATI");
    assert_eq!(WatiConfig::desc(), "WhatsApp via WATI Business API");
    assert_eq!(NextcloudTalkConfig::name(), "NextCloud Talk");
    assert_eq!(NextcloudTalkConfig::desc(), "NextCloud Talk platform");
    assert_eq!(IrcConfig::name(), "IRC");
    assert_eq!(IrcConfig::desc(), "IRC over TLS");
    assert_eq!(DingTalkConfig::name(), "DingTalk");
    assert_eq!(DingTalkConfig::desc(), "DingTalk Stream Mode");
    assert_eq!(QQConfig::name(), "QQ Official");
    assert_eq!(QQConfig::desc(), "Tencent QQ Bot");
    assert_eq!(NostrConfig::name(), "Nostr");
    assert_eq!(NostrConfig::desc(), "Nostr DMs");
}

#[test]
fn whatsapp_backend_helpers_distinguish_cloud_web_and_ambiguous_configs() {
    let cloud = WhatsAppConfig {
        access_token: Some("token".to_string()),
        phone_number_id: Some("phone".to_string()),
        verify_token: Some("verify".to_string()),
        app_secret: None,
        session_path: None,
        pair_phone: None,
        pair_code: None,
        allowed_numbers: Vec::new(),
    };
    let web = WhatsAppConfig { session_path: Some("/tmp/session".to_string()), ..cloud.clone() };
    let default_cloud = WhatsAppConfig {
        access_token: None,
        phone_number_id: None,
        verify_token: None,
        app_secret: None,
        session_path: None,
        pair_phone: None,
        pair_code: None,
        allowed_numbers: Vec::new(),
    };
    let incomplete_cloud =
        WhatsAppConfig { phone_number_id: Some("phone".to_string()), ..default_cloud.clone() };

    assert_eq!(cloud.backend_type(), "cloud");
    assert!(cloud.is_cloud_config());
    assert!(!cloud.is_web_config());
    assert_eq!(web.backend_type(), "cloud");
    assert!(web.is_web_config());
    assert!(web.is_ambiguous_config());
    assert_eq!(default_cloud.backend_type(), "cloud");
    assert!(!default_cloud.is_cloud_config());
    assert!(!default_cloud.is_web_config());
    assert!(!default_cloud.is_ambiguous_config());
    assert_eq!(
        WhatsAppConfig { session_path: Some("/tmp/session".to_string()), ..default_cloud.clone() }
            .backend_type(),
        "web"
    );
    assert!(!incomplete_cloud.is_cloud_config());
    assert!(!incomplete_cloud.is_ambiguous_config());
}
