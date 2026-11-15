//! 处理系统设置页面中对应功能区的消息、校验和配置持久化。

use crate::app::{App, Message, update_web_search_config_async};
use iced::Task;
use config_tools::WebSearchConfig;

use super::messages::{SettingsMessage, WebSearchMessage};

fn normalize_provider(raw: &str) -> String {
    match raw.trim().to_ascii_lowercase().as_str() {
        "ddg" | "duckduckgo" => "duckduckgo".to_string(),
        "brave" => "brave".to_string(),
        "serper" => "serper".to_string(),
        "google" => "google".to_string(),
        "bing" => "bing".to_string(),
        _ => "duckduckgo".to_string(),
    }
}

fn trim_to_option(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

fn parse_max_results(input: &str) -> Result<usize, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(WebSearchConfig::default().max_results);
    }

    let value = trimmed.parse::<usize>().map_err(|_| "max_results 必须是整数".to_string())?;
    if value == 0 {
        return Err("max_results 必须大于 0".to_string());
    }
    Ok(value.clamp(1, 10))
}

fn parse_timeout_secs(input: &str) -> Result<u64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(WebSearchConfig::default().timeout_secs);
    }

    let value = trimmed.parse::<u64>().map_err(|_| "timeout_secs 必须是整数秒".to_string())?;
    if value == 0 {
        return Err("timeout_secs 必须大于 0".to_string());
    }
    Ok(value)
}

fn persist_web_search_settings(app: &mut App) -> Result<Task<Message>, String> {
    let s = &app.web_search_settings;
    let enabled = s.enabled;
    let provider = normalize_provider(&s.provider);
    let api_key = trim_to_option(&s.api_key_input);
    let api_url = trim_to_option(&s.api_url_input);
    let brave_api_key = trim_to_option(&s.brave_api_key_input);
    let max_results = parse_max_results(&s.max_results_input)?;
    let timeout_secs = parse_timeout_secs(&s.timeout_secs_input)?;
    let user_agent = {
        let value = s.user_agent.trim();
        if value.is_empty() { WebSearchConfig::default().user_agent } else { value.to_string() }
    };

    Ok(update_web_search_config_async(move |web_search| {
        web_search.enabled = enabled;
        web_search.provider = provider;
        web_search.api_key = api_key;
        web_search.api_url = api_url;
        web_search.brave_api_key = brave_api_key;
        web_search.max_results = max_results;
        web_search.timeout_secs = timeout_secs;
        web_search.user_agent = user_agent;
    }))
}

/// 处理 `update` 对应的用户输入、异步结果或状态转换。
///
/// 参数来自已匹配的消息载荷或当前设置状态，函数只在当前消息边界内产生状态变更。
/// 返回的 `Task` 用于继续执行异步保存、加载或通知清理；没有后续动作时返回空任务。
pub fn update(app: &mut App, message: SettingsMessage) -> Task<Message> {
    let SettingsMessage::WebSearch(message) = message else {
        return Task::none();
    };

    match message {
        WebSearchMessage::Refresh => {
            app.web_search_settings.save_error = None;
            return Task::none();
        }
        WebSearchMessage::EnabledToggled(value) => app.web_search_settings.enabled = value,
        WebSearchMessage::ProviderChanged(value) => {
            app.web_search_settings.provider = normalize_provider(&value)
        }
        WebSearchMessage::ApiKeyChanged(value) => app.web_search_settings.api_key_input = value,
        WebSearchMessage::ApiUrlChanged(value) => app.web_search_settings.api_url_input = value,
        WebSearchMessage::BraveApiKeyChanged(value) => {
            app.web_search_settings.brave_api_key_input = value
        }
        WebSearchMessage::MaxResultsChanged(value) => {
            app.web_search_settings.max_results_input = value;
        }
        WebSearchMessage::TimeoutSecsChanged(value) => {
            app.web_search_settings.timeout_secs_input = value;
        }
        WebSearchMessage::UserAgentChanged(value) => app.web_search_settings.user_agent = value,
        WebSearchMessage::HelpOpen => {
            app.web_search_settings.show_help_modal = true;
            return Task::none();
        }
        WebSearchMessage::HelpClose => {
            app.web_search_settings.show_help_modal = false;
            return Task::none();
        }
    }

    match persist_web_search_settings(app) {
        Ok(task) => {
            app.web_search_settings.save_error = None;
            task
        }
        Err(err) => {
            app.web_search_settings.save_error = Some(err);
            Task::none()
        }
    }
}
#[cfg(test)]
#[path = "web_search_tests.rs"]
mod web_search_tests;
