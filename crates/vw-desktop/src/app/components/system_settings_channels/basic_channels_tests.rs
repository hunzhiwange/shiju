use crate::app::{App, Message};
use vw_config_types::channels::{
    DingTalkConfig, DingTalkInstanceConfig, DiscordConfig, FeishuConfig, FeishuInstanceConfig,
    GroupReplyConfig, GroupReplyMode, IMessageConfig, MatrixConfig, MatrixInstanceConfig,
    MattermostConfig, SignalConfig, SlackConfig, TelegramConfig, WebhookConfig,
};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn group_reply() -> GroupReplyConfig {
    GroupReplyConfig {
        mode: Some(GroupReplyMode::MentionOnly),
        allowed_sender_ids: vec!["alice".to_string(), "bob".to_string()],
    }
}

fn configure_basic_channels(app: &mut App) {
    app.channels_settings.telegram = Some(TelegramConfig {
        bot_token: "telegram-token".to_string(),
        allowed_users: vec!["telegram-user".to_string()],
        stream_mode: Default::default(),
        draft_update_interval_ms: 500,
        interrupt_on_new_message: true,
        mention_only: true,
        group_reply: Some(group_reply()),
        base_url: Some("https://telegram.example".to_string()),
    });
    app.channels_settings.discord = Some(DiscordConfig {
        bot_token: "discord-token".to_string(),
        guild_id: Some("guild".to_string()),
        allowed_users: vec!["discord-user".to_string()],
        listen_to_bots: true,
        mention_only: true,
        group_reply: Some(group_reply()),
    });
    app.channels_settings.slack = Some(SlackConfig {
        bot_token: "slack-token".to_string(),
        app_token: Some("xapp".to_string()),
        channel_id: Some("channel".to_string()),
        allowed_users: vec!["slack-user".to_string()],
        group_reply: Some(group_reply()),
    });
    app.channels_settings.mattermost = Some(MattermostConfig {
        url: "https://mattermost.example".to_string(),
        bot_token: "mattermost-token".to_string(),
        channel_id: Some("channel".to_string()),
        allowed_users: vec!["mattermost-user".to_string()],
        thread_replies: Some(false),
        mention_only: Some(true),
        group_reply: Some(group_reply()),
    });
    app.channels_settings.webhook =
        Some(WebhookConfig { port: 8787, secret: Some("secret".to_string()) });
    app.channels_settings.imessage =
        Some(IMessageConfig { allowed_contacts: vec!["user@example.com".to_string()] });
    app.channels_settings.matrix = Some(MatrixConfig {
        homeserver: "https://matrix.example".to_string(),
        access_token: "matrix-token".to_string(),
        user_id: Some("@bot:matrix.example".to_string()),
        device_id: Some("DEVICE".to_string()),
        room_id: "!room:matrix.example".to_string(),
        allowed_users: vec!["@alice:matrix.example".to_string()],
        mention_only: true,
    });
    app.channels_settings.signal = Some(SignalConfig {
        http_url: "http://127.0.0.1:8686".to_string(),
        account: "+10000000000".to_string(),
        group_id: Some("group".to_string()),
        allowed_from: vec!["+10000000001".to_string()],
        ignore_attachments: true,
        ignore_stories: true,
    });
    app.channels_settings.refresh_text_inputs();
}

#[test]
fn basic_channels_tests_are_wired() {
    assert!(module_path!().contains("basic_channels_tests"));
}

#[test]
fn basic_panels_build_disabled_and_expanded_enabled_states() {
    let mut app = test_app();
    keep_element(super::basic_channels::telegram_panel(&app));
    keep_element(super::basic_channels::discord_panel(&app));
    keep_element(super::basic_channels::slack_panel(&app));
    keep_element(super::basic_channels::mattermost_panel(&app));
    keep_element(super::basic_channels::webhook_panel(&app));
    keep_element(super::basic_channels::imessage_panel(&app));
    keep_element(super::basic_channels::matrix_panel(&app));
    keep_element(super::basic_channels::signal_panel(&app));

    configure_basic_channels(&mut app);
    keep_element(super::basic_channels::telegram_panel(&app));
    keep_element(super::basic_channels::discord_panel(&app));
    keep_element(super::basic_channels::slack_panel(&app));
    keep_element(super::basic_channels::mattermost_panel(&app));
    keep_element(super::basic_channels::webhook_panel(&app));
    keep_element(super::basic_channels::imessage_panel(&app));
    keep_element(super::basic_channels::matrix_panel(&app));
    keep_element(super::basic_channels::signal_panel(&app));
}

#[test]
fn basic_panels_build_enabled_collapsed_states_with_optional_fields_absent() {
    let mut app = test_app();
    app.channels_settings.telegram = Some(TelegramConfig {
        bot_token: "telegram-token".to_string(),
        allowed_users: vec![],
        stream_mode: Default::default(),
        draft_update_interval_ms: 100,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    });
    app.channels_settings.discord = Some(DiscordConfig {
        bot_token: "discord-token".to_string(),
        guild_id: None,
        allowed_users: vec![],
        listen_to_bots: false,
        mention_only: false,
        group_reply: None,
    });
    app.channels_settings.slack = Some(SlackConfig {
        bot_token: "slack-token".to_string(),
        app_token: None,
        channel_id: None,
        allowed_users: vec![],
        group_reply: None,
    });
    app.channels_settings.mattermost = Some(MattermostConfig {
        url: "https://mattermost.example".to_string(),
        bot_token: "mattermost-token".to_string(),
        channel_id: None,
        allowed_users: vec![],
        thread_replies: None,
        mention_only: None,
        group_reply: None,
    });
    app.channels_settings.webhook = Some(WebhookConfig { port: 1, secret: None });
    app.channels_settings.imessage = Some(IMessageConfig { allowed_contacts: vec![] });
    app.channels_settings.matrix = Some(MatrixConfig {
        homeserver: "https://matrix.example".to_string(),
        access_token: "matrix-token".to_string(),
        user_id: None,
        device_id: None,
        room_id: "!room:matrix.example".to_string(),
        allowed_users: vec![],
        mention_only: false,
    });
    app.channels_settings.signal = Some(SignalConfig {
        http_url: "http://127.0.0.1:8686".to_string(),
        account: "+10000000000".to_string(),
        group_id: None,
        allowed_from: vec![],
        ignore_attachments: false,
        ignore_stories: false,
    });
    app.channels_settings.refresh_text_inputs();

    keep_element(super::basic_channels::telegram_panel(&app));
    keep_element(super::basic_channels::discord_panel(&app));
    keep_element(super::basic_channels::slack_panel(&app));
    keep_element(super::basic_channels::mattermost_panel(&app));
    keep_element(super::basic_channels::webhook_panel(&app));
    keep_element(super::basic_channels::imessage_panel(&app));
    keep_element(super::basic_channels::matrix_panel(&app));
    keep_element(super::basic_channels::signal_panel(&app));
}

#[test]
fn matrix_panel_builds_blank_id_instance_with_optional_fields_absent() {
    let mut app = test_app();
    app.channels_settings.matrix_instances.push(MatrixInstanceConfig {
        id: "   ".to_string(),
        config: MatrixConfig {
            homeserver: "https://matrix.example".to_string(),
            access_token: "matrix-token".to_string(),
            user_id: None,
            device_id: None,
            room_id: "!room:matrix.example".to_string(),
            allowed_users: vec![],
            mention_only: false,
        },
    });
    app.channels_settings.refresh_text_inputs();

    assert!(app.channels_settings.matrix.is_none());
    assert_eq!(app.channels_settings.matrix_instances[0].id.trim(), "");
    keep_element(super::basic_channels::matrix_panel(&app));
}

#[test]
fn matrix_panel_builds_legacy_config_and_named_instances_together() {
    let mut app = test_app();
    app.channels_settings.matrix = Some(MatrixConfig {
        homeserver: "https://legacy.matrix.example".to_string(),
        access_token: "legacy-token".to_string(),
        user_id: Some("@legacy:matrix.example".to_string()),
        device_id: None,
        room_id: "!legacy:matrix.example".to_string(),
        allowed_users: vec!["@legacy-user:matrix.example".to_string()],
        mention_only: true,
    });
    app.channels_settings.matrix_instances.push(MatrixInstanceConfig {
        id: "ops".to_string(),
        config: MatrixConfig {
            homeserver: "https://ops.matrix.example".to_string(),
            access_token: "ops-token".to_string(),
            user_id: None,
            device_id: Some("OPS".to_string()),
            room_id: "!ops:matrix.example".to_string(),
            allowed_users: vec!["@ops:matrix.example".to_string()],
            mention_only: false,
        },
    });
    app.channels_settings.refresh_text_inputs();

    assert!(app.channels_settings.matrix.is_some());
    assert_eq!(app.channels_settings.matrix_instances[0].id, "ops");
    keep_element(super::basic_channels::matrix_panel(&app));
}

mod channel_instance_settings {
    use super::*;

    #[test]
    fn channel_instance_panels_build_for_matrix_feishu_dingtalk() {
        let mut app = test_app();
        app.channels_settings.matrix_instances.push(MatrixInstanceConfig {
            id: "planner".to_string(),
            config: MatrixConfig {
                homeserver: "https://matrix.profile.test".to_string(),
                access_token: "matrix-token".to_string(),
                user_id: Some("@bot:matrix.profile.test".to_string()),
                device_id: Some("DEVICE".to_string()),
                room_id: "!planner:matrix.profile.test".to_string(),
                allowed_users: vec!["@alice:matrix.profile.test".to_string()],
                mention_only: true,
            },
        });
        app.channels_settings.feishu_instances.push(FeishuInstanceConfig {
            id: "cn".to_string(),
            config: FeishuConfig {
                app_id: "feishu-app".to_string(),
                app_secret: "feishu-secret".to_string(),
                encrypt_key: Some("encrypt".to_string()),
                verification_token: Some("verify".to_string()),
                allowed_users: vec!["feishu-user".to_string()],
                group_reply: Some(group_reply()),
                receive_mode: Default::default(),
                port: Some(3001),
                draft_update_interval_ms: 1200,
                max_draft_edits: 4,
            },
        });
        app.channels_settings.dingtalk_instances.push(DingTalkInstanceConfig {
            id: "ops".to_string(),
            config: DingTalkConfig {
                client_id: "dingtalk-client".to_string(),
                client_secret: "dingtalk-secret".to_string(),
                allowed_users: vec!["dingtalk-user".to_string()],
            },
        });
        app.channels_settings.refresh_text_inputs();

        keep_element(super::super::basic_channels::matrix_panel(&app));
        keep_element(super::super::enterprise_channels::feishu_panel(&app));
        keep_element(super::super::enterprise_channels::dingtalk_panel(&app));
    }
}
