use super::*;
use crate::app::{App, Message};
use iced::advanced::widget::Tree;
use vw_config_types::channels::{TelegramConfig, WebhookConfig};

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
fn channel_tabs_builds_five_column_grid() {
    let app = test_app();

    assert_eq!(CHANNEL_TAB_COLUMNS, 5);
    keep_element(channel_tabs(app.channels_settings.active_tab));
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
