//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::message::settings::util::parse_comma_or_newline_list;
use crate::app::{App, Message, update_http_request_config_async};
use iced::Task;
use config_tools::HttpRequestConfig;

use super::messages::{HttpRequestMessage, SettingsMessage};

fn normalize_domain(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

#[cfg(test)]
#[path = "http_request_tests.rs"]
mod http_request_tests;

fn persist_http_request_settings(app: &mut App) -> Task<Message> {
    let s = &app.http_request_settings;
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

    update_http_request_config_async(move |http_request| {
        http_request.enabled = enabled;
        http_request.allowed_domains = allowed_domains;
        http_request.max_response_size = max_response_size;
        http_request.timeout_secs = timeout_secs;
        http_request.user_agent =
            if user_agent.is_empty() { defaults.user_agent.clone() } else { user_agent };
    })
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::HttpRequest(message) = message else {
        return Task::none();
    };

    if matches!(message, HttpRequestMessage::Refresh) {
        app.http_request_settings.save_error = None;
        return Task::none();
    }

    match message {
        HttpRequestMessage::EnabledToggled(value) => {
            app.http_request_settings.enabled = value;
        }
        HttpRequestMessage::NewAllowedDomainChanged(value) => {
            app.http_request_settings.new_allowed_domain_input = value;
        }
        HttpRequestMessage::AddAllowedDomain => {
            let candidates =
                parse_comma_or_newline_list(&app.http_request_settings.new_allowed_domain_input)
                    .into_iter()
                    .map(|domain| normalize_domain(&domain))
                    .filter(|domain| !domain.is_empty())
                    .collect::<Vec<_>>();

            if candidates.is_empty() {
                app.http_request_settings.save_error = Some("允许域名不能为空".to_string());
                return Task::none();
            }

            let mut inserted = 0usize;
            for domain in candidates {
                if !app.http_request_settings.allowed_domains.iter().any(|item| item == &domain) {
                    app.http_request_settings.allowed_domains.push(domain);
                    inserted += 1;
                }
            }

            if inserted == 0 {
                app.http_request_settings.save_error = Some("允许域名已存在".to_string());
                return Task::none();
            }

            app.http_request_settings.new_allowed_domain_input.clear();
        }
        HttpRequestMessage::RemoveAllowedDomain(index) => {
            if index < app.http_request_settings.allowed_domains.len() {
                app.http_request_settings.allowed_domains.remove(index);
            }
        }
        HttpRequestMessage::MaxResponseSizeChanged(value) => {
            app.http_request_settings.max_response_size = value;
        }
        HttpRequestMessage::TimeoutSecsChanged(value) => {
            app.http_request_settings.timeout_secs = value;
        }
        HttpRequestMessage::UserAgentChanged(value) => {
            app.http_request_settings.user_agent = value;
        }
        HttpRequestMessage::Refresh => unreachable!(),
    }

    app.http_request_settings.save_error = None;
    if app.http_request_settings.user_agent.trim().is_empty() {
        app.http_request_settings.user_agent = HttpRequestConfig::default().user_agent;
    }
    persist_http_request_settings(app)
}
