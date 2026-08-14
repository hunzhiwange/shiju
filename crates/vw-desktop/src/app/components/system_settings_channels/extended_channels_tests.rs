use crate::app::{App, Message};
#[cfg(not(target_arch = "wasm32"))]
use vw_config_types::channels::EmailConfig;
use vw_config_types::channels::{
    IrcConfig, LinqConfig, NextcloudTalkConfig, WatiConfig, WhatsAppConfig,
};

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

fn render_extended_channels(app: &App) {
    keep_element(super::extended_channels::whatsapp_panel(app));
    keep_element(super::extended_channels::linq_panel(app));
    keep_element(super::extended_channels::wati_panel(app));
    keep_element(super::extended_channels::nextcloud_talk_panel(app));
    #[cfg(not(target_arch = "wasm32"))]
    keep_element(super::extended_channels::email_panel(app));
    keep_element(super::extended_channels::irc_panel(app));
}

fn configure_extended_channels(app: &mut App) {
    app.channels_settings.whatsapp = Some(WhatsAppConfig {
        access_token: Some("wa-token".to_string()),
        phone_number_id: Some("phone-id".to_string()),
        verify_token: Some("verify".to_string()),
        app_secret: Some("secret".to_string()),
        session_path: Some("/tmp/wa".to_string()),
        pair_phone: Some("+1000".to_string()),
        pair_code: Some("123456".to_string()),
        allowed_numbers: vec!["+1001".to_string()],
    });
    app.channels_settings.linq = Some(LinqConfig {
        api_token: "linq-token".to_string(),
        from_phone: "+1002".to_string(),
        signing_secret: Some("signing".to_string()),
        allowed_senders: vec!["+1003".to_string()],
    });
    app.channels_settings.wati = Some(WatiConfig {
        api_token: "wati-token".to_string(),
        api_url: "https://live-mt-server.wati.io".to_string(),
        tenant_id: Some("tenant".to_string()),
        allowed_numbers: vec!["+1004".to_string()],
    });
    app.channels_settings.nextcloud_talk = Some(NextcloudTalkConfig {
        base_url: "https://cloud.example".to_string(),
        app_token: "nextcloud-token".to_string(),
        webhook_secret: Some("secret".to_string()),
        allowed_users: vec!["alice".to_string()],
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.channels_settings.email = Some(vw_config_types::channels::EmailConfig {
            allowed_senders: vec!["alice@example.com".to_string()],
            ..vw_config_types::channels::EmailConfig::default()
        });
    }
    app.channels_settings.irc = Some(IrcConfig {
        server: "irc.example".to_string(),
        port: 6697,
        nickname: "bot".to_string(),
        username: Some("bot-user".to_string()),
        channels: vec!["#general".to_string()],
        allowed_users: vec!["alice".to_string()],
        server_password: Some("server-pass".to_string()),
        nickserv_password: Some("nick-pass".to_string()),
        sasl_password: Some("sasl-pass".to_string()),
        verify_tls: Some(false),
    });
    app.channels_settings.refresh_text_inputs();
}

#[test]
fn extended_channels_tests_are_wired() {
    assert!(module_path!().contains("extended_channels_tests"));
}

#[test]
fn extended_panels_build_disabled_and_expanded_enabled_states() {
    let mut app = test_app();
    render_extended_channels(&app);

    configure_extended_channels(&mut app);
    render_extended_channels(&app);
}

#[test]
fn extended_panels_build_enabled_states() {
    let mut app = test_app();
    configure_extended_channels(&mut app);
    render_extended_channels(&app);
}

#[test]
fn extended_panels_build_with_optional_credentials_absent() {
    let mut app = test_app();
    app.channels_settings.whatsapp = Some(WhatsAppConfig {
        access_token: None,
        phone_number_id: None,
        verify_token: None,
        app_secret: None,
        session_path: None,
        pair_phone: None,
        pair_code: None,
        allowed_numbers: Vec::new(),
    });
    app.channels_settings.linq = Some(LinqConfig {
        api_token: "test-api-token".to_string(),
        from_phone: "+1000".to_string(),
        signing_secret: None,
        allowed_senders: Vec::new(),
    });
    app.channels_settings.wati = Some(WatiConfig {
        api_token: "test-api-token".to_string(),
        api_url: "https://wati.invalid".to_string(),
        tenant_id: None,
        allowed_numbers: Vec::new(),
    });
    app.channels_settings.nextcloud_talk = Some(NextcloudTalkConfig {
        base_url: "https://nextcloud.invalid".to_string(),
        app_token: "test-app-token".to_string(),
        webhook_secret: None,
        allowed_users: Vec::new(),
    });
    app.channels_settings.irc = Some(IrcConfig {
        server: "irc.invalid".to_string(),
        port: 6697,
        nickname: "bot".to_string(),
        username: None,
        channels: Vec::new(),
        allowed_users: Vec::new(),
        server_password: None,
        nickserv_password: None,
        sasl_password: None,
        verify_tls: None,
    });
    app.channels_settings.refresh_text_inputs();

    keep_element(super::extended_channels::whatsapp_panel(&app));
    keep_element(super::extended_channels::linq_panel(&app));
    keep_element(super::extended_channels::wati_panel(&app));
    keep_element(super::extended_channels::nextcloud_talk_panel(&app));
    keep_element(super::extended_channels::irc_panel(&app));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn email_panel_builds_numeric_boundaries_and_tls_states() {
    let mut app = test_app();
    app.channels_settings.email = Some(EmailConfig {
        imap_port: 1,
        smtp_port: u16::MAX,
        smtp_tls: false,
        idle_timeout_secs: 60,
        ..EmailConfig::default()
    });
    app.channels_settings.refresh_text_inputs();
    keep_element(super::extended_channels::email_panel(&app));

    let config = app.channels_settings.email.as_mut().expect("email config");
    config.imap_port = u16::MAX;
    config.smtp_port = 1;
    config.smtp_tls = true;
    config.idle_timeout_secs = 86_400;
    app.channels_settings.refresh_text_inputs();
    keep_element(super::extended_channels::email_panel(&app));
}
