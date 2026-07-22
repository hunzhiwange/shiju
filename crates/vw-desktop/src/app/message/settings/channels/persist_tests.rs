use super::*;
use crate::app::App;
use crate::app::message::settings::channels::toggles;
use vw_config_types::channels::{
    DingTalkConfig, DingTalkInstanceConfig, FeishuConfig, FeishuInstanceConfig, MatrixConfig,
    MatrixInstanceConfig,
};

fn app() -> App {
    App::new().0
}

#[test]
fn enabled_channels_reports_every_human_readable_name_in_display_order() {
    let mut app = app();
    for channel in [
        "telegram",
        "discord",
        "slack",
        "mattermost",
        "webhook",
        "imessage",
        "matrix",
        "signal",
        "whatsapp",
        "linq",
        "wati",
        "nextcloud_talk",
        #[cfg(not(target_arch = "wasm32"))]
        "email",
        "irc",
        "lark",
        "feishu",
        "dingtalk",
        "qq",
        "nostr",
        "clawdtalk",
    ] {
        toggles::toggle_enabled(&mut app, channel, true);
    }

    let names = enabled_channels(&app.channels_settings);
    assert_eq!(
        names,
        [
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
            #[cfg(not(target_arch = "wasm32"))]
            "Email",
            "IRC",
            "Lark",
            "Feishu",
            "DingTalk",
            "QQ",
            "Nostr",
            "ClawdTalk",
        ]
    );
}

#[test]
fn build_channels_config_copies_enabled_channels_and_normalizes_boundaries() {
    let mut app = app();
    for channel in [
        "telegram",
        "discord",
        "slack",
        "mattermost",
        "webhook",
        "imessage",
        "matrix",
        "signal",
        "whatsapp",
        "linq",
        "wati",
        "nextcloud_talk",
        #[cfg(not(target_arch = "wasm32"))]
        "email",
        "irc",
        "lark",
        "feishu",
        "dingtalk",
        "qq",
        "nostr",
        "clawdtalk",
    ] {
        toggles::toggle_enabled(&mut app, channel, true);
    }
    app.channels_settings.project_dir_input = "  /tmp/vibe-window-project  ".to_string();
    app.channels_settings.message_timeout_secs = 0;

    let saved = build_channels_config_from_settings(&app.channels_settings).unwrap();

    assert_eq!(
        saved.project_dir.as_deref(),
        Some(std::path::Path::new("/tmp/vibe-window-project"))
    );
    assert_eq!(saved.message_timeout_secs, 1);
    assert!(saved.cli);
    assert!(saved.telegram.is_some());
    assert!(saved.discord.is_some());
    assert!(saved.slack.is_some());
    assert!(saved.mattermost.is_some());
    assert!(saved.webhook.is_some());
    assert!(saved.imessage.is_some());
    assert!(saved.matrix.is_some());
    assert!(saved.signal.is_some());
    assert!(saved.whatsapp.is_some());
    assert!(saved.linq.is_some());
    assert!(saved.wati.is_some());
    assert!(saved.nextcloud_talk.is_some());
    #[cfg(not(target_arch = "wasm32"))]
    assert!(saved.email.is_some());
    assert!(saved.irc.is_some());
    assert!(saved.lark.is_some());
    assert!(saved.feishu.is_some());
    assert!(saved.dingtalk.is_some());
    assert!(saved.qq.is_some());
    assert!(saved.nostr.is_some());
    assert!(saved.clawdtalk.is_some());
}

#[test]
fn persist_channels_requires_at_least_one_enabled_channel() {
    let mut app = app();
    app.channels_settings.cli = false;
    app.channels_settings.telegram = None;
    app.channels_settings.discord = None;
    app.channels_settings.slack = None;
    app.channels_settings.mattermost = None;
    app.channels_settings.webhook = None;
    app.channels_settings.imessage = None;
    app.channels_settings.matrix = None;
    app.channels_settings.signal = None;
    app.channels_settings.whatsapp = None;
    app.channels_settings.linq = None;
    app.channels_settings.wati = None;
    app.channels_settings.nextcloud_talk = None;
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.channels_settings.email = None;
    }
    app.channels_settings.irc = None;
    app.channels_settings.lark = None;
    app.channels_settings.feishu = None;
    app.channels_settings.dingtalk = None;
    app.channels_settings.qq = None;
    app.channels_settings.nostr = None;
    app.channels_settings.clawdtalk = None;

    assert!(persist_channels_settings(&mut app).is_none());
    assert!(app.channels_settings.save_error.as_deref().unwrap_or("").contains("至少保留一个"));

    app.channels_settings.cli = true;
    assert!(persist_channels_settings(&mut app).is_some());
    assert!(app.channels_settings.save_error.is_none());
}

mod channel_instance_settings {
    use super::*;

    #[test]
    fn persist_channels_settings_preserves_named_instances() {
        let mut app = app();
        app.channels_settings.cli = false;
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
                group_reply: None,
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

        assert_eq!(enabled_channels(&app.channels_settings), ["Matrix", "Feishu", "DingTalk"]);

        let saved = build_channels_config_from_settings(&app.channels_settings).unwrap();

        assert_eq!(saved.matrix_instances[0].id, "planner");
        assert_eq!(saved.matrix_instances[0].config.access_token, "matrix-token");
        assert_eq!(saved.feishu_instances[0].id, "cn");
        assert_eq!(saved.feishu_instances[0].config.app_secret, "feishu-secret");
        assert_eq!(saved.dingtalk_instances[0].id, "ops");
        assert_eq!(saved.dingtalk_instances[0].config.client_secret, "dingtalk-secret");
        assert!(persist_channels_settings(&mut app).is_some());
        assert!(app.channels_settings.save_error.is_none());
    }

    #[test]
    fn invalid_named_instance_id_blocks_build_and_persist_with_clear_error() {
        let mut app = app();
        app.channels_settings.matrix_instances.push(MatrixInstanceConfig {
            id: "unsafe:id".to_string(),
            config: MatrixConfig {
                homeserver: String::new(),
                access_token: String::new(),
                user_id: None,
                device_id: None,
                room_id: String::new(),
                allowed_users: Vec::new(),
                mention_only: false,
            },
        });

        let error = build_channels_config_from_settings(&app.channels_settings).unwrap_err();
        assert!(error.contains("must not contain"));

        app.channels_settings.save_error = Some("stale error".to_string());
        assert!(persist_channels_settings(&mut app).is_none());
        assert!(
            app.channels_settings.save_error.as_deref().unwrap_or("").contains("must not contain")
        );
    }
}
