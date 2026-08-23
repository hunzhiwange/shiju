//! 验证浏览器后端选择及原生、计算机使用配置区的条件展示。
//! 未知后端走稳定回退，原生专属设置不得出现在其他后端页面。

use super::*;
use crate::app::message::settings::{BrowserMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

// 浏览器后端由用例逐项选择，默认应用仅提供完整视图上下文。
fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn labeled_option_displays_user_facing_label_not_config_value() {
    let option = LabeledOption { value: "new_window", label: "新窗口" };

    assert_eq!(option.to_string(), "新窗口");
    assert_ne!(option.to_string(), option.value);
}

#[test]
fn browser_open_options_keep_stable_config_values_and_labels() {
    let options = browser_open_options();
    let values: Vec<_> = options.iter().map(|option| option.value).collect();
    let labels: Vec<_> = options.iter().map(|option| option.label).collect();

    assert_eq!(values, ["default", "new_window", "new_tab"]);
    assert_eq!(labels, ["默认", "新窗口", "新标签页"]);
}

#[test]
fn backend_options_keep_stable_config_values_and_labels() {
    let options = backend_options();
    let values: Vec<_> = options.iter().map(|option| option.value).collect();
    let labels: Vec<_> = options.iter().map(|option| option.label).collect();

    assert_eq!(values, ["agent_browser", "native", "computer_use", "auto"]);
    assert_eq!(
        labels,
        ["代理浏览器（默认）", "原生浏览器（WebDriver）", "系统控制（Sidecar）", "自动"]
    );
}

#[test]
fn row_helpers_build_aligned_top_text_and_boolean_controls() {
    keep_element(field_row("标签", "说明", text("control")));
    keep_element(field_row_top("多行标签", "说明", text("multiline control")));
    keep_element(text_row("会话", "说明", "placeholder", "session", |value| {
        Message::Settings(SettingsMessage::Browser(BrowserMessage::SessionNameChanged(value)))
    }));
    keep_element(bool_row("启用", "说明", true, "启用浏览器", |value| {
        Message::Settings(SettingsMessage::Browser(BrowserMessage::EnabledToggled(value)))
    }));
}

#[test]
fn view_builds_empty_and_populated_browser_settings() {
    let mut app = test_app();
    keep_element(view(&app));

    app.browser_settings.enabled = true;
    app.browser_settings.allowed_domains_editor =
        iced::widget::text_editor::Content::with_text("example.com\n*.example.org");
    app.browser_settings.browser_open = "new_window".to_string();
    app.browser_settings.session_name_input = "isolated".to_string();
    app.browser_settings.backend = "native".to_string();
    app.browser_settings.native_headless = false;
    app.browser_settings.native_webdriver_url = "http://127.0.0.1:4444".to_string();
    app.browser_settings.native_chrome_path_input = "/Applications/Chrome.app".to_string();
    app.browser_settings.computer_use_endpoint = "http://127.0.0.1:8787/v1/actions".to_string();
    app.browser_settings.computer_use_api_key_input = "test-token".to_string();
    app.browser_settings.computer_use_timeout_ms_input = "1".to_string();
    app.browser_settings.computer_use_allow_remote_endpoint = true;
    app.browser_settings.computer_use_window_allowlist_input = "Chrome\nVibeWindow".to_string();
    app.browser_settings.computer_use_max_coordinate_x_input = "1920".to_string();
    app.browser_settings.computer_use_max_coordinate_y_input = "1080".to_string();
    keep_element(view(&app));
}

#[test]
fn view_builds_every_supported_selection_and_unknown_fallback() {
    let mut app = test_app();

    for browser_open in ["default", "new_window", "new_tab", "unknown"] {
        app.browser_settings.browser_open = browser_open.to_string();
        keep_element(view(&app));
    }

    for backend in ["agent_browser", "native", "computer_use", "auto", "unknown"] {
        app.browser_settings.backend = backend.to_string();
        keep_element(view(&app));
    }
}

#[test]
fn view_only_adds_native_section_for_native_backend() {
    let mut app = test_app();
    app.browser_settings.backend = "agent_browser".to_string();
    let default_tree = {
        let default = view(&app);
        Tree::new(default.as_widget())
    };
    assert_eq!(default_tree.children.len(), 4);

    app.browser_settings.backend = "native".to_string();
    let native = view(&app);
    let native_tree = Tree::new(native.as_widget());
    assert_eq!(native_tree.children.len(), 5);
    assert_eq!(native_tree.children[3].children.len(), 2);
    assert_eq!(native_tree.children[4].children.len(), 2);
}

#[test]
fn view_keeps_computer_use_section_without_native_section_for_computer_use_backend() {
    let mut app = test_app();
    app.browser_settings.backend = "computer_use".to_string();
    app.browser_settings.computer_use_endpoint = "http://127.0.0.1:8787/v1/actions".to_string();

    let element = view(&app);
    let tree = Tree::new(element.as_widget());

    assert_eq!(tree.children.len(), 4);
    assert_eq!(tree.children[3].children.len(), 2);
}
