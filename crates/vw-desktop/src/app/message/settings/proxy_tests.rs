use super::*;
use crate::app::App;
use vw_config_types::proxy::ProxyScope;

fn app() -> App {
    App::new().0
}

#[test]
fn update_ignores_unrelated_messages() {
    let mut app = app();
    app.proxy_settings.enabled = true;
    app.proxy_settings.http_proxy = "http://proxy.example".to_string();
    app.proxy_settings.save_error = Some("old error".to_string());
    app.proxy_settings.show_help_modal = true;

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.proxy_settings.enabled);
    assert_eq!(app.proxy_settings.http_proxy, "http://proxy.example");
    assert_eq!(app.proxy_settings.save_error.as_deref(), Some("old error"));
    assert!(app.proxy_settings.show_help_modal);
}

#[test]
fn enabled_toggle_updates_value_and_clears_stale_error() {
    let mut app = app();
    app.proxy_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ProxyEnabledToggled(true));
    assert!(app.proxy_settings.enabled);
    assert!(app.proxy_settings.save_error.is_none());

    app.proxy_settings.save_error = Some("another error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyEnabledToggled(false));
    assert!(!app.proxy_settings.enabled);
    assert!(app.proxy_settings.save_error.is_none());
}

#[test]
fn scope_change_accepts_environment_aliases_case_and_whitespace() {
    let mut app = app();

    for input in ["environment", " ENV ", "系统环境"] {
        app.proxy_settings.save_error = Some("old error".to_string());
        let _ = update(&mut app, SettingsMessage::ProxyScopeTextChanged(input.to_string()));

        assert_eq!(app.proxy_settings.scope, ProxyScope::Environment, "input: {input}");
        assert!(app.proxy_settings.save_error.is_none(), "input: {input}");
    }
}

#[test]
fn scope_change_accepts_services_aliases_case_and_whitespace() {
    let mut app = app();

    for input in ["services", " SERVICE ", "指定服务"] {
        app.proxy_settings.save_error = Some("old error".to_string());
        let _ = update(&mut app, SettingsMessage::ProxyScopeTextChanged(input.to_string()));

        assert_eq!(app.proxy_settings.scope, ProxyScope::Services, "input: {input}");
        assert!(app.proxy_settings.save_error.is_none(), "input: {input}");
    }
}

#[test]
fn scope_change_falls_back_to_vibewindow_for_other_inputs() {
    let mut app = app();

    for input in ["vibewindow", "unknown", "  "] {
        app.proxy_settings.scope = ProxyScope::Environment;
        app.proxy_settings.save_error = Some("old error".to_string());
        let _ = update(&mut app, SettingsMessage::ProxyScopeTextChanged(input.to_string()));

        assert_eq!(app.proxy_settings.scope, ProxyScope::Vibewindow, "input: {input}");
        assert!(app.proxy_settings.save_error.is_none(), "input: {input}");
    }
}

#[test]
fn proxy_address_changes_preserve_input_and_clear_stale_error() {
    let mut app = app();

    app.proxy_settings.save_error = Some("http error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyHttpChanged(" http://proxy ".to_string()));
    assert_eq!(app.proxy_settings.http_proxy, " http://proxy ");
    assert!(app.proxy_settings.save_error.is_none());

    app.proxy_settings.save_error = Some("https error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyHttpsChanged(" https://proxy ".to_string()));
    assert_eq!(app.proxy_settings.https_proxy, " https://proxy ");
    assert!(app.proxy_settings.save_error.is_none());

    app.proxy_settings.save_error = Some("all error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyAllChanged(" socks5://proxy ".to_string()));
    assert_eq!(app.proxy_settings.all_proxy, " socks5://proxy ");
    assert!(app.proxy_settings.save_error.is_none());
}

#[test]
fn proxy_list_changes_preserve_mixed_separators_and_clear_stale_error() {
    let mut app = app();
    let no_proxy = " localhost, 127.0.0.1\n*.internal ";
    let services = " provider.openai,tool.http_request\n browser ";

    app.proxy_settings.save_error = Some("no proxy error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyNoProxyChanged(no_proxy.to_string()));
    assert_eq!(app.proxy_settings.no_proxy_input, no_proxy);
    assert!(app.proxy_settings.save_error.is_none());

    app.proxy_settings.save_error = Some("services error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyServicesChanged(services.to_string()));
    assert_eq!(app.proxy_settings.services_input, services);
    assert!(app.proxy_settings.save_error.is_none());
}

#[test]
fn help_messages_toggle_modal_without_clearing_save_error() {
    let mut app = app();
    app.proxy_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ProxyHelpOpen);
    assert!(app.proxy_settings.show_help_modal);
    assert_eq!(app.proxy_settings.save_error.as_deref(), Some("old error"));

    let _ = update(&mut app, SettingsMessage::ProxyHelpClose);
    assert!(!app.proxy_settings.show_help_modal);
    assert_eq!(app.proxy_settings.save_error.as_deref(), Some("old error"));
}

#[test]
fn proxy_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        SettingsMessage::ProxyEnabledToggled(true),
        SettingsMessage::ProxyHttpChanged(String::new()),
        SettingsMessage::ProxyNoProxyChanged("localhost,\n127.0.0.1".to_string()),
    ] {
        let task = update(&mut app, message);
        assert!(iced_runtime::task::into_stream(task.collect()).is_some());
    }
}
