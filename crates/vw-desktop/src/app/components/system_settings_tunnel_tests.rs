//! 验证隧道提供商稳定值、条件控件、消息映射和视图结构。
//!
//! 提供商值按精确稳定键匹配，未知或近似输入必须回退到未选择状态。

use super::*;
use crate::app::message::settings::{SettingsMessage, TunnelMessage};
use crate::app::{App, Message};
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
}

fn content_tree(tree: &Tree) -> &Tree {
    tree
}

fn provider_controls(tree: &Tree) -> &Tree {
    let content = content_tree(tree);
    assert_eq!(content.children.len(), 3);

    let provider_section = &content.children[2];
    assert_eq!(provider_section.children.len(), 2);

    let panel = &provider_section.children[1];
    panel
}

fn unwrap_tunnel_message(message: Message) -> TunnelMessage {
    match message {
        Message::Settings(SettingsMessage::Tunnel(message)) => message,
        other => panic!("unexpected message: {other:?}"),
    }
}

#[test]
fn provider_options_have_stable_values_and_user_labels() {
    let expected = [
        ("none", "未启用"),
        ("cloudflare", "Cloudflare"),
        ("tailscale", "Tailscale"),
        ("ngrok", "ngrok"),
        ("custom", "自定义"),
    ];

    for (option, (value, label)) in PROVIDER_OPTIONS.iter().zip(expected) {
        assert_eq!(option.value, value);
        assert_eq!(option.label, label);
        assert_eq!(option.to_string(), label);
    }
}

#[test]
fn selected_provider_recognizes_supported_values_and_defaults_to_none() {
    for expected in PROVIDER_OPTIONS {
        assert_eq!(
            selected_provider(expected.value).map(|option| option.value),
            Some(expected.value),
        );
    }

    assert_eq!(selected_provider("").map(|option| option.value), Some("none"));
    assert_eq!(selected_provider("unsupported").map(|option| option.value), Some("none"));
}

#[test]
fn selected_provider_requires_an_exact_stable_provider_value() {
    for provider in [" Cloudflare", "cloudflare ", "CLOUDFLARE", "TAILSCALE", "None"] {
        assert_eq!(
            selected_provider(provider).map(|option| option.value),
            Some("none"),
            "provider={provider}",
        );
    }
}

#[test]
fn row_helpers_build_label_description_and_control_structure() {
    let trees = [
        element_tree(field_row("标签", "说明", text("control"))),
        element_tree(text_row("令牌", "说明", "placeholder", "value", cloudflare_token_changed)),
        element_tree(bool_row("漏斗", "说明", true, "启用", tailscale_funnel_toggled)),
    ];

    for tree in &trees {
        let row = content_tree(tree);
        assert_eq!(row.children.len(), 2);
        assert_eq!(row.children[0].children.len(), 2);
        assert!(row.children[1].children.is_empty());
    }
}

#[test]
fn provider_callback_maps_display_option_to_provider_value() {
    for option in PROVIDER_OPTIONS {
        match unwrap_tunnel_message(provider_changed(option)) {
            TunnelMessage::ProviderChanged(value) => assert_eq!(value, option.value),
            other => panic!("unexpected tunnel message: {other:?}"),
        }
    }
}

#[test]
fn cloudflare_callback_preserves_untrimmed_token_input() {
    let input = "  sample token  ".to_string();

    match unwrap_tunnel_message(cloudflare_token_changed(input.clone())) {
        TunnelMessage::CloudflareTokenChanged(value) => assert_eq!(value, input),
        other => panic!("unexpected tunnel message: {other:?}"),
    }
}

#[test]
fn tailscale_callbacks_preserve_boolean_and_hostname_inputs() {
    for enabled in [false, true] {
        match unwrap_tunnel_message(tailscale_funnel_toggled(enabled)) {
            TunnelMessage::TailscaleFunnelToggled(value) => assert_eq!(value, enabled),
            other => panic!("unexpected tunnel message: {other:?}"),
        }
    }

    let hostname = "节点.tailnet.ts.net".to_string();
    match unwrap_tunnel_message(tailscale_hostname_changed(hostname.clone())) {
        TunnelMessage::TailscaleHostnameChanged(value) => assert_eq!(value, hostname),
        other => panic!("unexpected tunnel message: {other:?}"),
    }
}

#[test]
fn ngrok_callbacks_preserve_empty_auth_token_and_domain_inputs() {
    match unwrap_tunnel_message(ngrok_auth_token_changed(String::new())) {
        TunnelMessage::NgrokAuthTokenChanged(value) => assert!(value.is_empty()),
        other => panic!("unexpected tunnel message: {other:?}"),
    }

    let domain = "sample.ngrok.app".to_string();
    match unwrap_tunnel_message(ngrok_domain_changed(domain.clone())) {
        TunnelMessage::NgrokDomainChanged(value) => assert_eq!(value, domain),
        other => panic!("unexpected tunnel message: {other:?}"),
    }
}

#[test]
fn custom_callbacks_preserve_command_health_url_and_pattern_inputs() {
    let command = "bore local {port} --to {host}".to_string();
    match unwrap_tunnel_message(custom_start_command_changed(command.clone())) {
        TunnelMessage::CustomStartCommandChanged(value) => assert_eq!(value, command),
        other => panic!("unexpected tunnel message: {other:?}"),
    }

    match unwrap_tunnel_message(custom_health_url_changed(String::new())) {
        TunnelMessage::CustomHealthUrlChanged(value) => assert!(value.is_empty()),
        other => panic!("unexpected tunnel message: {other:?}"),
    }

    let pattern = "https://".to_string();
    match unwrap_tunnel_message(custom_url_pattern_changed(pattern.clone())) {
        TunnelMessage::CustomUrlPatternChanged(value) => assert_eq!(value, pattern),
        other => panic!("unexpected tunnel message: {other:?}"),
    }
}

#[test]
fn content_keeps_shared_page_structure_for_disabled_provider() {
    let mut app = test_app();
    app.tunnel_settings.provider = "none".to_string();

    let tree = element_tree(content(&app));
    let content = content_tree(&tree);

    assert_eq!(content.children.len(), 3);
    assert_eq!(content.children[0].children.len(), 2);
    assert_eq!(content.children[1].children.len(), 1);

    let controls = provider_controls(&tree);
    assert_eq!(controls.children.len(), 1);
    assert!(controls.children[0].children.is_empty());
}

#[test]
fn content_builds_each_provider_section_and_unknown_fallback() {
    let mut app = test_app();
    app.tunnel_settings.cloudflare_token = "sample token".to_string();
    app.tunnel_settings.tailscale_funnel = true;
    app.tunnel_settings.tailscale_hostname = "node.tailnet.ts.net".to_string();
    app.tunnel_settings.ngrok_auth_token = "sample auth".to_string();
    app.tunnel_settings.ngrok_domain = "sample.ngrok.app".to_string();
    app.tunnel_settings.custom_start_command = "bore local {port}".to_string();
    app.tunnel_settings.custom_health_url = "http://127.0.0.1:4040".to_string();
    app.tunnel_settings.custom_url_pattern = "https://".to_string();

    for (provider, expected_controls, first_control_is_leaf) in [
        ("none", 1, true),
        ("cloudflare", 1, false),
        ("tailscale", 5, false),
        ("ngrok", 3, false),
        ("custom", 5, false),
        ("unsupported", 1, true),
    ] {
        app.tunnel_settings.provider = provider.to_string();
        let tree = element_tree(content(&app));
        let controls = provider_controls(&tree);

        assert_eq!(controls.children.len(), expected_controls, "provider={provider}");
        assert_eq!(
            controls.children[0].children.is_empty(),
            first_control_is_leaf,
            "provider={provider}",
        );
    }
}
