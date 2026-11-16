//! Local channel overview behavior; host page composition remains in vw-desktop.
use crate::test_support::Fixture;
use crate::{messages::Message, overview::*, state::ChannelsSettingsTab};
fn test_app() -> Fixture {
    Fixture::new().0
}
fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}
mod system_settings_channels_tests {
    use super::*;
    #[test]
    fn system_settings_channels_tests_are_wired() {
        assert!(module_path!().contains("system_settings_channels_tests"));
    }

    #[test]
    fn global_settings_panel_builds_default_and_boundary_values() {
        let mut app = test_app();
        keep_element(global_settings_content(app.view()));

        app.channels_settings.cli = true;
        app.channels_settings.message_timeout_secs = 0;
        app.channels_settings.project_dir_input = "/tmp/project".to_string();
        keep_element(global_settings_content(app.view()));

        app.channels_settings.message_timeout_secs = 86_400;
        keep_element(global_settings_content(app.view()));
    }

    #[test]
    fn channel_tabs_builds_domestic_two_group_grid() {
        let app = test_app();

        assert_eq!(CHANNEL_TAB_COLUMNS, 5);
        assert_eq!(CHINA_CHANNEL_CARD_COLUMNS, 3);
        assert_eq!(CHANNEL_TAB_GROUPS.map(|(_, title)| title), ["全局", "中文互联网"]);
        keep_element(channel_tabs(
            config_ui::DesktopLocale::SimplifiedChinese,
            app.channels_settings.active_tab,
        ));
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
            keep_element(china_channel_card(
                config_ui::DesktopLocale::SimplifiedChinese,
                descriptor,
                descriptor.tab,
            ));
        }
        keep_element(china_channel_cards(
            config_ui::DesktopLocale::SimplifiedChinese,
            &china,
            ChannelsSettingsTab::Global,
        ));
    }

    #[test]
    fn channel_tabs_group_global_china_and_international_channels() {
        for tab in
            [ChannelsSettingsTab::Global, ChannelsSettingsTab::Webhook, ChannelsSettingsTab::Email]
        {
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
                .all(|descriptor| channel_tab_group(descriptor.tab)
                    != ChannelTabGroup::International)
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
}

mod card;
mod delivery_modal;
