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
    let app = test_app();
    let trees = [
        element_tree(field_row("标签", "说明", text("control"))),
        element_tree(text_row(
            &app,
            "令牌",
            "说明",
            "placeholder",
            "value",
            cloudflare_token_changed,
        )),
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

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn help_rollout_17_renders_contextual_help() {
    use super::super::{
        system_settings::SystemTab,
        system_settings_network as network,
        system_settings_proxy::system_settings_proxy_tests::rollout::{
            MODES, Scene, isolated, renderer,
        },
    };

    if !isolated(concat!(module_path!(), "::help_rollout_17_renders_contextual_help")) {
        return;
    }
    let mut renderer = renderer().await;
    let mut app = test_app();
    app.system_settings_tab = SystemTab::Tunnel;
    app.tunnel_settings.save_error = Some("Fixture tunnel connection failed".into());
    app.tunnel_settings.custom_start_command = "fixture-tunnel {port}".into();

    for (locale, theme) in MODES {
        app.desktop_locale = locale;
        app.app_theme = theme;
        let t = |key| tr(locale, key);
        for (provider, fields) in [
            (
                "custom",
                &[
                    ("启动命令", "支持在命令中使用 {host} 与 {port} 占位符。"),
                    ("健康检查 URL", "可选，用于确认隧道服务是否已准备就绪。"),
                    ("URL 模式", "可选，可填前缀或正则片段用于提取公网地址。"),
                    (
                        "自定义隧道",
                        "当前运行时使用命令型自定义隧道，因此这里配置 start_command / health_url / url_pattern。",
                    ),
                ][..],
            ),
            (
                "cloudflare",
                &[
                    ("Cloudflare Tunnel", "使用 cloudflared token 将本地网关安全暴露到公网。"),
                    ("令牌", "Zero Trust 侧下发的 tunnel token。"),
                ][..],
            ),
            ("tailscale", &[("主机名", "留空时将使用当前 Tailscale 节点名。")][..]),
            (
                "ngrok",
                &[
                    ("ngrok", "配置 ngrok token，并可选绑定自定义 domain。"),
                    ("认证令牌", "ngrok 控制台生成的 auth token。"),
                    ("域名", "可选，绑定预留的自定义域名。"),
                ][..],
            ),
            ("none", &[("未启用隧道", "保持 provider=none 时，网关不会自动创建公网隧道。")][..]),
        ] {
            app.tunnel_settings.provider = provider.into();
            let mut scene = Scene::new(&app, &mut renderer, provider, network::view(&app));
            scene.label("Fixture tunnel connection failed");
            for (title, description) in fields {
                scene.help(t(title), t(description));
            }
            if provider == "custom" {
                scene.help(t("接入方式"), t("选择隧道 provider，并根据 provider 展开对应参数。"));
                scene.label(t("选择自动暴露网关到公网或私有网络的方式。"));
                assert!(matches!(
                    scene.edit("fixture-tunnel {port}").as_slice(),
                    [Message::Settings(SettingsMessage::Tunnel(TunnelMessage::CustomStartCommandChanged(value)))]
                        if value == "fixture-tunnel {port"
                ));
            }
            if provider == "tailscale" {
                scene.label(t("serve 仅 tailnet 内可访问，funnel 才会暴露到公网。访问地址以启动日志为准，通常不要使用本地监听端口。"));
                let checkbox = scene.label(t("启用 funnel（关闭时使用 serve）")).center();
                assert!(matches!(
                    scene.click(checkbox).as_slice(),
                    [Message::Settings(SettingsMessage::Tunnel(
                        TunnelMessage::TailscaleFunnelToggled(true)
                    ))]
                ));
            }
            if provider == "none" {
                scene.label(t("当前仅监听本地网关地址，不会主动创建公网或 tailnet 暴露。"));
            }
        }
    }
}
