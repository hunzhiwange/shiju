//! 渠道设置测试，覆盖多实例字段分类、文本编辑器刷新和项目目录规范化。
//! 重建输入状态时应保留静态编辑器并清除已删除实例留下的陈旧草稿。

use super::*;

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("channels_tests"));
}

fn values(prefix: &str) -> Vec<String> {
    vec![format!("{prefix}-1"), format!("{prefix}-2")]
}

fn group(prefix: &str) -> vw_config_types::channels::GroupReplyConfig {
    vw_config_types::channels::GroupReplyConfig { mode: None, allowed_sender_ids: values(prefix) }
}

#[test]
fn named_instance_field_accepts_complete_instance_keys_and_rejects_malformed_keys() {
    assert_eq!(
        named_instance_field("matrix_instances.2.allowed_users", "matrix_instances"),
        Some("allowed_users")
    );
    assert_eq!(
        named_instance_field(
            "feishu_instances.primary.group_reply.allowed_sender_ids",
            "feishu_instances"
        ),
        Some("group_reply.allowed_sender_ids")
    );
    assert_eq!(named_instance_field("matrix_instances", "matrix_instances"), None);
    assert_eq!(named_instance_field("matrix_instances.0", "matrix_instances"), None);
    assert_eq!(named_instance_field("matrix_instances0.allowed_users", "matrix_instances"), None);
    assert_eq!(named_instance_field("feishu_instances.0.allowed_users", "matrix_instances"), None);
}

#[test]
fn multiline_channel_key_classifies_static_and_supported_instance_fields() {
    assert!(is_multiline_channel_key("telegram.allowed_users"));
    assert!(is_multiline_channel_key("irc.channels"));
    assert!(is_multiline_channel_key("matrix_instances.0.allowed_users"));
    assert!(is_multiline_channel_key("feishu_instances.cn.group_reply.allowed_sender_ids"));
    assert!(is_multiline_channel_key("dingtalk_instances.ops.allowed_users"));

    assert!(!is_multiline_channel_key("telegram.bot_token"));
    assert!(!is_multiline_channel_key("matrix_instances.0.room_id"));
    assert!(!is_multiline_channel_key("dingtalk_instances.ops.group_reply.allowed_sender_ids"));
    assert!(!is_multiline_channel_key("unknown.allowed_users"));
}

#[test]
fn static_and_named_multiline_key_helpers_cover_known_catalogs() {
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
        assert!(is_static_multiline_channel_key(key), "expected static multiline key: {key}");
        assert!(is_multiline_channel_key(key), "expected multiline key: {key}");
    }

    for key in [
        "matrix_instances.ops.allowed_users",
        "feishu_instances.cn.allowed_users",
        "feishu_instances.cn.group_reply.allowed_sender_ids",
        "dingtalk_instances.ops.allowed_users",
    ] {
        assert!(is_named_instance_multiline_channel_key(key), "expected named key: {key}");
        assert!(is_multiline_channel_key(key), "expected multiline key: {key}");
    }

    for key in [
        "matrix.allowed_user",
        "matrix_instances.ops.room_id",
        "feishu_instances.cn.group_reply.mode",
        "dingtalk_instances.ops.client_id",
    ] {
        assert!(!is_static_multiline_channel_key(key), "unexpected static key: {key}");
        assert!(!is_named_instance_multiline_channel_key(key), "unexpected named key: {key}");
    }
}

#[test]
fn default_channels_state_matches_ui_defaults() {
    let state = ChannelsSettingsState::default();

    assert_eq!(state.project_dir(), None);
    assert!(state.cli);
    assert_eq!(state.message_timeout_secs, 300);
    assert_eq!(state.active_tab, ChannelsSettingsTab::Global);
    assert!(state.save_error.is_none());
    assert!(state.text_inputs.is_empty());
    assert!(state.text_editors.is_empty());
}

#[test]
fn project_dir_trims_blank_input_and_preserves_non_blank_path() {
    let mut state = ChannelsSettingsState {
        project_dir_input: "   ".to_string(),
        ..ChannelsSettingsState::default()
    };

    assert_eq!(state.project_dir(), None);

    state.project_dir_input = "  /tmp/vibe-window  ".to_string();
    assert_eq!(state.project_dir(), Some(PathBuf::from("/tmp/vibe-window")));
}

#[test]
fn refresh_text_inputs_rebuilds_all_channel_list_fields() {
    let mut state = ChannelsSettingsState::default();
    state.text_inputs.insert("telegram.allowed_users".to_string(), "stale".to_string());
    state.text_inputs.insert("unrelated".to_string(), "keep".to_string());
    state.text_editors.insert("telegram.allowed_users".to_string(), text_editor::Content::new());
    state.text_editors.insert("unrelated".to_string(), text_editor::Content::new());

    state.telegram = Some(vw_config_types::channels::TelegramConfig {
        bot_token: "telegram-token".to_string(),
        allowed_users: values("telegram-user"),
        stream_mode: vw_config_types::channels::StreamMode::Partial,
        draft_update_interval_ms: 250,
        interrupt_on_new_message: true,
        mention_only: true,
        group_reply: Some(group("telegram-sender")),
        base_url: Some("https://telegram.example".to_string()),
    });
    state.discord = Some(vw_config_types::channels::DiscordConfig {
        bot_token: "discord-token".to_string(),
        guild_id: Some("guild".to_string()),
        allowed_users: values("discord-user"),
        listen_to_bots: true,
        mention_only: true,
        group_reply: Some(group("discord-sender")),
    });
    state.slack = Some(vw_config_types::channels::SlackConfig {
        bot_token: "slack-token".to_string(),
        app_token: Some("app".to_string()),
        channel_id: Some("channel".to_string()),
        allowed_users: values("slack-user"),
        group_reply: Some(group("slack-sender")),
    });
    state.mattermost = Some(vw_config_types::channels::MattermostConfig {
        url: "https://mattermost.example".to_string(),
        bot_token: "mattermost-token".to_string(),
        channel_id: Some("channel".to_string()),
        allowed_users: values("mattermost-user"),
        thread_replies: Some(true),
        mention_only: Some(true),
        group_reply: Some(group("mattermost-sender")),
    });
    state.imessage = Some(vw_config_types::channels::IMessageConfig {
        allowed_contacts: values("imessage-contact"),
    });
    state.matrix = Some(vw_config_types::channels::MatrixConfig {
        homeserver: "https://matrix.example".to_string(),
        access_token: "matrix-token".to_string(),
        user_id: Some("@user:example".to_string()),
        device_id: Some("device".to_string()),
        room_id: "!room:example".to_string(),
        allowed_users: values("matrix-user"),
        mention_only: true,
    });
    state.signal = Some(vw_config_types::channels::SignalConfig {
        http_url: "http://signal.example".to_string(),
        account: "+1000".to_string(),
        group_id: Some("group".to_string()),
        allowed_from: values("signal-from"),
        ignore_attachments: true,
        ignore_stories: true,
    });
    state.whatsapp = Some(vw_config_types::channels::WhatsAppConfig {
        access_token: Some("wa-token".to_string()),
        phone_number_id: Some("phone-id".to_string()),
        verify_token: Some("verify".to_string()),
        app_secret: Some("secret".to_string()),
        session_path: None,
        pair_phone: None,
        pair_code: None,
        allowed_numbers: values("whatsapp-number"),
    });
    state.linq = Some(vw_config_types::channels::LinqConfig {
        api_token: "linq-token".to_string(),
        from_phone: "+1001".to_string(),
        signing_secret: Some("secret".to_string()),
        allowed_senders: values("linq-sender"),
    });
    state.wati = Some(vw_config_types::channels::WatiConfig {
        api_token: "wati-token".to_string(),
        api_url: "https://wati.example".to_string(),
        tenant_id: Some("tenant".to_string()),
        allowed_numbers: values("wati-number"),
    });
    state.nextcloud_talk = Some(vw_config_types::channels::NextcloudTalkConfig {
        base_url: "https://nextcloud.example".to_string(),
        app_token: "nextcloud-token".to_string(),
        webhook_secret: Some("secret".to_string()),
        allowed_users: values("nextcloud-user"),
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        state.email = Some(vw_config_types::channels::EmailConfig {
            allowed_senders: values("email-sender"),
            ..vw_config_types::channels::EmailConfig::default()
        });
    }
    state.irc = Some(vw_config_types::channels::IrcConfig {
        server: "irc.example".to_string(),
        port: 6697,
        nickname: "bot".to_string(),
        username: Some("user".to_string()),
        channels: values("irc-channel"),
        allowed_users: values("irc-user"),
        server_password: Some("server-pass".to_string()),
        nickserv_password: Some("nick-pass".to_string()),
        sasl_password: Some("sasl-pass".to_string()),
        verify_tls: Some(true),
    });
    state.lark = Some(vw_config_types::channels::LarkConfig {
        app_id: "lark-app".to_string(),
        app_secret: "lark-secret".to_string(),
        encrypt_key: Some("encrypt".to_string()),
        verification_token: Some("verify".to_string()),
        allowed_users: values("lark-user"),
        mention_only: true,
        group_reply: Some(group("lark-sender")),
        use_feishu: false,
        receive_mode: vw_config_types::channels::LarkReceiveMode::Webhook,
        port: Some(80),
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    });
    state.feishu = Some(vw_config_types::channels::FeishuConfig {
        app_id: "feishu-app".to_string(),
        app_secret: "feishu-secret".to_string(),
        encrypt_key: Some("encrypt".to_string()),
        verification_token: Some("verify".to_string()),
        allowed_users: values("feishu-user"),
        group_reply: Some(group("feishu-sender")),
        receive_mode: vw_config_types::channels::LarkReceiveMode::Webhook,
        port: Some(81),
        draft_update_interval_ms: 3000,
        max_draft_edits: 20,
    });
    state.dingtalk = Some(vw_config_types::channels::DingTalkConfig {
        client_id: "dingtalk-client".to_string(),
        client_secret: "dingtalk-secret".to_string(),
        allowed_users: values("dingtalk-user"),
    });
    state.qq = Some(vw_config_types::channels::QQConfig {
        app_id: "qq-app".to_string(),
        app_secret: "qq-secret".to_string(),
        allowed_users: values("qq-user"),
        receive_mode: vw_config_types::channels::QQReceiveMode::Websocket,
    });
    state.nostr = Some(vw_config_types::channels::NostrConfig {
        private_key: "nostr-key".to_string(),
        relays: values("nostr-relay"),
        allowed_pubkeys: values("nostr-pubkey"),
    });
    state.clawdtalk = Some(vw_config_types::channels::ClawdTalkConfig {
        api_key: "clawd-key".to_string(),
        connection_id: "conn".to_string(),
        from_number: "+1002".to_string(),
        allowed_destinations: values("clawd-destination"),
        webhook_secret: Some("secret".to_string()),
    });

    state.refresh_text_inputs();

    assert_eq!(state.text_inputs["unrelated"], "keep");
    assert!(!state.text_editors.contains_key("unrelated"));
    assert_eq!(state.text_inputs["telegram.allowed_users"], "telegram-user-1, telegram-user-2");
    assert_eq!(
        state.text_editors["telegram.allowed_users"].text(),
        "telegram-user-1\ntelegram-user-2"
    );
    assert_eq!(
        state.text_inputs["telegram.group_reply.allowed_sender_ids"],
        "telegram-sender-1, telegram-sender-2"
    );
    assert_eq!(state.text_inputs["discord.allowed_users"], "discord-user-1, discord-user-2");
    assert_eq!(
        state.text_inputs["slack.group_reply.allowed_sender_ids"],
        "slack-sender-1, slack-sender-2"
    );
    assert_eq!(
        state.text_inputs["mattermost.group_reply.allowed_sender_ids"],
        "mattermost-sender-1, mattermost-sender-2"
    );
    assert_eq!(
        state.text_inputs["imessage.allowed_contacts"],
        "imessage-contact-1, imessage-contact-2"
    );
    assert_eq!(state.text_inputs["matrix.allowed_users"], "matrix-user-1, matrix-user-2");
    assert_eq!(state.text_inputs["signal.allowed_from"], "signal-from-1, signal-from-2");
    assert_eq!(
        state.text_inputs["whatsapp.allowed_numbers"],
        "whatsapp-number-1, whatsapp-number-2"
    );
    assert_eq!(state.text_inputs["linq.allowed_senders"], "linq-sender-1, linq-sender-2");
    assert_eq!(state.text_inputs["wati.allowed_numbers"], "wati-number-1, wati-number-2");
    assert_eq!(
        state.text_inputs["nextcloud_talk.allowed_users"],
        "nextcloud-user-1, nextcloud-user-2"
    );
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(state.text_inputs["email.allowed_senders"], "email-sender-1, email-sender-2");
    assert_eq!(state.text_inputs["irc.channels"], "irc-channel-1, irc-channel-2");
    assert_eq!(state.text_inputs["irc.allowed_users"], "irc-user-1, irc-user-2");
    assert_eq!(state.text_inputs["lark.allowed_users"], "lark-user-1, lark-user-2");
    assert_eq!(
        state.text_inputs["lark.group_reply.allowed_sender_ids"],
        "lark-sender-1, lark-sender-2"
    );
    assert_eq!(state.text_inputs["feishu.allowed_users"], "feishu-user-1, feishu-user-2");
    assert_eq!(
        state.text_inputs["feishu.group_reply.allowed_sender_ids"],
        "feishu-sender-1, feishu-sender-2"
    );
    assert_eq!(state.text_inputs["dingtalk.allowed_users"], "dingtalk-user-1, dingtalk-user-2");
    assert_eq!(state.text_inputs["qq.allowed_users"], "qq-user-1, qq-user-2");
    assert_eq!(state.text_inputs["nostr.relays"], "nostr-relay-1, nostr-relay-2");
    assert_eq!(state.text_inputs["nostr.allowed_pubkeys"], "nostr-pubkey-1, nostr-pubkey-2");
    assert_eq!(
        state.text_inputs["clawdtalk.allowed_destinations"],
        "clawd-destination-1, clawd-destination-2"
    );
}

#[test]
fn refresh_text_inputs_inserts_empty_group_reply_when_config_omits_group_reply() {
    let mut state = ChannelsSettingsState {
        telegram: Some(vw_config_types::channels::TelegramConfig {
            bot_token: "token".to_string(),
            allowed_users: Vec::new(),
            stream_mode: vw_config_types::channels::StreamMode::Off,
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: None,
            base_url: None,
        }),
        ..ChannelsSettingsState::default()
    };

    state.refresh_text_inputs();

    assert_eq!(state.text_inputs["telegram.allowed_users"], "");
    assert_eq!(state.text_inputs["telegram.group_reply.allowed_sender_ids"], "");
    assert_eq!(state.text_editors["telegram.group_reply.allowed_sender_ids"].text(), "");
}

#[test]
fn refresh_text_inputs_inserts_empty_group_reply_for_other_optional_group_configs() {
    let mut state = ChannelsSettingsState {
        discord: Some(vw_config_types::channels::DiscordConfig {
            bot_token: "token".to_string(),
            guild_id: None,
            allowed_users: Vec::new(),
            listen_to_bots: false,
            mention_only: false,
            group_reply: None,
        }),
        slack: Some(vw_config_types::channels::SlackConfig {
            bot_token: "token".to_string(),
            app_token: None,
            channel_id: None,
            allowed_users: Vec::new(),
            group_reply: None,
        }),
        mattermost: Some(vw_config_types::channels::MattermostConfig {
            url: "https://mattermost.example".to_string(),
            bot_token: "token".to_string(),
            channel_id: None,
            allowed_users: Vec::new(),
            thread_replies: None,
            mention_only: None,
            group_reply: None,
        }),
        lark: Some(vw_config_types::channels::LarkConfig {
            app_id: "app".to_string(),
            app_secret: "secret".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: Vec::new(),
            mention_only: false,
            group_reply: None,
            use_feishu: false,
            receive_mode: vw_config_types::channels::LarkReceiveMode::Websocket,
            port: None,
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        }),
        feishu: Some(vw_config_types::channels::FeishuConfig {
            app_id: "app".to_string(),
            app_secret: "secret".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: Vec::new(),
            group_reply: None,
            receive_mode: vw_config_types::channels::LarkReceiveMode::Websocket,
            port: None,
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        }),
        ..ChannelsSettingsState::default()
    };
    state.feishu_instances.push(vw_config_types::channels::FeishuInstanceConfig {
        id: "cn".to_string(),
        config: state.feishu.clone().expect("feishu fixture"),
    });

    state.refresh_text_inputs();

    for key in [
        "discord.group_reply.allowed_sender_ids",
        "slack.group_reply.allowed_sender_ids",
        "mattermost.group_reply.allowed_sender_ids",
        "lark.group_reply.allowed_sender_ids",
        "feishu.group_reply.allowed_sender_ids",
        "feishu_instances.0.group_reply.allowed_sender_ids",
    ] {
        assert_eq!(state.text_inputs[key], "", "unexpected input for {key}");
        assert_eq!(state.text_editors[key].text(), "", "unexpected editor for {key}");
    }
}

#[test]
fn refresh_text_inputs_preserves_static_editors_and_discards_stale_instance_editors() {
    let mut state = ChannelsSettingsState::default();
    state.text_inputs.insert("telegram.allowed_users".to_string(), "stale input".to_string());
    state.text_editors.insert(
        "telegram.allowed_users".to_string(),
        text_editor::Content::with_text("draft static editor"),
    );
    state.text_editors.insert(
        "matrix_instances.4.allowed_users".to_string(),
        text_editor::Content::with_text("stale instance editor"),
    );
    state
        .text_editors
        .insert("unrelated".to_string(), text_editor::Content::with_text("unrelated editor"));

    state.refresh_text_inputs();

    assert!(!state.text_inputs.contains_key("telegram.allowed_users"));
    assert_eq!(state.text_editors["telegram.allowed_users"].text(), "draft static editor");
    assert!(!state.text_editors.contains_key("matrix_instances.4.allowed_users"));
    assert!(!state.text_editors.contains_key("unrelated"));
}

#[test]
fn channel_instance_settings_refreshes_named_instance_list_inputs() {
    let mut state = ChannelsSettingsState::default();
    state.text_inputs.insert("matrix_instances.1.allowed_users".to_string(), "stale".to_string());
    state
        .text_editors
        .insert("matrix_instances.1.allowed_users".to_string(), text_editor::Content::new());
    state.matrix_instances.push(vw_config_types::channels::MatrixInstanceConfig {
        id: "planner".to_string(),
        config: vw_config_types::channels::MatrixConfig {
            homeserver: "https://matrix.example".to_string(),
            access_token: "token".to_string(),
            user_id: None,
            device_id: None,
            room_id: "!room:example".to_string(),
            allowed_users: values("matrix-instance-user"),
            mention_only: false,
        },
    });
    state.feishu_instances.push(vw_config_types::channels::FeishuInstanceConfig {
        id: "cn".to_string(),
        config: vw_config_types::channels::FeishuConfig {
            app_id: "app".to_string(),
            app_secret: "secret".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: values("feishu-instance-user"),
            group_reply: Some(group("feishu-instance-sender")),
            receive_mode: vw_config_types::channels::LarkReceiveMode::Websocket,
            port: None,
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        },
    });
    state.dingtalk_instances.push(vw_config_types::channels::DingTalkInstanceConfig {
        id: "ops".to_string(),
        config: vw_config_types::channels::DingTalkConfig {
            client_id: "client".to_string(),
            client_secret: "secret".to_string(),
            allowed_users: values("dingtalk-instance-user"),
        },
    });

    state.refresh_text_inputs();

    assert_eq!(
        state.text_inputs["matrix_instances.0.allowed_users"],
        "matrix-instance-user-1, matrix-instance-user-2"
    );
    assert!(!state.text_inputs.contains_key("matrix_instances.1.allowed_users"));
    assert_eq!(
        state.text_inputs["feishu_instances.0.group_reply.allowed_sender_ids"],
        "feishu-instance-sender-1, feishu-instance-sender-2"
    );
    assert_eq!(
        state.text_editors["dingtalk_instances.0.allowed_users"].text(),
        "dingtalk-instance-user-1\ndingtalk-instance-user-2"
    );
}
