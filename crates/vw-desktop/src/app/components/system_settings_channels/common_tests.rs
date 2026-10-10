#![allow(unused_imports, dead_code)]

//! 验证渠道设置共享字段、开关、实例列表与样式组件。
//! fixture 以空值和多实例组合检查共享控件的布局及消息归属。

use super::common::{
    GROUP_REPLY_MODE_OPTIONS, LARK_RECEIVE_MODE_OPTIONS, QQ_RECEIVE_MODE_OPTIONS, add_instance_row,
    bool_row, bool_row_with_key, enabled_channels, group_reply_mode_value, hint_row,
    instance_title_row, is_multiline_list_key, lark_receive_mode_value, localized_label,
    multiline_placeholder, number_row, number_row_with_hint, number_row_with_key, panel, pick_row,
    pick_row_with_key, qq_receive_mode_value, text_row, text_row_with_key,
};
use crate::app::{App, Message};
use iced::widget::text;
use vw_config_types::channels::{
    ClawdTalkConfig, DingTalkConfig, DingTalkInstanceConfig, DiscordConfig, FeishuConfig,
    FeishuInstanceConfig, GroupReplyConfig, GroupReplyMode, IMessageConfig, IrcConfig, LarkConfig,
    LarkReceiveMode, LinqConfig, MattermostConfig, NextcloudTalkConfig, NostrConfig, QQConfig,
    QQReceiveMode, SignalConfig, SlackConfig, TelegramConfig, WatiConfig, WebhookConfig,
    WhatsAppConfig,
};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn feishu_config() -> FeishuConfig {
    FeishuConfig {
        app_id: "test-app".to_string(),
        app_secret: "test-secret".to_string(),
        encrypt_key: None,
        verification_token: None,
        allowed_users: vec!["alice".to_string()],
        group_reply: None,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 1_000,
        max_draft_edits: 3,
    }
}

fn dingtalk_config() -> DingTalkConfig {
    DingTalkConfig {
        client_id: "test-client".to_string(),
        client_secret: "test-secret".to_string(),
        allowed_users: vec!["alice".to_string()],
    }
}

#[test]
fn localized_label_maps_every_known_key_and_falls_back_to_original_key() {
    for (key, expected) in [
        ("bot_token", "机器人令牌"),
        ("instance_id", "实例 ID"),
        ("allowed_users", "允许用户"),
        ("group_reply.mode", "群聊回复模式"),
        ("group_reply.allowed_sender_ids", "允许发送者 ID"),
        ("mention_only", "仅提及时回复"),
        ("homeserver", "Homeserver 地址"),
        ("access_token", "访问令牌"),
        ("user_id", "用户 ID"),
        ("device_id", "设备 ID"),
        ("room_id", "房间 ID"),
        ("http_url", "HTTP 地址"),
        ("account", "账号"),
        ("group_id", "群组 ID"),
        ("ignore_attachments", "忽略附件"),
        ("ignore_stories", "忽略动态"),
        ("phone_number_id", "电话号码 ID"),
        ("verify_token", "验证令牌"),
        ("app_secret", "应用密钥"),
        ("session_path", "会话路径"),
        ("pair_phone", "配对手机号"),
        ("pair_code", "配对码"),
        ("allowed_numbers", "允许号码"),
        ("api_token", "API 令牌"),
        ("from_phone", "发送号码"),
        ("signing_secret", "签名密钥"),
        ("api_url", "API 地址"),
        ("tenant_id", "租户 ID"),
        ("base_url", "基础地址"),
        ("app_token", "应用令牌"),
        ("webhook_secret", "Webhook 密钥"),
        ("imap_host", "IMAP 主机"),
        ("imap_port", "IMAP 端口"),
        ("imap_folder", "IMAP 文件夹"),
        ("smtp_host", "SMTP 主机"),
        ("smtp_port", "SMTP 端口"),
        ("smtp_tls", "SMTP 使用 TLS"),
        ("username", "用户名"),
        ("password", "密码"),
        ("from_address", "发件地址"),
        ("idle_timeout_secs", "空闲超时"),
        ("allowed_senders", "允许发送者"),
        ("server", "服务器"),
        ("port", "端口"),
        ("nickname", "昵称"),
        ("channels", "频道列表"),
        ("server_password", "服务器密码"),
        ("nickserv_password", "NickServ 密码"),
        ("sasl_password", "SASL 密码"),
        ("verify_tls", "验证 TLS"),
        ("app_id", "应用 ID"),
        ("encrypt_key", "加密密钥"),
        ("verification_token", "验证令牌"),
        ("use_feishu", "使用飞书端点"),
        ("receive_mode", "接收模式"),
        ("draft_update_interval_ms", "草稿刷新间隔"),
        ("max_draft_edits", "最大草稿编辑次数"),
        ("client_id", "应用 Key"),
        ("client_secret", "应用密钥"),
        ("private_key", "私钥"),
        ("relays", "中继列表"),
        ("allowed_pubkeys", "允许公钥"),
        ("api_key", "API 密钥"),
        ("connection_id", "连接 ID"),
        ("from_number", "发送号码"),
        ("allowed_destinations", "允许目的地"),
    ] {
        assert_eq!(localized_label(key), expected);
    }
    assert_eq!(localized_label("unknown.key"), "unknown.key");
}

#[test]
fn multiline_placeholder_falls_back_to_original_text() {
    assert_eq!(multiline_placeholder("逗号或换行分隔"), "每行一个，亦支持逗号分隔");
    assert_eq!(
        multiline_placeholder("电话号码或邮箱，逗号或换行分隔"),
        "每行一个电话号码或邮箱，亦支持逗号分隔"
    );
    assert_eq!(
        multiline_placeholder("逗号或换行分隔，支持 *"),
        "每行一个，亦支持逗号分隔，可使用 *"
    );
    assert_eq!(multiline_placeholder("unknown.key"), "unknown.key");
}

#[test]
fn mode_value_helpers_cover_defaults_and_variants() {
    assert_eq!(group_reply_mode_value(&None), "all_messages");
    assert_eq!(
        group_reply_mode_value(&Some(GroupReplyConfig { mode: None, allowed_sender_ids: vec![] })),
        "all_messages"
    );
    assert_eq!(
        group_reply_mode_value(&Some(GroupReplyConfig {
            mode: Some(GroupReplyMode::AllMessages),
            allowed_sender_ids: vec![],
        })),
        "all_messages"
    );
    assert_eq!(
        group_reply_mode_value(&Some(GroupReplyConfig {
            mode: Some(GroupReplyMode::MentionOnly),
            allowed_sender_ids: vec![],
        })),
        "mention_only"
    );
    assert_eq!(lark_receive_mode_value(LarkReceiveMode::Webhook), "webhook");
    assert_eq!(lark_receive_mode_value(LarkReceiveMode::Websocket), "websocket");
    assert_eq!(qq_receive_mode_value(QQReceiveMode::Webhook), "webhook");
    assert_eq!(qq_receive_mode_value(QQReceiveMode::Websocket), "websocket");
}

#[test]
fn labeled_options_display_human_labels() {
    assert_eq!(
        GROUP_REPLY_MODE_OPTIONS.map(|option| (option.value, option.to_string())),
        [("all_messages", "全部消息".to_string()), ("mention_only", "仅被提及".to_string())]
    );
    assert_eq!(
        LARK_RECEIVE_MODE_OPTIONS.map(|option| (option.value, option.to_string())),
        [("websocket", "WebSocket".to_string()), ("webhook", "Webhook".to_string())]
    );
    assert_eq!(
        QQ_RECEIVE_MODE_OPTIONS.map(|option| (option.value, option.to_string())),
        [("webhook", "Webhook".to_string()), ("websocket", "WebSocket".to_string())]
    );
}

#[test]
#[should_panic(expected = "multiline channel editor should be initialized")]
fn multiline_text_row_rejects_missing_editor_state() {
    let app = test_app();

    keep_element(text_row(
        &app,
        "allowed_users",
        "逗号或换行分隔",
        "",
        "telegram.allowed_users",
        false,
    ));
}

#[test]
fn panel_builds_collapsed_disabled_and_expanded_enabled_content() {
    keep_element(panel("x", "标题", "说明", false, text("body").into()));
    keep_element(panel("x", "标题", "说明", true, text("body").into()));
}

#[test]
fn enabled_channels_returns_empty_when_every_channel_is_disabled() {
    let mut app = test_app();
    app.channels_settings.cli = false;

    assert!(enabled_channels(&app).is_empty());
}
