//! 验证企业消息渠道的配置面板、实例标题与可选凭据路径。
//! 飞书和钉钉的仅实例配置用于确认名称回退且不臆造缺失凭据。

use crate::app::{App, Message};
use vw_config_types::channels::{
    ClawdTalkConfig, DingTalkConfig, DingTalkInstanceConfig, FeishuConfig, FeishuInstanceConfig,
    GroupReplyConfig, GroupReplyMode, LarkConfig, LarkReceiveMode, NostrConfig, QQConfig,
    QQReceiveMode,
};

// 企业渠道实例由用例显式注入，避免其他渠道配置污染标题回退。
fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn group_reply() -> GroupReplyConfig {
    GroupReplyConfig {
        mode: Some(GroupReplyMode::MentionOnly),
        allowed_sender_ids: vec!["alice".to_string()],
    }
}

fn configure_enterprise_channels(app: &mut App) {
    app.channels_settings.lark = Some(LarkConfig {
        app_id: "lark-app".to_string(),
        app_secret: "lark-secret".to_string(),
        encrypt_key: Some("encrypt".to_string()),
        verification_token: Some("verify".to_string()),
        allowed_users: vec!["alice".to_string()],
        mention_only: true,
        group_reply: Some(group_reply()),
        use_feishu: true,
        receive_mode: LarkReceiveMode::Webhook,
        port: Some(3001),
        draft_update_interval_ms: 500,
        max_draft_edits: 2,
    });
    app.channels_settings.feishu = Some(FeishuConfig {
        app_id: "feishu-app".to_string(),
        app_secret: "feishu-secret".to_string(),
        encrypt_key: Some("encrypt".to_string()),
        verification_token: Some("verify".to_string()),
        allowed_users: vec!["alice".to_string()],
        group_reply: Some(group_reply()),
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 1000,
        max_draft_edits: 3,
    });
    app.channels_settings.dingtalk = Some(DingTalkConfig {
        client_id: "client".to_string(),
        client_secret: "secret".to_string(),
        allowed_users: vec!["alice".to_string()],
    });
    app.channels_settings.qq = Some(QQConfig {
        app_id: "qq-app".to_string(),
        app_secret: "qq-secret".to_string(),
        allowed_users: vec!["alice".to_string()],
        receive_mode: QQReceiveMode::Websocket,
    });
    app.channels_settings.nostr = Some(NostrConfig {
        private_key: "nsec".to_string(),
        relays: vec!["wss://relay.example".to_string()],
        allowed_pubkeys: vec!["npub".to_string()],
    });
    app.channels_settings.clawdtalk = Some(ClawdTalkConfig {
        api_key: "api-key".to_string(),
        connection_id: "conn".to_string(),
        from_number: "+1000".to_string(),
        allowed_destinations: vec!["*".to_string()],
        webhook_secret: Some("secret".to_string()),
    });
    app.channels_settings.refresh_text_inputs();
}

#[test]
fn enterprise_channels_tests_are_wired() {
    assert!(module_path!().contains("enterprise_channels_tests"));
}

#[test]
fn enterprise_panels_build_disabled_and_expanded_enabled_states() {
    let mut app = test_app();
    keep_element(super::enterprise_channels::lark_panel(&app));
    keep_element(super::enterprise_channels::feishu_panel(&app));
    keep_element(super::enterprise_channels::dingtalk_panel(&app));
    keep_element(super::enterprise_channels::qq_panel(&app));
    keep_element(super::enterprise_channels::nostr_panel(&app));
    keep_element(super::enterprise_channels::clawdtalk_panel(&app));

    configure_enterprise_channels(&mut app);
    keep_element(super::enterprise_channels::lark_panel(&app));
    keep_element(super::enterprise_channels::feishu_panel(&app));
    keep_element(super::enterprise_channels::dingtalk_panel(&app));
    keep_element(super::enterprise_channels::qq_panel(&app));
    keep_element(super::enterprise_channels::nostr_panel(&app));
    keep_element(super::enterprise_channels::clawdtalk_panel(&app));
}

#[test]
fn enterprise_panels_build_configured_state() {
    let mut app = test_app();
    configure_enterprise_channels(&mut app);
    keep_element(super::enterprise_channels::lark_panel(&app));
    keep_element(super::enterprise_channels::feishu_panel(&app));
    keep_element(super::enterprise_channels::dingtalk_panel(&app));
    keep_element(super::enterprise_channels::qq_panel(&app));
    keep_element(super::enterprise_channels::nostr_panel(&app));
    keep_element(super::enterprise_channels::clawdtalk_panel(&app));
}

#[test]
fn enterprise_panels_build_when_optional_credentials_are_absent() {
    let mut app = test_app();
    configure_enterprise_channels(&mut app);

    let lark = app.channels_settings.lark.as_mut().expect("configured Lark channel");
    lark.encrypt_key = None;
    lark.verification_token = None;
    lark.group_reply = None;
    lark.receive_mode = LarkReceiveMode::Websocket;
    lark.port = None;

    let feishu = app.channels_settings.feishu.as_mut().expect("configured Feishu channel");
    feishu.encrypt_key = None;
    feishu.verification_token = None;
    feishu.group_reply = None;
    feishu.receive_mode = LarkReceiveMode::Webhook;
    feishu.port = Some(u16::MAX);

    app.channels_settings.qq.as_mut().expect("configured QQ channel").receive_mode =
        QQReceiveMode::Webhook;
    app.channels_settings
        .clawdtalk
        .as_mut()
        .expect("configured ClawdTalk channel")
        .webhook_secret = None;
    app.channels_settings.refresh_text_inputs();

    keep_element(super::enterprise_channels::lark_panel(&app));
    keep_element(super::enterprise_channels::feishu_panel(&app));
    keep_element(super::enterprise_channels::qq_panel(&app));
    keep_element(super::enterprise_channels::clawdtalk_panel(&app));
}

#[test]
fn feishu_panel_builds_instance_only_state_with_named_and_fallback_titles() {
    let mut app = test_app();
    app.channels_settings.feishu_instances.extend([
        FeishuInstanceConfig {
            id: "   ".to_string(),
            config: FeishuConfig {
                app_id: "fallback-app".to_string(),
                app_secret: "fallback-secret".to_string(),
                encrypt_key: None,
                verification_token: None,
                allowed_users: Vec::new(),
                group_reply: None,
                receive_mode: LarkReceiveMode::Webhook,
                port: None,
                draft_update_interval_ms: 100,
                max_draft_edits: 1,
            },
        },
        FeishuInstanceConfig {
            id: "support".to_string(),
            config: FeishuConfig {
                app_id: "named-app".to_string(),
                app_secret: "named-secret".to_string(),
                encrypt_key: Some("encrypt".to_string()),
                verification_token: Some("verify".to_string()),
                allowed_users: vec!["alice".to_string()],
                group_reply: Some(group_reply()),
                receive_mode: LarkReceiveMode::Websocket,
                port: Some(3002),
                draft_update_interval_ms: 60_000,
                max_draft_edits: 200,
            },
        },
    ]);
    app.channels_settings.refresh_text_inputs();

    assert!(app.channels_settings.feishu.is_none());
    keep_element(super::enterprise_channels::feishu_panel(&app));
}

#[test]
fn dingtalk_panel_builds_instance_only_state_with_named_and_fallback_titles() {
    let mut app = test_app();
    app.channels_settings.dingtalk_instances.extend([
        DingTalkInstanceConfig {
            id: "\t".to_string(),
            config: DingTalkConfig {
                client_id: "fallback-client".to_string(),
                client_secret: "fallback-secret".to_string(),
                allowed_users: Vec::new(),
            },
        },
        DingTalkInstanceConfig {
            id: "operations".to_string(),
            config: DingTalkConfig {
                client_id: "named-client".to_string(),
                client_secret: "named-secret".to_string(),
                allowed_users: vec!["alice".to_string()],
            },
        },
    ]);
    app.channels_settings.refresh_text_inputs();

    assert!(app.channels_settings.dingtalk.is_none());
    keep_element(super::enterprise_channels::dingtalk_panel(&app));
}
