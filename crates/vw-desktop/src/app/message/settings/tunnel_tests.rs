// 验证隧道提供商规范化与可选配置区段构建。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
use crate::app::state::TunnelSettingsState;

fn app() -> App {
    App::new().0
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn normalize_provider_trims_case_and_rejects_unknown_values() {
    for provider in ["cloudflare", "tailscale", "ngrok", "custom"] {
        assert_eq!(normalize_provider(&format!("  {}  ", provider.to_ascii_uppercase())), provider);
    }

    assert_eq!(normalize_provider("none"), "none");
    assert_eq!(normalize_provider("unsupported"), "none");
    assert_eq!(normalize_provider("   "), "none");
}

#[test]
fn trim_to_option_discards_blank_values_and_trims_present_values() {
    assert_eq!(trim_to_option(" \n\t "), None);
    assert_eq!(trim_to_option("  node.example.com  "), Some("node.example.com".to_string()));
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn build_tunnel_config_keeps_default_provider_sections_absent() {
    let config = build_tunnel_config(&TunnelSettingsState::default());

    assert_eq!(config.provider, "none");
    assert!(config.cloudflare.is_none());
    assert!(config.tailscale.is_none());
    assert!(config.ngrok.is_none());
    assert!(config.custom.is_none());
}

#[test]
fn build_tunnel_config_treats_whitespace_only_fields_as_absent() {
    let settings = TunnelSettingsState {
        provider: " none ".to_string(),
        cloudflare_token: " \t ".to_string(),
        tailscale_funnel: false,
        tailscale_hostname: " \n ".to_string(),
        ngrok_auth_token: "  ".to_string(),
        ngrok_domain: " \r ".to_string(),
        custom_start_command: " \t ".to_string(),
        custom_health_url: " \n ".to_string(),
        custom_url_pattern: "  ".to_string(),
        save_error: None,
    };

    let config = build_tunnel_config(&settings);

    assert_eq!(config.provider, "none");
    assert!(config.cloudflare.is_none());
    assert!(config.tailscale.is_none());
    assert!(config.ngrok.is_none());
    assert!(config.custom.is_none());
}

#[test]
fn build_tunnel_config_includes_selected_empty_provider_sections() {
    let mut settings = TunnelSettingsState::default();

    settings.provider = "cloudflare".to_string();
    let cloudflare = build_tunnel_config(&settings).cloudflare.unwrap();
    assert!(cloudflare.token.is_empty());

    settings.provider = "tailscale".to_string();
    let tailscale = build_tunnel_config(&settings).tailscale.unwrap();
    assert!(!tailscale.funnel);
    assert!(tailscale.hostname.is_none());

    settings.provider = "ngrok".to_string();
    let ngrok = build_tunnel_config(&settings).ngrok.unwrap();
    assert!(ngrok.auth_token.is_empty());
    assert!(ngrok.domain.is_none());

    settings.provider = "custom".to_string();
    let custom = build_tunnel_config(&settings).custom.unwrap();
    assert!(custom.start_command.is_empty());
    assert!(custom.health_url.is_none());
    assert!(custom.url_pattern.is_none());
}

// 数据边界：覆盖规范化、默认值以及相关状态字段的既定规则。
// 这里固定可观察结果，避免依赖处理过程中的临时实现细节。
#[test]
fn build_tunnel_config_trims_populated_sections_even_when_provider_is_none() {
    let settings = TunnelSettingsState {
        provider: " unexpected ".to_string(),
        cloudflare_token: " cf-token ".to_string(),
        tailscale_funnel: true,
        tailscale_hostname: " node.tailnet.ts.net ".to_string(),
        ngrok_auth_token: " ngrok-token ".to_string(),
        ngrok_domain: " demo.ngrok.app ".to_string(),
        custom_start_command: " bore local {port} ".to_string(),
        custom_health_url: " http://127.0.0.1:4040/health ".to_string(),
        custom_url_pattern: " https://[^ ]+ ".to_string(),
        save_error: Some("old error".to_string()),
    };

    let config = build_tunnel_config(&settings);

    assert_eq!(config.provider, "none");
    assert_eq!(config.cloudflare.unwrap().token, "cf-token");
    let tailscale = config.tailscale.unwrap();
    assert!(tailscale.funnel);
    assert_eq!(tailscale.hostname.as_deref(), Some("node.tailnet.ts.net"));
    let ngrok = config.ngrok.unwrap();
    assert_eq!(ngrok.auth_token, "ngrok-token");
    assert_eq!(ngrok.domain.as_deref(), Some("demo.ngrok.app"));
    let custom = config.custom.unwrap();
    assert!(custom.url.is_none());
    assert!(custom.auth_token.is_none());
    assert_eq!(custom.start_command, "bore local {port}");
    assert_eq!(custom.health_url.as_deref(), Some("http://127.0.0.1:4040/health"));
    assert_eq!(custom.url_pattern.as_deref(), Some("https://[^ ]+"));
}

#[test]
fn update_rejects_non_tunnel_messages_without_changing_state() {
    let mut app = app();
    app.tunnel_settings.provider = "custom".to_string();
    app.tunnel_settings.save_error = Some("keep me".to_string());

    let _ = update(&mut app, SettingsMessage::SystemHelpClose);

    assert_eq!(app.tunnel_settings.provider, "custom");
    assert_eq!(app.tunnel_settings.save_error.as_deref(), Some("keep me"));
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn update_applies_every_tunnel_field_and_clears_save_error() {
    let mut app = app();
    app.tunnel_settings.save_error = Some("old error".to_string());

    let messages = [
        TunnelMessage::ProviderChanged(" NGROK ".to_string()),
        TunnelMessage::CloudflareTokenChanged(" cf-token ".to_string()),
        TunnelMessage::TailscaleFunnelToggled(true),
        TunnelMessage::TailscaleHostnameChanged("node.tailnet.ts.net".to_string()),
        TunnelMessage::NgrokAuthTokenChanged("ngrok-token".to_string()),
        TunnelMessage::NgrokDomainChanged("demo.ngrok.app".to_string()),
        TunnelMessage::CustomStartCommandChanged("bore local {port}".to_string()),
        TunnelMessage::CustomHealthUrlChanged("http://127.0.0.1:4040/health".to_string()),
        TunnelMessage::CustomUrlPatternChanged("https://[^ ]+".to_string()),
    ];

    for message in messages {
        let _ = update(&mut app, SettingsMessage::Tunnel(message));
    }

    assert_eq!(app.tunnel_settings.provider, "ngrok");
    assert_eq!(app.tunnel_settings.cloudflare_token, " cf-token ");
    assert!(app.tunnel_settings.tailscale_funnel);
    assert_eq!(app.tunnel_settings.tailscale_hostname, "node.tailnet.ts.net");
    assert_eq!(app.tunnel_settings.ngrok_auth_token, "ngrok-token");
    assert_eq!(app.tunnel_settings.ngrok_domain, "demo.ngrok.app");
    assert_eq!(app.tunnel_settings.custom_start_command, "bore local {port}");
    assert_eq!(app.tunnel_settings.custom_health_url, "http://127.0.0.1:4040/health");
    assert_eq!(app.tunnel_settings.custom_url_pattern, "https://[^ ]+");
    assert!(app.tunnel_settings.save_error.is_none());
}
