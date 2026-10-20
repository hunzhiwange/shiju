//! 验证渠道设置总览、分类标签、启用摘要与错误横幅。
//! 单渠道、多渠道和空摘要用例用于约束全局面板的计数与文案来源。

use super::*;
use crate::app::{App, Message};
use iced::advanced::widget::Tree;
use vw_config_types::channels::{TelegramConfig, WebhookConfig};

// 总览 fixture 不预启用任何渠道，便于精确验证摘要从配置推导。
fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn system_settings_channels_tests_are_wired() {
    assert!(module_path!().contains("system_settings_channels_tests"));
}

#[test]
fn global_settings_panel_builds_default_and_boundary_values() {
    let mut app = test_app();
    keep_element(global_settings_content(&app));

    app.channels_settings.cli = true;
    app.channels_settings.message_timeout_secs = 0;
    app.channels_settings.project_dir_input = "/tmp/project".to_string();
    keep_element(global_settings_content(&app));

    app.channels_settings.message_timeout_secs = 86_400;
    keep_element(global_settings_content(&app));
}

#[test]
fn view_builds_empty_enabled_channel_summary() {
    let app = test_app();

    assert_eq!(enabled_channels(&app), ["CLI"]);
    keep_element(view(&app));
}

#[test]
fn channel_tabs_builds_domestic_two_group_grid() {
    let app = test_app();

    assert_eq!(CHANNEL_TAB_COLUMNS, 5);
    assert_eq!(CHINA_CHANNEL_CARD_COLUMNS, 3);
    assert_eq!(CHANNEL_TAB_GROUPS.map(|(_, title)| title), ["全局", "中文互联网"]);
    keep_element(channel_tabs(app.channels_settings.active_tab));
}

#[test]
fn china_channel_cards_build_brand_icons_and_active_states() {
    let descriptors = channel_tab_descriptors();
    let china = descriptors
        .iter()
        .filter(|descriptor| channel_tab_group(descriptor.tab) == ChannelTabGroup::China)
        .collect::<Vec<_>>();

    assert_eq!(china.len(), 5);
    for descriptor in &china {
        std::hint::black_box(china_channel_icon(descriptor.tab));
        keep_element(china_channel_card(descriptor, descriptor.tab));
    }
    keep_element(china_channel_cards(&china, ChannelsSettingsTab::Global));
}

#[test]
fn channel_tabs_group_global_china_and_international_channels() {
    for tab in [
        ChannelsSettingsTab::Global,
        ChannelsSettingsTab::Webhook,
        ChannelsSettingsTab::Email,
    ] {
        assert!(matches!(channel_tab_group(tab), ChannelTabGroup::Global));
    }
    for tab in [
        ChannelsSettingsTab::Feishu,
        ChannelsSettingsTab::DingTalk,
        ChannelsSettingsTab::WeCom,
        ChannelsSettingsTab::Qq,
        ChannelsSettingsTab::WeChat,
    ] {
        assert!(matches!(channel_tab_group(tab), ChannelTabGroup::China));
    }
    for tab in [ChannelsSettingsTab::Telegram, ChannelsSettingsTab::Lark] {
        assert!(matches!(channel_tab_group(tab), ChannelTabGroup::International));
    }

    let descriptors = channel_tab_descriptors();
    assert!(
        descriptors
            .iter()
            .all(|descriptor| channel_tab_group(descriptor.tab) != ChannelTabGroup::International)
    );
    assert_eq!(
        descriptors
            .iter()
            .find(|descriptor| descriptor.tab == ChannelsSettingsTab::Feishu)
            .map(|descriptor| descriptor.label),
        Some("飞书")
    );
    assert_eq!(
        descriptors
            .iter()
            .find(|descriptor| descriptor.tab == ChannelsSettingsTab::DingTalk)
            .map(|descriptor| descriptor.label),
        Some("钉钉")
    );
}

#[test]
fn channels_error_banner_follows_channel_tabs() {
    let mut app = test_app();

    for descriptor in channel_tab_descriptors() {
        app.channels_settings.active_tab = descriptor.tab;
        let clean_children = {
            let clean = view(&app);
            let clean_tree = Tree::new(clean.as_widget());
            clean_tree.children[0].children.len()
        };
        assert_eq!(clean_children, 6);

        app.channels_settings.save_error = Some("channel save failed".to_string());
        let error_children = {
            let error = view(&app);
            let error_tree = Tree::new(error.as_widget());
            error_tree.children[0].children.len()
        };
        assert_eq!(error_children, 7);
        app.channels_settings.save_error = None;
    }
}

#[test]
fn view_builds_cli_enabled_channel_summary() {
    let mut app = test_app();
    app.channels_settings.cli = true;

    assert_eq!(enabled_channels(&app), ["CLI"]);
    keep_element(view(&app));
}

#[test]
fn view_builds_multi_channel_enabled_summary() {
    let mut app = test_app();
    app.channels_settings.cli = true;
    app.channels_settings.telegram = Some(TelegramConfig {
        bot_token: "token".to_string(),
        allowed_users: Vec::new(),
        stream_mode: Default::default(),
        draft_update_interval_ms: 500,
        interrupt_on_new_message: false,
        mention_only: false,
        group_reply: None,
        base_url: None,
    });
    app.channels_settings.webhook = Some(WebhookConfig { port: 8787, secret: None });
    app.channels_settings
        .text_editors
        .insert("telegram.allowed_users".to_string(), iced::widget::text_editor::Content::new());
    app.channels_settings.text_editors.insert(
        "telegram.group_reply.allowed_sender_ids".to_string(),
        iced::widget::text_editor::Content::new(),
    );

    let enabled = enabled_channels(&app);

    assert!(enabled.contains(&"CLI"));
    assert!(enabled.contains(&"Telegram"));
    assert!(enabled.contains(&"Webhook"));
    keep_element(view(&app));
}
