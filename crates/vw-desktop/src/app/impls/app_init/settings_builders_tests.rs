use std::path::PathBuf;

use vw_config_types::channels::{
    ChannelsConfig, ClawdTalkConfig, DingTalkConfig, DingTalkInstanceConfig, DiscordConfig,
    FeishuConfig, FeishuInstanceConfig, GroupReplyConfig, IMessageConfig, IrcConfig, LarkConfig,
    LinqConfig, MatrixConfig, MatrixInstanceConfig, MattermostConfig, NextcloudTalkConfig,
    NostrConfig, QQConfig, SignalConfig, SlackConfig, TelegramConfig, WatiConfig, WebhookConfig,
    WhatsAppConfig,
};
use vw_config_types::staff::{StaffConfig, StaffMemberConfig};

#[test]
fn build_channels_settings_maps_empty_config_to_default_state() {
    let settings = super::build_channels_settings(&ChannelsConfig::default());

    assert!(settings.cli);
    assert_eq!(settings.project_dir_input, "");
    assert_eq!(settings.message_timeout_secs, 300);
    assert_eq!(settings.active_tab, super::super::state::ChannelsSettingsTab::Global);
    assert!(settings.text_inputs.is_empty());
    assert!(settings.text_editors.is_empty());
    assert!(settings.save_error.is_none());
}

#[test]
fn build_channels_settings_saturates_message_timeout_at_u32_max() {
    let config = ChannelsConfig { message_timeout_secs: u64::MAX, ..ChannelsConfig::default() };

    let settings = super::build_channels_settings(&config);

    assert_eq!(settings.message_timeout_secs, u32::MAX);
}

#[test]
fn build_channels_settings_preserves_zero_and_u32_max_message_timeout() {
    let zero = ChannelsConfig { message_timeout_secs: 0, ..ChannelsConfig::default() };
    let max = ChannelsConfig { message_timeout_secs: u32::MAX as u64, ..ChannelsConfig::default() };

    assert_eq!(super::build_channels_settings(&zero).message_timeout_secs, 0);
    assert_eq!(super::build_channels_settings(&max).message_timeout_secs, u32::MAX);
}

#[test]
fn build_channels_settings_preserves_project_dir_and_disabled_cli() {
    let config = ChannelsConfig {
        cli: false,
        project_dir: Some(PathBuf::from("/tmp/vibe-window/project")),
        ..ChannelsConfig::default()
    };

    let settings = super::build_channels_settings(&config);

    assert!(!settings.cli);
    assert_eq!(settings.project_dir_input, "/tmp/vibe-window/project");
    assert_eq!(settings.project_dir(), Some(PathBuf::from("/tmp/vibe-window/project")));
}

#[test]
fn build_channels_settings_defaults_to_global_tab() {
    let config = populated_channels_config();

    let settings = super::build_channels_settings(&config);

    assert_eq!(settings.active_tab, super::super::state::ChannelsSettingsTab::Global);
}

#[test]
fn build_channels_settings_clones_channel_configs_and_refreshes_text_inputs() {
    let config = populated_channels_config();

    let settings = super::build_channels_settings(&config);

    assert_eq!(settings.telegram.as_ref().unwrap().bot_token, "telegram-token");
    assert_eq!(settings.discord.as_ref().unwrap().bot_token, "discord-token");
    assert_eq!(settings.slack.as_ref().unwrap().bot_token, "slack-token");
    assert_eq!(settings.mattermost.as_ref().unwrap().url, "https://mattermost.test");
    assert_eq!(settings.webhook.as_ref().unwrap().port, 9876);
    assert_eq!(
        settings.imessage.as_ref().unwrap().allowed_contacts,
        vec!["+10000000000".to_string()]
    );
    assert_eq!(settings.matrix.as_ref().unwrap().room_id, "!room:matrix.test");
    assert_eq!(settings.signal.as_ref().unwrap().account, "+12223334444");
    assert_eq!(settings.whatsapp.as_ref().unwrap().session_path, Some("/tmp/wa".to_string()));
    assert_eq!(settings.linq.as_ref().unwrap().from_phone, "+15550000000");
    assert_eq!(settings.wati.as_ref().unwrap().tenant_id, Some("tenant".to_string()));
    assert_eq!(settings.nextcloud_talk.as_ref().unwrap().base_url, "https://cloud.test");
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(settings.email.as_ref().unwrap().username, "mail-user");
    assert_eq!(settings.irc.as_ref().unwrap().nickname, "vibe");
    assert_eq!(settings.lark.as_ref().unwrap().app_id, "lark-app");
    assert_eq!(settings.feishu.as_ref().unwrap().app_id, "feishu-app");
    assert_eq!(settings.dingtalk.as_ref().unwrap().client_id, "dingtalk-client");
    assert_eq!(settings.qq.as_ref().unwrap().app_id, "qq-app");
    assert_eq!(settings.nostr.as_ref().unwrap().private_key, "nostr-key");
    assert_eq!(settings.clawdtalk.as_ref().unwrap().connection_id, "clawd-connection");

    assert_eq!(settings.text_inputs["telegram.allowed_users"], "alice, bob");
    assert_eq!(settings.text_inputs["telegram.group_reply.allowed_sender_ids"], "tg-group");
    assert_eq!(settings.text_inputs["discord.allowed_users"], "discord-user");
    assert_eq!(settings.text_inputs["slack.allowed_users"], "slack-user");
    assert_eq!(settings.text_inputs["mattermost.allowed_users"], "mattermost-user");
    assert_eq!(settings.text_inputs["imessage.allowed_contacts"], "+10000000000");
    assert_eq!(settings.text_inputs["matrix.allowed_users"], "@alice:matrix.test");
    assert_eq!(settings.text_inputs["signal.allowed_from"], "+12223334444");
    assert_eq!(settings.text_inputs["whatsapp.allowed_numbers"], "+13334445555");
    assert_eq!(settings.text_inputs["linq.allowed_senders"], "+14445556666");
    assert_eq!(settings.text_inputs["wati.allowed_numbers"], "+15556667777");
    assert_eq!(settings.text_inputs["nextcloud_talk.allowed_users"], "cloud-user");
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(settings.text_inputs["email.allowed_senders"], "sender@example.com");
    assert_eq!(settings.text_inputs["irc.channels"], "#vibe");
    assert_eq!(settings.text_inputs["irc.allowed_users"], "irc-user");
    assert_eq!(settings.text_inputs["lark.allowed_users"], "lark-user");
    assert_eq!(settings.text_inputs["lark.group_reply.allowed_sender_ids"], "lark-group");
    assert_eq!(settings.text_inputs["feishu.allowed_users"], "feishu-user");
    assert_eq!(settings.text_inputs["feishu.group_reply.allowed_sender_ids"], "feishu-group");
    assert_eq!(settings.text_inputs["dingtalk.allowed_users"], "dingtalk-user");
    assert_eq!(settings.text_inputs["qq.allowed_users"], "qq-user");
    assert_eq!(settings.text_inputs["nostr.relays"], "wss://relay.test");
    assert_eq!(settings.text_inputs["nostr.allowed_pubkeys"], "nostr-pubkey");
    assert_eq!(settings.text_inputs["clawdtalk.allowed_destinations"], "+16667778888");
}

#[test]
fn build_staff_settings_uses_full_agent_staff_config() {
    let mut full_agent_cfg = vw_config_types::config::Config::default();
    full_agent_cfg.staff = StaffConfig {
        members: vec![StaffMemberConfig {
            id: "planner".to_string(),
            name: "Planner".to_string(),
            role: "planner".to_string(),
            level: "manager".to_string(),
            status: "available".to_string(),
            enabled: true,
            capacity: 2,
            delegate_agent: Some("main".to_string()),
            acp_agent: None,
            ipc_agent_id: None,
            channel_bindings: Vec::new(),
            capabilities: Default::default(),
        }],
        ..StaffConfig::default()
    };

    let settings = super::build_staff_settings(&full_agent_cfg);

    assert_eq!(settings.config, full_agent_cfg.staff);
    assert_eq!(settings.selected_member_id.as_deref(), Some("planner"));
    assert_eq!(settings.member_drafts.len(), 1);
    assert_eq!(settings.capability_drafts.len(), 1);
    assert!(!settings.loading);
}

mod channel_instance_settings {
    use super::*;

    #[test]
    fn channels_settings_builds_named_channel_instances() {
        let mut config = ChannelsConfig::default();
        config.matrix_enabled = true;
        config.matrix_instances.push(MatrixInstanceConfig {
            id: "planner".to_string(),
            config: MatrixConfig {
                homeserver: "https://matrix.profile.test".to_string(),
                access_token: "matrix-profile-token".to_string(),
                user_id: Some("@planner:matrix.profile.test".to_string()),
                device_id: Some("PLANNER".to_string()),
                room_id: "!planner:matrix.profile.test".to_string(),
                allowed_users: vec!["@alice:matrix.profile.test".to_string()],
                mention_only: true,
            },
        });
        config.feishu_instances.push(FeishuInstanceConfig {
            id: "cn".to_string(),
            config: FeishuConfig {
                app_id: "feishu-profile-app".to_string(),
                app_secret: "feishu-profile-secret".to_string(),
                encrypt_key: Some("feishu-profile-encrypt".to_string()),
                verification_token: Some("feishu-profile-verify".to_string()),
                allowed_users: vec!["feishu-profile-user".to_string()],
                group_reply: Some(group_reply("feishu-profile-group")),
                receive_mode: Default::default(),
                port: Some(8088),
                draft_update_interval_ms: 1234,
                max_draft_edits: 7,
            },
        });
        config.dingtalk_instances.push(DingTalkInstanceConfig {
            id: "ops".to_string(),
            config: DingTalkConfig {
                client_id: "dingtalk-profile-client".to_string(),
                client_secret: "dingtalk-profile-secret".to_string(),
                allowed_users: vec!["dingtalk-profile-user".to_string()],
            },
        });

        let settings = super::super::build_channels_settings(&config);

        assert_eq!(settings.matrix_instances[0].id, "planner");
        assert_eq!(settings.matrix_instances[0].config.room_id, "!planner:matrix.profile.test");
        assert_eq!(settings.feishu_instances[0].id, "cn");
        assert_eq!(settings.feishu_instances[0].config.app_id, "feishu-profile-app");
        assert_eq!(settings.dingtalk_instances[0].id, "ops");
        assert_eq!(settings.dingtalk_instances[0].config.client_id, "dingtalk-profile-client");
        assert_eq!(
            settings.text_inputs["matrix_instances.0.allowed_users"],
            "@alice:matrix.profile.test"
        );
        assert_eq!(settings.text_inputs["feishu_instances.0.allowed_users"], "feishu-profile-user");
        assert_eq!(
            settings.text_inputs["feishu_instances.0.group_reply.allowed_sender_ids"],
            "feishu-profile-group"
        );
        assert_eq!(
            settings.text_inputs["dingtalk_instances.0.allowed_users"],
            "dingtalk-profile-user"
        );
        assert_eq!(settings.active_tab, crate::app::state::ChannelsSettingsTab::Global);
    }
}

fn populated_channels_config() -> ChannelsConfig {
    ChannelsConfig {
        cli: false,
        telegram: Some(TelegramConfig {
            bot_token: "telegram-token".to_string(),
            allowed_users: vec!["alice".to_string(), "bob".to_string()],
            stream_mode: Default::default(),
            draft_update_interval_ms: 1000,
            interrupt_on_new_message: false,
            mention_only: false,
            group_reply: Some(group_reply("tg-group")),
            base_url: None,
        }),
        discord: Some(DiscordConfig {
            bot_token: "discord-token".to_string(),
            guild_id: Some("guild".to_string()),
            allowed_users: vec!["discord-user".to_string()],
            listen_to_bots: false,
            mention_only: false,
            group_reply: Some(group_reply("discord-group")),
        }),
        slack: Some(SlackConfig {
            bot_token: "slack-token".to_string(),
            app_token: None,
            channel_id: Some("channel".to_string()),
            allowed_users: vec!["slack-user".to_string()],
            group_reply: Some(group_reply("slack-group")),
        }),
        mattermost: Some(MattermostConfig {
            url: "https://mattermost.test".to_string(),
            bot_token: "mattermost-token".to_string(),
            channel_id: Some("mattermost-channel".to_string()),
            allowed_users: vec!["mattermost-user".to_string()],
            thread_replies: Some(true),
            mention_only: Some(false),
            group_reply: Some(group_reply("mattermost-group")),
        }),
        webhook: Some(WebhookConfig { port: 9876, secret: Some("webhook-secret".to_string()) }),
        imessage: Some(IMessageConfig { allowed_contacts: vec!["+10000000000".to_string()] }),
        matrix: Some(MatrixConfig {
            homeserver: "https://matrix.test".to_string(),
            access_token: "matrix-token".to_string(),
            user_id: Some("@bot:matrix.test".to_string()),
            device_id: Some("device".to_string()),
            room_id: "!room:matrix.test".to_string(),
            allowed_users: vec!["@alice:matrix.test".to_string()],
            mention_only: false,
        }),
        matrix_enabled: true,
        matrix_instances: Vec::new(),
        signal: Some(SignalConfig {
            http_url: "http://signal.test".to_string(),
            account: "+12223334444".to_string(),
            group_id: Some("signal-group".to_string()),
            allowed_from: vec!["+12223334444".to_string()],
            ignore_attachments: false,
            ignore_stories: false,
        }),
        whatsapp: Some(WhatsAppConfig {
            access_token: Some("whatsapp-token".to_string()),
            phone_number_id: None,
            verify_token: Some("verify".to_string()),
            app_secret: Some("app-secret".to_string()),
            session_path: Some("/tmp/wa".to_string()),
            pair_phone: None,
            pair_code: None,
            allowed_numbers: vec!["+13334445555".to_string()],
        }),
        linq: Some(LinqConfig {
            api_token: "linq-token".to_string(),
            from_phone: "+15550000000".to_string(),
            signing_secret: Some("linq-secret".to_string()),
            allowed_senders: vec!["+14445556666".to_string()],
        }),
        wati: Some(WatiConfig {
            api_token: "wati-token".to_string(),
            api_url: "https://wati.test".to_string(),
            tenant_id: Some("tenant".to_string()),
            allowed_numbers: vec!["+15556667777".to_string()],
        }),
        nextcloud_talk: Some(NextcloudTalkConfig {
            base_url: "https://cloud.test".to_string(),
            app_token: "cloud-token".to_string(),
            webhook_secret: Some("cloud-secret".to_string()),
            allowed_users: vec!["cloud-user".to_string()],
        }),
        #[cfg(not(target_arch = "wasm32"))]
        email: Some(email_config()),
        irc: Some(IrcConfig {
            server: "irc.test".to_string(),
            port: 6697,
            nickname: "vibe".to_string(),
            username: Some("vibe-user".to_string()),
            channels: vec!["#vibe".to_string()],
            allowed_users: vec!["irc-user".to_string()],
            server_password: None,
            nickserv_password: None,
            sasl_password: None,
            verify_tls: Some(true),
        }),
        lark: Some(LarkConfig {
            app_id: "lark-app".to_string(),
            app_secret: "lark-secret".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: vec!["lark-user".to_string()],
            mention_only: false,
            group_reply: Some(group_reply("lark-group")),
            use_feishu: false,
            receive_mode: Default::default(),
            port: Some(8081),
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        }),
        feishu: Some(FeishuConfig {
            app_id: "feishu-app".to_string(),
            app_secret: "feishu-secret".to_string(),
            encrypt_key: None,
            verification_token: None,
            allowed_users: vec!["feishu-user".to_string()],
            group_reply: Some(group_reply("feishu-group")),
            receive_mode: Default::default(),
            port: Some(8082),
            draft_update_interval_ms: 3000,
            max_draft_edits: 20,
        }),
        feishu_instances: Vec::new(),
        dingtalk: Some(DingTalkConfig {
            client_id: "dingtalk-client".to_string(),
            client_secret: "dingtalk-secret".to_string(),
            allowed_users: vec!["dingtalk-user".to_string()],
        }),
        dingtalk_instances: Vec::new(),
        qq: Some(QQConfig {
            app_id: "qq-app".to_string(),
            app_secret: "qq-secret".to_string(),
            allowed_users: vec!["qq-user".to_string()],
            receive_mode: Default::default(),
        }),
        nostr: Some(NostrConfig {
            private_key: "nostr-key".to_string(),
            relays: vec!["wss://relay.test".to_string()],
            allowed_pubkeys: vec!["nostr-pubkey".to_string()],
        }),
        clawdtalk: Some(ClawdTalkConfig {
            api_key: "clawd-key".to_string(),
            connection_id: "clawd-connection".to_string(),
            from_number: "+15550001111".to_string(),
            allowed_destinations: vec!["+16667778888".to_string()],
            webhook_secret: Some("clawd-secret".to_string()),
        }),
        message_timeout_secs: 42,
        project_dir: Some(PathBuf::from("/tmp/vibe-window/project")),
    }
}

fn group_reply(sender_id: &str) -> GroupReplyConfig {
    GroupReplyConfig { mode: None, allowed_sender_ids: vec![sender_id.to_string()] }
}

#[cfg(not(target_arch = "wasm32"))]
fn email_config() -> vw_config_types::channels::EmailConfig {
    vw_config_types::channels::EmailConfig {
        imap_host: "imap.test".to_string(),
        smtp_host: "smtp.test".to_string(),
        username: "mail-user".to_string(),
        password: "mail-password".to_string(),
        from_address: "bot@example.com".to_string(),
        allowed_senders: vec!["sender@example.com".to_string()],
        ..Default::default()
    }
}
