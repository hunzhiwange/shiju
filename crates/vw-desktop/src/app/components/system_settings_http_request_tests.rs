use super::*;
use crate::app::message::settings::{HttpRequestMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn number_display_formats_plain_and_suffixed_values() {
    assert_eq!(number_display(0, ""), "0");
    assert_eq!(number_display(30, "秒"), "30 秒");
    assert_eq!(number_display(u32::MAX, "字节"), "4294967295 字节");
}

#[test]
fn normalize_number_value_rounds_then_clamps_to_bounds() {
    assert_eq!(normalize_number_value(-1.0, 0, 100), 0);
    assert_eq!(normalize_number_value(41.49, 0, 100), 41);
    assert_eq!(normalize_number_value(41.5, 0, 100), 42);
    assert_eq!(normalize_number_value(100.9, 0, 100), 100);
    assert_eq!(normalize_number_value(7.4, 7, 7), 7);
}

#[test]
fn row_helpers_build_each_http_request_control() {
    keep_element(field_row("标签", "说明", text("control")));
    keep_element(text_row("User-Agent", "说明", "placeholder", "agent", |value| {
        Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::UserAgentChanged(value)))
    }));
    keep_element(bool_row("启用", "说明", true, "启用网络请求工具", |value| {
        Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::EnabledToggled(value)))
    }));
    keep_element(number_row("无限制", "说明", 0, 0, u32::MAX, "", None, |value| {
        Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::MaxResponseSizeChanged(
            value,
        )))
    }));
    keep_element(number_row("超时", "说明", 30, 0, 86_400, "秒", Some("安全提示"), |value| {
        Message::Settings(SettingsMessage::HttpRequest(HttpRequestMessage::TimeoutSecsChanged(
            value,
        )))
    }));
}

#[test]
fn view_builds_empty_allowlist_and_zero_limit_state() {
    let mut app = test_app();
    app.http_request_settings.allowed_domains.clear();
    app.http_request_settings.max_response_size = 0;
    app.http_request_settings.timeout_secs = 0;

    keep_element(view(&app));
}

#[test]
fn view_builds_enabled_populated_allowlist_and_upper_bounds() {
    let mut app = test_app();
    app.http_request_settings.enabled = true;
    app.http_request_settings.allowed_domains =
        vec!["example.com".to_string(), "*.example.org".to_string(), "*".to_string()];
    app.http_request_settings.new_allowed_domain_input = "api.example.com".to_string();
    app.http_request_settings.max_response_size = u32::MAX;
    app.http_request_settings.timeout_secs = 86_400;
    app.http_request_settings.user_agent = "VibeWindow-Test/1.0".to_string();

    keep_element(view(&app));
}

#[test]
fn view_renders_one_allowlist_row_for_each_configured_domain() {
    let mut app = test_app();
    app.http_request_settings.allowed_domains =
        vec!["api.example.com".to_string(), "*.internal.example".to_string()];

    let tree = Tree::new(view(&app).as_widget());
    assert_eq!(tree.children.len(), 6);
    assert_eq!(tree.children[5].children.len(), 2);
}
