//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。
use crate::Message as SettingsMessage;
use crate::app_init::state::HttpRequestSettingsState;
use super::parse_comma_or_newline_list;


use config_tools::HttpRequestConfig;

use crate::messages::{HttpRequestMessage};

fn normalize_domain(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

#[cfg(test)]
#[path = "http_request_tests.rs"]
mod http_request_tests;

fn persist_http_request_settings(app: &mut HttpRequestSettingsState) -> Effect {
    let s = &app;
    let defaults = HttpRequestConfig::default();
    let enabled = s.enabled;
    let allowed_domains = s
        .allowed_domains
        .iter()
        .map(|domain| normalize_domain(domain))
        .filter(|domain| !domain.is_empty())
        .collect::<Vec<_>>();
    let max_response_size = s.max_response_size as usize;
    let timeout_secs = s.timeout_secs as u64;
    let user_agent = s.user_agent.trim().to_string();

    {
        let mut http_request = config_tools::HttpRequestConfig::default();
        http_request.enabled = enabled;
        http_request.allowed_domains = allowed_domains;
        http_request.max_response_size = max_response_size;
        http_request.timeout_secs = timeout_secs;
        http_request.user_agent =
            if user_agent.is_empty() { defaults.user_agent.clone() } else { user_agent };

        Effect::Persist(http_request)
    }
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回领域效果，由宿主执行持久化或平台动作。
pub fn update(app: &mut HttpRequestSettingsState, message: SettingsMessage) -> Effect {
    let SettingsMessage::HttpRequest(message) = message else {
        return Effect::None;
    };

    if matches!(message, HttpRequestMessage::Refresh) {
        app.save_error = None;
        return Effect::None;
    }

    match message {
        HttpRequestMessage::EnabledToggled(value) => {
            app.enabled = value;
        }
        HttpRequestMessage::NewAllowedDomainChanged(value) => {
            app.new_allowed_domain_input = value;
        }
        HttpRequestMessage::AddAllowedDomain => {
            let candidates =
                parse_comma_or_newline_list(&app.new_allowed_domain_input)
                    .into_iter()
                    .map(|domain| normalize_domain(&domain))
                    .filter(|domain| !domain.is_empty())
                    .collect::<Vec<_>>();

            if candidates.is_empty() {
                app.save_error = Some("允许域名不能为空".to_string());
                return Effect::None;
            }

            let mut inserted = 0usize;
            for domain in candidates {
                if !app.allowed_domains.iter().any(|item| item == &domain) {
                    app.allowed_domains.push(domain);
                    inserted += 1;
                }
            }

            if inserted == 0 {
                app.save_error = Some("允许域名已存在".to_string());
                return Effect::None;
            }

            app.new_allowed_domain_input.clear();
        }
        HttpRequestMessage::RemoveAllowedDomain(index) => {
            if index < app.allowed_domains.len() {
                app.allowed_domains.remove(index);
            }
        }
        HttpRequestMessage::MaxResponseSizeChanged(value) => {
            app.max_response_size = value;
        }
        HttpRequestMessage::TimeoutSecsChanged(value) => {
            app.timeout_secs = value;
        }
        HttpRequestMessage::UserAgentChanged(value) => {
            app.user_agent = value;
        }
        HttpRequestMessage::Refresh => unreachable!(),
    }

    app.save_error = None;
    if app.user_agent.trim().is_empty() {
        app.user_agent = HttpRequestConfig::default().user_agent;
    }
    persist_http_request_settings(app)
}

pub enum Effect { None, Persist(config_tools::HttpRequestConfig) }

/// Apply only fields owned by this settings page.
pub fn apply_config(target: &mut config_tools::HttpRequestConfig, value: config_tools::HttpRequestConfig) {
        target.enabled = value.enabled;
        target.allowed_domains = value.allowed_domains;
        target.max_response_size = value.max_response_size;
        target.timeout_secs = value.timeout_secs;
        target.user_agent = value.user_agent;
}
