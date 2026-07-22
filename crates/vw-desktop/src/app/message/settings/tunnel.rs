//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::state::TunnelSettingsState;
use crate::app::{App, Message, update_tunnel_config_async};
use iced::Task;
use vw_config_types::gateway::{
    CloudflareTunnelConfig, CustomTunnelConfig, NgrokTunnelConfig, TailscaleTunnelConfig,
    TunnelConfig,
};

use super::messages::{SettingsMessage, TunnelMessage};

fn normalize_provider(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "cloudflare" => "cloudflare".to_string(),
        "tailscale" => "tailscale".to_string(),
        "ngrok" => "ngrok".to_string(),
        "custom" => "custom".to_string(),
        _ => "none".to_string(),
    }
}

fn trim_to_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn build_tunnel_config(s: &TunnelSettingsState) -> TunnelConfig {
    let tailscale_funnel = s.tailscale_funnel;
    let provider = normalize_provider(&s.provider);
    let cloudflare_token = s.cloudflare_token.trim().to_string();
    let tailscale_hostname = trim_to_option(&s.tailscale_hostname);
    let ngrok_auth_token = s.ngrok_auth_token.trim().to_string();
    let ngrok_domain = trim_to_option(&s.ngrok_domain);
    let custom_start_command = s.custom_start_command.trim().to_string();
    let custom_health_url = trim_to_option(&s.custom_health_url);
    let custom_url_pattern = trim_to_option(&s.custom_url_pattern);

    let include_cloudflare = provider == "cloudflare" || !cloudflare_token.is_empty();
    let include_tailscale =
        provider == "tailscale" || s.tailscale_funnel || tailscale_hostname.is_some();
    let include_ngrok =
        provider == "ngrok" || !ngrok_auth_token.is_empty() || ngrok_domain.is_some();
    let include_custom = provider == "custom"
        || !custom_start_command.is_empty()
        || custom_health_url.is_some()
        || custom_url_pattern.is_some();

    TunnelConfig {
        provider,
        cloudflare: include_cloudflare
            .then_some(CloudflareTunnelConfig { token: cloudflare_token }),
        tailscale: include_tailscale.then_some(TailscaleTunnelConfig {
            funnel: tailscale_funnel,
            hostname: tailscale_hostname,
        }),
        ngrok: include_ngrok
            .then_some(NgrokTunnelConfig { auth_token: ngrok_auth_token, domain: ngrok_domain }),
        custom: include_custom.then_some(CustomTunnelConfig {
            url: None,
            auth_token: None,
            start_command: custom_start_command,
            health_url: custom_health_url,
            url_pattern: custom_url_pattern,
        }),
    }
}

fn persist_tunnel_settings(app: &mut App) -> Task<Message> {
    let config = build_tunnel_config(&app.tunnel_settings);
    update_tunnel_config_async(move |tunnel| *tunnel = config)
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::Tunnel(message) = message else {
        return Task::none();
    };

    match message {
        TunnelMessage::ProviderChanged(value) => {
            app.tunnel_settings.provider = normalize_provider(&value);
        }
        TunnelMessage::CloudflareTokenChanged(value) => {
            app.tunnel_settings.cloudflare_token = value
        }
        TunnelMessage::TailscaleFunnelToggled(value) => {
            app.tunnel_settings.tailscale_funnel = value
        }
        TunnelMessage::TailscaleHostnameChanged(value) => {
            app.tunnel_settings.tailscale_hostname = value;
        }
        TunnelMessage::NgrokAuthTokenChanged(value) => app.tunnel_settings.ngrok_auth_token = value,
        TunnelMessage::NgrokDomainChanged(value) => app.tunnel_settings.ngrok_domain = value,
        TunnelMessage::CustomStartCommandChanged(value) => {
            app.tunnel_settings.custom_start_command = value;
        }
        TunnelMessage::CustomHealthUrlChanged(value) => {
            app.tunnel_settings.custom_health_url = value;
        }
        TunnelMessage::CustomUrlPatternChanged(value) => {
            app.tunnel_settings.custom_url_pattern = value;
        }
    }

    app.tunnel_settings.save_error = None;
    persist_tunnel_settings(app)
}
#[cfg(test)]
#[path = "tunnel_tests.rs"]
mod tunnel_tests;
