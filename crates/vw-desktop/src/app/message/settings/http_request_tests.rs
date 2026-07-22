use super::*;
use crate::app::App;
use vw_config_types::tools::HttpRequestConfig;

fn app() -> App {
    App::new().0
}

fn has_scheduled_action(task: Task<Message>) -> bool {
    task.units() > 0
}

#[test]
fn http_request_normalizes_domains_and_handles_duplicates() {
    assert_eq!(normalize_domain(" Example.COM "), "example.com");

    let mut app = app();
    let _ = update(
        &mut app,
        SettingsMessage::HttpRequest(HttpRequestMessage::NewAllowedDomainChanged(
            "Example.com,\nAPI.EXAMPLE.COM".to_string(),
        )),
    );
    let _ = update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::AddAllowedDomain));
    assert_eq!(
        app.http_request_settings.allowed_domains,
        vec!["example.com".to_string(), "api.example.com".to_string()]
    );
    assert!(app.http_request_settings.new_allowed_domain_input.is_empty());

    let _ = update(
        &mut app,
        SettingsMessage::HttpRequest(HttpRequestMessage::NewAllowedDomainChanged(
            "example.com".to_string(),
        )),
    );
    let _ = update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::AddAllowedDomain));
    assert!(app.http_request_settings.save_error.as_deref().unwrap_or("").contains("已存在"));
}

#[test]
fn http_request_update_resets_user_agent_and_removes_domains() {
    let mut app = app();
    app.http_request_settings.allowed_domains = vec!["example.com".to_string()];
    app.http_request_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::Refresh));
    assert!(app.http_request_settings.save_error.is_none());

    let _ = update(
        &mut app,
        SettingsMessage::HttpRequest(HttpRequestMessage::UserAgentChanged("   ".to_string())),
    );
    assert_eq!(app.http_request_settings.user_agent, HttpRequestConfig::default().user_agent);

    let _ =
        update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::RemoveAllowedDomain(0)));
    assert!(app.http_request_settings.allowed_domains.is_empty());
}

#[test]
fn update_ignores_unrelated_messages_without_changing_http_request_state() {
    let mut app = app();
    app.http_request_settings.enabled = true;
    app.http_request_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);

    assert!(app.http_request_settings.enabled);
    assert_eq!(app.http_request_settings.save_error.as_deref(), Some("old error"));
}

#[test]
fn refresh_only_clears_the_save_error() {
    let mut app = app();
    app.http_request_settings.user_agent = "  ".to_string();
    app.http_request_settings.new_allowed_domain_input = "example.com".to_string();
    app.http_request_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::Refresh));

    assert_eq!(app.http_request_settings.user_agent, "  ");
    assert_eq!(app.http_request_settings.new_allowed_domain_input, "example.com");
    assert!(app.http_request_settings.save_error.is_none());
}

#[test]
fn add_allowed_domain_rejects_empty_input_without_changing_the_list() {
    let mut app = app();
    app.http_request_settings.allowed_domains = vec!["existing.example".to_string()];
    app.http_request_settings.new_allowed_domain_input = "  ,\n \r\n,".to_string();

    let _ = update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::AddAllowedDomain));

    assert_eq!(app.http_request_settings.allowed_domains, vec!["existing.example".to_string()]);
    assert_eq!(app.http_request_settings.new_allowed_domain_input, "  ,\n \r\n,");
    assert_eq!(app.http_request_settings.save_error.as_deref(), Some("允许域名不能为空"));
}

#[test]
fn add_allowed_domain_keeps_existing_entries_and_inserts_only_new_candidates() {
    let mut app = app();
    app.http_request_settings.allowed_domains = vec!["existing.example".to_string()];
    app.http_request_settings.new_allowed_domain_input =
        " EXISTING.EXAMPLE, New.Example\nnew.example ".to_string();
    app.http_request_settings.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::AddAllowedDomain));

    assert_eq!(
        app.http_request_settings.allowed_domains,
        vec!["existing.example".to_string(), "new.example".to_string()]
    );
    assert!(app.http_request_settings.new_allowed_domain_input.is_empty());
    assert!(app.http_request_settings.save_error.is_none());
}

#[test]
fn remove_allowed_domain_ignores_an_out_of_bounds_index() {
    let mut app = app();
    app.http_request_settings.allowed_domains =
        vec!["first.example".to_string(), "second.example".to_string()];
    app.http_request_settings.save_error = Some("old error".to_string());

    let _ =
        update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::RemoveAllowedDomain(2)));

    assert_eq!(
        app.http_request_settings.allowed_domains,
        vec!["first.example".to_string(), "second.example".to_string()]
    );
    assert!(app.http_request_settings.save_error.is_none());
}

#[test]
fn update_applies_enabled_numeric_and_non_blank_user_agent_edits() {
    let mut app = app();

    let _ =
        update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::EnabledToggled(true)));
    let _ = update(
        &mut app,
        SettingsMessage::HttpRequest(HttpRequestMessage::MaxResponseSizeChanged(u32::MAX)),
    );
    let _ =
        update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::TimeoutSecsChanged(0)));
    let _ = update(
        &mut app,
        SettingsMessage::HttpRequest(HttpRequestMessage::UserAgentChanged(
            " Custom Agent ".to_string(),
        )),
    );

    assert!(app.http_request_settings.enabled);
    assert_eq!(app.http_request_settings.max_response_size, u32::MAX);
    assert_eq!(app.http_request_settings.timeout_secs, 0);
    assert_eq!(app.http_request_settings.user_agent, " Custom Agent ");
    assert!(app.http_request_settings.save_error.is_none());
}

#[test]
fn domain_validation_rejections_skip_persistence_but_new_domain_schedules_it() {
    let mut app = app();

    app.http_request_settings.new_allowed_domain_input = "  ,\n".to_string();
    let empty_task =
        update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::AddAllowedDomain));
    assert!(!has_scheduled_action(empty_task));

    app.http_request_settings.allowed_domains = vec!["existing.example".to_string()];
    app.http_request_settings.new_allowed_domain_input = "EXISTING.EXAMPLE".to_string();
    let duplicate_task =
        update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::AddAllowedDomain));
    assert!(!has_scheduled_action(duplicate_task));

    app.http_request_settings.new_allowed_domain_input = "new.example".to_string();
    let inserted_task =
        update(&mut app, SettingsMessage::HttpRequest(HttpRequestMessage::AddAllowedDomain));
    assert!(has_scheduled_action(inserted_task));
    assert_eq!(
        app.http_request_settings.allowed_domains,
        vec!["existing.example".to_string(), "new.example".to_string()]
    );
}
