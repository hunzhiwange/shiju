// 验证频道启停、动态实例编号分配与默认配置创建。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::App;
use vw_config_types::channels::{LarkReceiveMode, QQReceiveMode};

fn app() -> App {
    App::new().0
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn toggle_enabled_creates_static_channel_defaults() {
    let mut app = app();
    let channels = [
        "telegram",
        "discord",
        "slack",
        "mattermost",
        "webhook",
        "imessage",
        "signal",
        "whatsapp",
        "linq",
        "wati",
        "nextcloud_talk",
        #[cfg(not(target_arch = "wasm32"))]
        "email",
        "irc",
        "lark",
        "qq",
        "nostr",
        "clawdtalk",
    ];

    for channel in channels {
        toggle_enabled(&mut app, channel, true);
    }

    let telegram = app.channels_settings.telegram.as_ref().expect("telegram");
    assert!(telegram.bot_token.is_empty());
    assert_eq!(telegram.draft_update_interval_ms, 1000);
    assert!(!telegram.interrupt_on_new_message);
    assert!(app.channels_settings.discord.as_ref().expect("discord").guild_id.is_none());
    assert!(app.channels_settings.slack.as_ref().expect("slack").app_token.is_none());
    let mattermost = app.channels_settings.mattermost.as_ref().expect("mattermost");
    assert_eq!(mattermost.thread_replies, Some(true));
    assert_eq!(mattermost.mention_only, Some(false));
    let webhook = app.channels_settings.webhook.as_ref().expect("webhook");
    assert_eq!(webhook.port, 8787);
    assert!(webhook.secret.is_none());
    assert!(app.channels_settings.imessage.as_ref().expect("imessage").allowed_contacts.is_empty());
    let signal = app.channels_settings.signal.as_ref().expect("signal");
    assert!(signal.http_url.is_empty());
    assert!(!signal.ignore_attachments);
    assert!(!signal.ignore_stories);
    let whatsapp = app.channels_settings.whatsapp.as_ref().expect("whatsapp");
    assert!(whatsapp.access_token.is_none());
    assert!(whatsapp.session_path.is_none());
    assert!(app.channels_settings.linq.as_ref().expect("linq").api_token.is_empty());
    let wati = app.channels_settings.wati.as_ref().expect("wati");
    assert_eq!(wati.api_url, "https://live-mt-server.wati.io");
    assert!(wati.tenant_id.is_none());
    assert!(
        app.channels_settings.nextcloud_talk.as_ref().expect("nextcloud talk").base_url.is_empty()
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let email = app.channels_settings.email.as_ref().expect("email");
        assert_eq!(email.imap_port, 993);
        assert_eq!(email.smtp_port, 465);
        assert!(email.smtp_tls);
    }
    let irc = app.channels_settings.irc.as_ref().expect("irc");
    assert_eq!(irc.port, 6697);
    assert_eq!(irc.verify_tls, Some(true));
    let lark = app.channels_settings.lark.as_ref().expect("lark");
    assert_eq!(lark.receive_mode, LarkReceiveMode::Websocket);
    assert_eq!(lark.draft_update_interval_ms, 3000);
    assert_eq!(lark.max_draft_edits, 20);
    assert_eq!(app.channels_settings.qq.as_ref().expect("qq").receive_mode, QQReceiveMode::Webhook);
    assert_eq!(
        app.channels_settings
            .nostr
            .as_ref()
            .expect("nostr")
            .relays
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "wss://relay.damus.io",
            "wss://nos.lol",
            "wss://relay.primal.net",
            "wss://relay.snort.social",
        ]
    );
    assert!(app.channels_settings.clawdtalk.as_ref().expect("clawdtalk").webhook_secret.is_none());
}

#[test]
fn toggle_enabled_disables_every_static_channel() {
    let mut app = app();
    let channels = [
        "telegram",
        "discord",
        "slack",
        "mattermost",
        "webhook",
        "imessage",
        "signal",
        "whatsapp",
        "linq",
        "wati",
        "nextcloud_talk",
        #[cfg(not(target_arch = "wasm32"))]
        "email",
        "irc",
        "lark",
        "qq",
        "nostr",
        "clawdtalk",
    ];

    for channel in channels {
        toggle_enabled(&mut app, channel, true);
        toggle_enabled(&mut app, channel, false);
    }

    assert!(app.channels_settings.telegram.is_none());
    assert!(app.channels_settings.discord.is_none());
    assert!(app.channels_settings.slack.is_none());
    assert!(app.channels_settings.mattermost.is_none());
    assert!(app.channels_settings.webhook.is_none());
    assert!(app.channels_settings.imessage.is_none());
    assert!(app.channels_settings.signal.is_none());
    assert!(app.channels_settings.whatsapp.is_none());
    assert!(app.channels_settings.linq.is_none());
    assert!(app.channels_settings.wati.is_none());
    assert!(app.channels_settings.nextcloud_talk.is_none());
    #[cfg(not(target_arch = "wasm32"))]
    assert!(app.channels_settings.email.is_none());
    assert!(app.channels_settings.irc.is_none());
    assert!(app.channels_settings.lark.is_none());
    assert!(app.channels_settings.qq.is_none());
    assert!(app.channels_settings.nostr.is_none());
    assert!(app.channels_settings.clawdtalk.is_none());
}

#[test]
fn toggle_enabled_ignores_unknown_channel() {
    let mut app = app();

    toggle_enabled(&mut app, "unknown", false);
    toggle_enabled(&mut app, "unknown", true);
    assert_eq!(app.channels_settings.active_tab, crate::app::state::ChannelsSettingsTab::Global);
}

#[test]
fn next_instance_id_returns_first_unused_numeric_id() {
    assert_eq!(next_instance_id(false, std::iter::empty::<&str>()), "1");
    assert_eq!(next_instance_id(true, ["1", "3"].into_iter()), "2");
    assert_eq!(next_instance_id(false, ["custom", "1", "2"].into_iter()), "3");
}
