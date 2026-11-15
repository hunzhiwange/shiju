//! Lark/飞书回执反应测试。
//!
//! 本模块验证消息 reaction URL、语言环境识别和随机回执表情池选择，
//! 确保不同平台与用户语言下的轻量确认反馈一致。

use super::*;

#[test]
fn lark_reaction_url_matches_region() {
    let ch_lark = make_channel();
    assert_eq!(
        ch_lark.message_reaction_url("om_test_message_id"),
        "https://open.larksuite.com/open-apis/im/v1/messages/om_test_message_id/reactions"
    );

    let feishu_cfg = config_channel_services::FeishuConfig {
        app_id: "cli_app123".into(),
        app_secret: "secret456".into(),
        encrypt_key: None,
        verification_token: Some("vtoken789".into()),
        allowed_users: vec!["*".into()],
        group_reply: None,
        receive_mode: config_channel_services::LarkReceiveMode::Webhook,
        port: Some(9898),
        draft_update_interval_ms: 3_000,
        max_draft_edits: 20,
    };
    // Lark 与飞书共享大部分协议，但 OpenAPI 域名不同；这里锁定区域映射。
    let ch_feishu = LarkChannel::from_feishu_config(&feishu_cfg);
    assert_eq!(
        ch_feishu.message_reaction_url("om_test_message_id"),
        "https://open.feishu.cn/open-apis/im/v1/messages/om_test_message_id/reactions"
    );
}

#[test]
fn lark_reaction_locale_explicit_language_tags() {
    assert_eq!(map_locale_tag("zh-CN"), Some(LarkAckLocale::ZhCn));
    assert_eq!(map_locale_tag("zh_TW"), Some(LarkAckLocale::ZhTw));
    assert_eq!(map_locale_tag("zh-Hant"), Some(LarkAckLocale::ZhTw));
    assert_eq!(map_locale_tag("en-US"), Some(LarkAckLocale::En));
    assert_eq!(map_locale_tag("ja-JP"), Some(LarkAckLocale::Ja));
    assert_eq!(map_locale_tag("fr-FR"), None);
}

#[test]
fn lark_reaction_locale_prefers_explicit_payload_locale() {
    // 明确 locale 来自平台 payload，优先级高于文本脚本推断。
    let payload = serde_json::json!({
        "sender": {
            "locale": "ja-JP"
        },
        "message": {
            "content": "{\"text\":\"hello\"}"
        }
    });
    assert_eq!(detect_lark_ack_locale(Some(&payload), "你好，世界"), LarkAckLocale::Ja);
}

#[test]
fn lark_reaction_locale_unsupported_payload_falls_back_to_text_script() {
    // 平台 locale 不支持时退回到文本字符集推断，避免直接落到英文。
    let payload = serde_json::json!({
        "sender": {
            "locale": "fr-FR"
        },
        "message": {
            "content": "{\"text\":\"頑張れ\"}"
        }
    });
    assert_eq!(detect_lark_ack_locale(Some(&payload), "頑張ってください"), LarkAckLocale::Ja);
}

#[test]
fn lark_reaction_locale_detects_simplified_and_traditional_text() {
    assert_eq!(detect_lark_ack_locale(None, "继续奋斗，今天很强"), LarkAckLocale::ZhCn);
    assert_eq!(detect_lark_ack_locale(None, "繼續奮鬥，今天很強"), LarkAckLocale::ZhTw);
}

#[test]
fn lark_reaction_locale_defaults_to_english_for_unsupported_text() {
    assert_eq!(detect_lark_ack_locale(None, "Bonjour tout le monde"), LarkAckLocale::En);
}

#[test]
fn random_lark_ack_reaction_respects_detected_locale_pool() {
    // 随机性只允许发生在对应语言池内部，保证用户看到本地化的回执。
    let payload = serde_json::json!({
        "sender": {
            "locale": "zh-CN"
        }
    });
    let selected = random_lark_ack_reaction(Some(&payload), "hello");
    assert!(LARK_ACK_REACTIONS_ZH_CN.contains(&selected));

    let payload = serde_json::json!({
        "sender": {
            "locale": "zh-TW"
        }
    });
    let selected = random_lark_ack_reaction(Some(&payload), "hello");
    assert!(LARK_ACK_REACTIONS_ZH_TW.contains(&selected));

    let payload = serde_json::json!({
        "sender": {
            "locale": "en-US"
        }
    });
    let selected = random_lark_ack_reaction(Some(&payload), "hello");
    assert!(LARK_ACK_REACTIONS_EN.contains(&selected));

    let payload = serde_json::json!({
        "sender": {
            "locale": "ja-JP"
        }
    });
    let selected = random_lark_ack_reaction(Some(&payload), "hello");
    assert!(LARK_ACK_REACTIONS_JA.contains(&selected));
}
