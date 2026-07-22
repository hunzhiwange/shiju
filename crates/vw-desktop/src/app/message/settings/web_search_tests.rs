use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

#[test]
fn normalize_provider_accepts_supported_names_and_falls_back() {
    assert_eq!(normalize_provider(" DDG "), "duckduckgo");
    assert_eq!(normalize_provider("DuckDuckGo"), "duckduckgo");
    assert_eq!(normalize_provider("BRAVE"), "brave");
    assert_eq!(normalize_provider("serper"), "serper");
    assert_eq!(normalize_provider("Google"), "google");
    assert_eq!(normalize_provider("BING"), "bing");
    assert_eq!(normalize_provider("unsupported"), "duckduckgo");
    assert_eq!(normalize_provider("   "), "duckduckgo");
}

#[test]
fn trim_to_option_trims_values_and_rejects_blank() {
    assert_eq!(trim_to_option("  secret  "), Some("secret".to_string()));
    assert_eq!(
        trim_to_option("\nhttps://search.example\t"),
        Some("https://search.example".to_string())
    );
    assert_eq!(trim_to_option(" \n\t "), None);
}

#[test]
fn parse_max_results_applies_default_bounds_and_errors() {
    assert_eq!(parse_max_results("  ").unwrap(), WebSearchConfig::default().max_results);
    assert_eq!(parse_max_results(" 1 ").unwrap(), 1);
    assert_eq!(parse_max_results("7").unwrap(), 7);
    assert_eq!(parse_max_results("11").unwrap(), 10);
    assert_eq!(parse_max_results(&usize::MAX.to_string()).unwrap(), 10);
    assert_eq!(parse_max_results("0").unwrap_err(), "max_results 必须大于 0");
    assert_eq!(parse_max_results("many").unwrap_err(), "max_results 必须是整数");
}

#[test]
fn parse_timeout_secs_applies_default_and_rejects_invalid_values() {
    assert_eq!(parse_timeout_secs("\t").unwrap(), WebSearchConfig::default().timeout_secs);
    assert_eq!(parse_timeout_secs(" 1 ").unwrap(), 1);
    assert_eq!(parse_timeout_secs("3600").unwrap(), 3600);
    assert_eq!(parse_timeout_secs("0").unwrap_err(), "timeout_secs 必须大于 0");
    assert_eq!(parse_timeout_secs("soon").unwrap_err(), "timeout_secs 必须是整数秒");
}

#[test]
fn persistence_validation_reports_max_results_before_timeout() {
    let mut app = app();
    app.web_search_settings.max_results_input = "invalid max".to_string();
    app.web_search_settings.timeout_secs_input = "invalid timeout".to_string();

    let error = match persist_web_search_settings(&mut app) {
        Ok(_) => panic!("invalid max_results should reject persistence"),
        Err(error) => error,
    };

    assert_eq!(error, "max_results 必须是整数");
}

#[test]
fn update_ignores_unrelated_messages_and_handles_refresh_and_help() {
    let mut app = app();
    app.web_search_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);
    assert_eq!(app.web_search_settings.save_error.as_deref(), Some("old error"));

    let _ = update(&mut app, SettingsMessage::WebSearch(WebSearchMessage::HelpOpen));
    assert!(app.web_search_settings.show_help_modal);
    assert_eq!(app.web_search_settings.save_error.as_deref(), Some("old error"));

    let _ = update(&mut app, SettingsMessage::WebSearch(WebSearchMessage::HelpClose));
    assert!(!app.web_search_settings.show_help_modal);
    assert_eq!(app.web_search_settings.save_error.as_deref(), Some("old error"));

    let _ = update(&mut app, SettingsMessage::WebSearch(WebSearchMessage::Refresh));
    assert!(app.web_search_settings.save_error.is_none());
}

#[test]
fn update_applies_each_edit_and_clears_stale_error() {
    let mut app = app();
    app.web_search_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::WebSearch(WebSearchMessage::EnabledToggled(true)));
    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::ProviderChanged(" BRAVE ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::ApiKeyChanged(" api key ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::ApiUrlChanged(
            " https://search.example ".to_string(),
        )),
    );
    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::BraveApiKeyChanged(" brave key ".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::MaxResultsChanged("10".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::TimeoutSecsChanged("45".to_string())),
    );
    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::UserAgentChanged(" Test Agent ".to_string())),
    );

    assert!(app.web_search_settings.enabled);
    assert_eq!(app.web_search_settings.provider, "brave");
    assert_eq!(app.web_search_settings.api_key_input, " api key ");
    assert_eq!(app.web_search_settings.api_url_input, " https://search.example ");
    assert_eq!(app.web_search_settings.brave_api_key_input, " brave key ");
    assert_eq!(app.web_search_settings.max_results_input, "10");
    assert_eq!(app.web_search_settings.timeout_secs_input, "45");
    assert_eq!(app.web_search_settings.user_agent, " Test Agent ");
    assert!(app.web_search_settings.save_error.is_none());
}

#[test]
fn update_reports_max_results_validation_errors_and_recovers() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::MaxResultsChanged("0".to_string())),
    );
    assert_eq!(app.web_search_settings.save_error.as_deref(), Some("max_results 必须大于 0"));

    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::MaxResultsChanged("invalid".to_string())),
    );
    assert_eq!(app.web_search_settings.save_error.as_deref(), Some("max_results 必须是整数"));

    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::MaxResultsChanged("5".to_string())),
    );
    assert!(app.web_search_settings.save_error.is_none());
}

#[test]
fn update_reports_timeout_validation_errors_and_recovers() {
    let mut app = app();

    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::TimeoutSecsChanged("0".to_string())),
    );
    assert_eq!(app.web_search_settings.save_error.as_deref(), Some("timeout_secs 必须大于 0"));

    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::TimeoutSecsChanged("invalid".to_string())),
    );
    assert_eq!(app.web_search_settings.save_error.as_deref(), Some("timeout_secs 必须是整数秒"));

    let _ = update(
        &mut app,
        SettingsMessage::WebSearch(WebSearchMessage::TimeoutSecsChanged("15".to_string())),
    );
    assert!(app.web_search_settings.save_error.is_none());
}
