use super::*;
use crate::app::App;
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;
use vw_config_types::proxy::ProxyScope;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

#[test]
fn proxy_scope_label_returns_stable_user_labels() {
    assert_eq!(proxy_scope_label(ProxyScope::Environment), "系统环境");
    assert_eq!(proxy_scope_label(ProxyScope::Vibewindow), "VibeWindow");
    assert_eq!(proxy_scope_label(ProxyScope::Services), "指定服务");
}

#[test]
fn proxy_scope_options_match_scope_labels() {
    assert_eq!(PROXY_SCOPE_OPTIONS, ["系统环境", "VibeWindow", "指定服务"]);

    for scope in [ProxyScope::Environment, ProxyScope::Vibewindow, ProxyScope::Services] {
        assert!(PROXY_SCOPE_OPTIONS.contains(&proxy_scope_label(scope)));
    }
}

#[test]
fn field_row_accepts_fill_width_control() {
    let element: Element<'_, Message> =
        field_row("HTTP 代理", "HTTP 请求代理地址。", text("value"));
    drop(element);
}

#[test]
fn view_builds_default_and_populated_proxy_settings() {
    let app = test_app();
    keep_element(view(&app));

    let mut populated = test_app();
    populated.proxy_settings.enabled = true;
    populated.proxy_settings.scope = ProxyScope::Environment;
    populated.proxy_settings.http_proxy = "http://127.0.0.1:7890".to_string();
    populated.proxy_settings.https_proxy = "http://127.0.0.1:7891".to_string();
    populated.proxy_settings.all_proxy = "socks5://127.0.0.1:7892".to_string();
    populated.proxy_settings.no_proxy_input = "localhost, 127.0.0.1, *.internal".to_string();
    populated.proxy_settings.services_input = "provider.openai, tool.http_request".to_string();
    keep_element(view(&populated));

    populated.proxy_settings.scope = ProxyScope::Services;
    keep_element(view(&populated));
}

#[test]
fn overlays_return_base_dialog_or_help_modal() {
    let app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    let mut with_help = test_app();
    with_help.proxy_settings.show_help_modal = true;
    keep_element(view_overlays(&with_help, text("dialog").into()));
}

#[test]
fn view_groups_proxy_fields_into_expected_sections() {
    let tree = element_tree(view(&test_app()));

    assert_eq!(tree.children.len(), 7);
    assert_eq!(tree.children[2].children.len(), 3);
    assert_eq!(tree.children[4].children.len(), 5);
    assert_eq!(tree.children[6].children.len(), 3);
}
