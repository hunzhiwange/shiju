use super::common::{
    GROUP_REPLY_MODE_OPTIONS, LARK_RECEIVE_MODE_OPTIONS, QQ_RECEIVE_MODE_OPTIONS, add_instance_row,
    bool_row, bool_row_with_key, enabled_channels, group_reply_mode_value, hint_row,
    instance_title_row, is_multiline_list_key, lark_receive_mode_value, localized_label,
    multiline_placeholder, number_row, number_row_with_key, panel, pick_row, pick_row_with_key,
    qq_receive_mode_value, text_row, text_row_with_key,
};
use crate::app::{App, Message};
use iced::widget::text;
use vw_config_types::channels::{
    ClawdTalkConfig, DingTalkConfig, DingTalkInstanceConfig, DiscordConfig, FeishuConfig,
    FeishuInstanceConfig, GroupReplyConfig, GroupReplyMode, IMessageConfig, IrcConfig, LarkConfig,
    LarkReceiveMode, LinqConfig, MatrixConfig, MatrixInstanceConfig, MattermostConfig,
    NextcloudTalkConfig, NostrConfig, QQConfig, QQReceiveMode, SignalConfig, SlackConfig,
    TelegramConfig, WatiConfig, WebhookConfig, WhatsAppConfig,
};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn matrix_config() -> MatrixConfig {
    MatrixConfig {
        homeserver: "https://matrix.example".to_string(),
        access_token: "test-access-token".to_string(),
        user_id: Some("@bot:matrix.example".to_string()),
        device_id: Some("TEST-DEVICE".to_string()),
        room_id: "!room:matrix.example".to_string(),
        allowed_users: vec!["@alice:matrix.example".to_string()],
        mention_only: true,
    }
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

fn configure_all_direct_channels(app: &mut App) {
    app.channels_settings.cli = true;
    app.channels_settings.telegram = Some(TelegramConfig {
        bot_token: "test-token".to_string(),
        allowed_users: vec![],
        stream_mode: Default::default(),
        draft_update_interval_ms: 500,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    });
    app.channels_settings.discord = Some(DiscordConfig {
        bot_token: "test-token".to_string(),
        guild_id: None,
        allowed_users: vec![],
        listen_to_bots: false,
        mention_only: false,
        group_reply: None,
    });
    app.channels_settings.slack = Some(SlackConfig {
        bot_token: "test-token".to_string(),
        app_token: None,
        channel_id: None,
        allowed_users: vec![],
        group_reply: None,
    });
    app.channels_settings.mattermost = Some(MattermostConfig {
        url: "https://mattermost.example".to_string(),
        bot_token: "test-token".to_string(),
        channel_id: None,
        allowed_users: vec![],
        thread_replies: None,
        mention_only: None,
        group_reply: None,
    });
    app.channels_settings.webhook = Some(WebhookConfig { port: 8_787, secret: None });
    app.channels_settings.imessage = Some(IMessageConfig { allowed_contacts: vec![] });
    app.channels_settings.matrix_enabled = true;
    app.channels_settings.matrix = Some(matrix_config());
    app.channels_settings.signal = Some(SignalConfig {
        http_url: "http://127.0.0.1:8686".to_string(),
        account: "+10000000000".to_string(),
        group_id: None,
        allowed_from: vec![],
        ignore_attachments: false,
        ignore_stories: false,
    });
    app.channels_settings.whatsapp = Some(WhatsAppConfig {
        access_token: None,
        phone_number_id: None,
        verify_token: None,
        app_secret: None,
        session_path: None,
        pair_phone: None,
        pair_code: None,
        allowed_numbers: vec![],
    });
    app.channels_settings.linq = Some(LinqConfig {
        api_token: "test-token".to_string(),
        from_phone: "+10000000001".to_string(),
        signing_secret: None,
        allowed_senders: vec![],
    });
    app.channels_settings.wati = Some(WatiConfig {
        api_token: "test-token".to_string(),
        api_url: "https://wati.example".to_string(),
        tenant_id: None,
        allowed_numbers: vec![],
    });
    app.channels_settings.nextcloud_talk = Some(NextcloudTalkConfig {
        base_url: "https://nextcloud.example".to_string(),
        app_token: "test-token".to_string(),
        webhook_secret: None,
        allowed_users: vec![],
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.channels_settings.email = Some(vw_config_types::channels::EmailConfig::default());
    }
    app.channels_settings.irc = Some(IrcConfig {
        server: "irc.example".to_string(),
        port: 6_697,
        nickname: "test-bot".to_string(),
        username: None,
        channels: vec![],
        allowed_users: vec![],
        server_password: None,
        nickserv_password: None,
        sasl_password: None,
        verify_tls: Some(true),
    });
    app.channels_settings.lark = Some(LarkConfig {
        app_id: "test-app".to_string(),
        app_secret: "test-secret".to_string(),
        encrypt_key: None,
        verification_token: None,
        allowed_users: vec![],
        mention_only: false,
        group_reply: None,
        use_feishu: false,
        receive_mode: LarkReceiveMode::Websocket,
        port: None,
        draft_update_interval_ms: 1_000,
        max_draft_edits: 3,
    });
    app.channels_settings.feishu = Some(feishu_config());
    app.channels_settings.dingtalk = Some(dingtalk_config());
    app.channels_settings.qq = Some(QQConfig {
        app_id: "test-app".to_string(),
        app_secret: "test-secret".to_string(),
        allowed_users: vec![],
        receive_mode: QQReceiveMode::Websocket,
    });
    app.channels_settings.nostr = Some(NostrConfig {
        private_key: "test-private-key".to_string(),
        relays: vec!["wss://relay.example".to_string()],
        allowed_pubkeys: vec![],
    });
    app.channels_settings.clawdtalk = Some(ClawdTalkConfig {
        api_key: "test-api-key".to_string(),
        connection_id: "test-connection".to_string(),
        from_number: "+10000000002".to_string(),
        allowed_destinations: vec![],
        webhook_secret: None,
    });
}

#[test]
fn multiline_list_keys_identify_collection_fields() {
    for key in [
        "telegram.allowed_users",
        "telegram.group_reply.allowed_sender_ids",
        "discord.allowed_users",
        "discord.group_reply.allowed_sender_ids",
        "slack.allowed_users",
        "slack.group_reply.allowed_sender_ids",
        "mattermost.allowed_users",
        "mattermost.group_reply.allowed_sender_ids",
        "imessage.allowed_contacts",
        "matrix.allowed_users",
        "signal.allowed_from",
        "whatsapp.allowed_numbers",
        "linq.allowed_senders",
        "wati.allowed_numbers",
        "nextcloud_talk.allowed_users",
        "email.allowed_senders",
        "irc.channels",
        "irc.allowed_users",
        "lark.allowed_users",
        "lark.group_reply.allowed_sender_ids",
        "feishu.allowed_users",
        "feishu.group_reply.allowed_sender_ids",
        "dingtalk.allowed_users",
        "qq.allowed_users",
        "nostr.relays",
        "nostr.allowed_pubkeys",
        "clawdtalk.allowed_destinations",
    ] {
        assert!(is_multiline_list_key(key), "{key} should be multiline");
    }
    assert!(!is_multiline_list_key("telegram.enabled"));
}

#[test]
fn named_instance_multiline_keys_require_supported_prefix_and_field() {
    for key in [
        "matrix_instances.primary.allowed_users",
        "feishu_instances.cn.allowed_users",
        "feishu_instances.cn.group_reply.allowed_sender_ids",
        "dingtalk_instances.ops.allowed_users",
    ] {
        assert!(is_multiline_list_key(key), "{key} should be multiline");
    }
    for key in [
        "matrix_instances",
        "matrix_instances.primary",
        "matrix_instances.primary.room_id",
        "feishu_instances.cn.group_reply.mode",
        "dingtalk_instances.ops.client_id",
        "slack_instances.primary.allowed_users",
    ] {
        assert!(!is_multiline_list_key(key), "{key} should remain single-line");
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
fn row_helpers_build_static_and_dynamic_single_line_multiline_and_control_rows() {
    let mut app = test_app();
    app.channels_settings.text_editors.insert(
        "telegram.allowed_users".to_string(),
        iced::widget::text_editor::Content::with_text("alice\nbob"),
    );
    app.channels_settings.text_editors.insert(
        "matrix_instances.primary.allowed_users".to_string(),
        iced::widget::text_editor::Content::with_text("@alice:matrix.example"),
    );
    app.channels_settings
        .text_inputs
        .insert("telegram.bot_token".to_string(), "edited-token".to_string());
    app.channels_settings.text_inputs.insert(
        "matrix_instances.primary.room_id".to_string(),
        "!edited:matrix.example".to_string(),
    );

    keep_element(text_row(
        &app,
        "bot_token",
        "Telegram token",
        "fallback-token",
        "telegram.bot_token",
        true,
    ));
    keep_element(text_row(
        &app,
        "allowed_users",
        "逗号或换行分隔",
        "",
        "telegram.allowed_users",
        false,
    ));
    keep_element(text_row_with_key(
        &app,
        "room_id",
        "Matrix room",
        "!fallback:matrix.example",
        "matrix_instances.primary.room_id".to_string(),
        false,
    ));
    keep_element(text_row_with_key(
        &app,
        "allowed_users",
        "逗号或换行分隔",
        "",
        "matrix_instances.primary.allowed_users".to_string(),
        false,
    ));
    keep_element(bool_row("mention_only", true, "仅提及时回复", "telegram.mention_only"));
    keep_element(bool_row_with_key(
        "verify_tls",
        false,
        "验证服务器证书",
        "irc_instances.primary.verify_tls".to_string(),
    ));
    keep_element(number_row("port", 3000, 1, 65_535, "", "lark.port"));
    keep_element(number_row("draft_update_interval_ms", 3000, 100, 60_000, "ms", "lark.delay"));
    keep_element(number_row_with_key(
        "port",
        3001,
        1,
        65_535,
        "",
        "feishu_instances.cn.port".to_string(),
    ));
    keep_element(pick_row(
        "receive_mode",
        "webhook",
        &LARK_RECEIVE_MODE_OPTIONS,
        "lark.receive_mode",
    ));
    keep_element(pick_row(
        "receive_mode",
        "missing",
        &LARK_RECEIVE_MODE_OPTIONS,
        "lark.receive_mode",
    ));
    keep_element(pick_row_with_key(
        "receive_mode",
        "websocket",
        &QQ_RECEIVE_MODE_OPTIONS,
        "qq_instances.primary.receive_mode".to_string(),
    ));
    keep_element(instance_title_row("Matrix · primary".to_string(), "matrix", 0));
    keep_element(add_instance_row("matrix", "添加 Matrix 实例"));
    keep_element(hint_row("提示"));
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
    keep_element(panel("x", "标题", "说明", false, false, text("body").into()));
    keep_element(panel("x", "标题", "说明", false, true, text("body").into()));
    keep_element(panel("x", "标题", "说明", true, true, text("body").into()));
}

#[test]
fn enabled_channels_returns_empty_when_every_channel_is_disabled() {
    let mut app = test_app();
    app.channels_settings.cli = false;

    assert!(enabled_channels(&app).is_empty());
}

#[test]
fn enabled_channels_reports_every_direct_channel_in_display_order() {
    let mut app = test_app();
    configure_all_direct_channels(&mut app);

    let mut expected = vec![
        "CLI",
        "Telegram",
        "Discord",
        "Slack",
        "Mattermost",
        "Webhook",
        "iMessage",
        "Matrix",
        "Signal",
        "WhatsApp",
        "Linq",
        "WATI",
        "Nextcloud Talk",
    ];
    #[cfg(not(target_arch = "wasm32"))]
    expected.push("Email");
    expected.extend(["IRC", "Lark", "Feishu", "DingTalk", "QQ", "Nostr", "ClawdTalk"]);

    assert_eq!(enabled_channels(&app), expected);
}

#[test]
fn enabled_channels_recognizes_named_instances_without_legacy_config() {
    let mut app = test_app();
    app.channels_settings.cli = false;
    app.channels_settings.matrix_enabled = true;
    app.channels_settings
        .matrix_instances
        .push(MatrixInstanceConfig { id: "primary".to_string(), config: matrix_config() });
    app.channels_settings
        .feishu_instances
        .push(FeishuInstanceConfig { id: "cn".to_string(), config: feishu_config() });
    app.channels_settings
        .dingtalk_instances
        .push(DingTalkInstanceConfig { id: "ops".to_string(), config: dingtalk_config() });

    assert_eq!(enabled_channels(&app), ["Matrix", "Feishu", "DingTalk"]);
}

#[test]
fn enabled_channels_does_not_duplicate_channels_with_legacy_and_named_instances() {
    let mut app = test_app();
    app.channels_settings.cli = false;
    app.channels_settings.matrix_enabled = true;
    app.channels_settings.matrix = Some(matrix_config());
    app.channels_settings
        .matrix_instances
        .push(MatrixInstanceConfig { id: "primary".to_string(), config: matrix_config() });
    app.channels_settings.feishu = Some(feishu_config());
    app.channels_settings
        .feishu_instances
        .push(FeishuInstanceConfig { id: "cn".to_string(), config: feishu_config() });
    app.channels_settings.dingtalk = Some(dingtalk_config());
    app.channels_settings
        .dingtalk_instances
        .push(DingTalkInstanceConfig { id: "ops".to_string(), config: dingtalk_config() });

    assert_eq!(enabled_channels(&app), ["Matrix", "Feishu", "DingTalk"]);
}
